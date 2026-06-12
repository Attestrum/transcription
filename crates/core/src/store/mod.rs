//! Transcript + settings persistence.
//!
//! Implements `docs/diagrams/store/transcript-schema.md`: one JSON document
//! per transcript under `transcripts/`, recordings archived as a sibling WAV
//! under `audio/`, verified models under `models/`, and one `settings.json`.
//! Unknown/missing optional fields are tolerated on decode so older files
//! keep loading as the schema grows.

mod transcript;

pub use transcript::{Settings, Transcript, TranscriptMeta, TranscriptSegment};

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::CoreError;

/// The on-disk store rooted at the app-data dir (or the user's override).
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Open (creating the layout if needed) a store at `root`.
    pub fn open(root: &Path) -> Result<Self, CoreError> {
        for sub in ["transcripts", "audio", "models"] {
            fs::create_dir_all(root.join(sub))?;
        }
        Ok(Self {
            root: root.to_owned(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn transcripts_dir(&self) -> PathBuf {
        self.root.join("transcripts")
    }

    pub fn audio_dir(&self) -> PathBuf {
        self.root.join("audio")
    }

    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    fn transcript_path(&self, id: &str) -> PathBuf {
        self.transcripts_dir().join(format!("{id}.json"))
    }

    /// Load every transcript's summary, newest `updatedAt` first. Files that
    /// fail to parse are skipped (one corrupt document must not hide the
    /// rest of the library).
    pub fn list(&self) -> Result<Vec<TranscriptMeta>, CoreError> {
        let mut metas: Vec<TranscriptMeta> = Vec::new();
        for entry in fs::read_dir(self.transcripts_dir())? {
            let path = entry?.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            if let Ok(t) = serde_json::from_str::<Transcript>(&text) {
                metas.push(t.meta());
            }
        }
        metas.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(metas)
    }

    pub fn load(&self, id: &str) -> Result<Transcript, CoreError> {
        let path = self.transcript_path(id);
        let text =
            fs::read_to_string(&path).map_err(|_| CoreError::TranscriptNotFound(id.to_owned()))?;
        serde_json::from_str(&text)
            .map_err(|e| CoreError::Store(format!("parse {}: {e}", path.display())))
    }

    /// Atomic save: write to a temp file in the same dir, then rename.
    pub fn save(&self, transcript: &Transcript) -> Result<(), CoreError> {
        let path = self.transcript_path(&transcript.id);
        let json = serde_json::to_string_pretty(transcript)
            .map_err(|e| CoreError::Store(format!("serialize transcript: {e}")))?;
        write_atomic(&path, json.as_bytes())
    }

    /// Remove the transcript document and, if present, its sibling WAV.
    pub fn delete(&self, id: &str) -> Result<(), CoreError> {
        let transcript = self.load(id)?;
        fs::remove_file(self.transcript_path(id))?;
        if let Some(rel) = &transcript.audio_relative_path {
            let wav = self.audio_dir().join(rel);
            if wav.is_file() {
                fs::remove_file(wav)?;
            }
        }
        Ok(())
    }

    /// Load settings, falling back to defaults when the file is missing or
    /// unreadable (a corrupt settings file must not brick startup).
    pub fn load_settings(&self) -> Settings {
        fs::read_to_string(self.root.join("settings.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), CoreError> {
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| CoreError::Store(format!("serialize settings: {e}")))?;
        write_atomic(&self.root.join("settings.json"), json.as_bytes())
    }
}

/// Write via temp-file-then-rename so readers never observe a half-written
/// document.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> (PathBuf, Store) {
        let root =
            std::env::temp_dir().join(format!("attestrum-store-{tag}-{}", uuid::Uuid::new_v4()));
        let store = Store::open(&root).unwrap();
        (root, store)
    }

    fn sample_transcript() -> Transcript {
        Transcript::new(
            "Test Recording".into(),
            None,
            12.5,
            "tiny".into(),
            "en".into(),
            vec![
                TranscriptSegment {
                    id: 0,
                    start: 0.0,
                    end: 4.2,
                    text: "hello world".into(),
                    original_text: None,
                },
                TranscriptSegment {
                    id: 1,
                    start: 4.2,
                    end: 8.0,
                    text: "second segment".into(),
                    original_text: None,
                },
            ],
        )
    }

    #[test]
    fn save_load_roundtrip_preserves_document() {
        let (root, store) = temp_store("roundtrip");
        let t = sample_transcript();
        store.save(&t).unwrap();
        let loaded = store.load(&t.id).unwrap();
        assert_eq!(loaded, t);
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn list_is_sorted_newest_first_and_skips_corrupt_files() {
        let (root, store) = temp_store("list");
        let mut a = sample_transcript();
        a.title = "older".into();
        let mut b = sample_transcript();
        b.title = "newer".into();
        b.updated_at += time::Duration::seconds(60);
        store.save(&a).unwrap();
        store.save(&b).unwrap();
        std::fs::write(store.transcripts_dir().join("garbage.json"), b"{not json").unwrap();

        let metas = store.list().unwrap();
        assert_eq!(metas.len(), 2);
        assert_eq!(metas[0].title, "newer");
        assert_eq!(metas[1].title, "older");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn delete_removes_document_and_sibling_wav() {
        let (root, store) = temp_store("delete");
        let mut t = sample_transcript();
        let wav_rel = format!("{}.wav", t.id);
        std::fs::write(store.audio_dir().join(&wav_rel), b"RIFF").unwrap();
        t.audio_relative_path = Some(wav_rel.clone());
        store.save(&t).unwrap();

        store.delete(&t.id).unwrap();
        assert!(!store
            .transcripts_dir()
            .join(format!("{}.json", t.id))
            .exists());
        assert!(!store.audio_dir().join(&wav_rel).exists());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn missing_transcript_is_typed() {
        let (root, store) = temp_store("missing");
        let err = store.load("no-such-id").unwrap_err();
        assert!(
            matches!(err, CoreError::TranscriptNotFound(_)),
            "got {err:?}"
        );
        let err = store.delete("no-such-id").unwrap_err();
        assert!(
            matches!(err, CoreError::TranscriptNotFound(_)),
            "got {err:?}"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn settings_default_when_missing_and_roundtrip() {
        let (root, store) = temp_store("settings");
        let defaults = store.load_settings();
        assert_eq!(defaults, Settings::default());

        let mut s = defaults;
        s.language = "de".into();
        s.fx_enabled = false;
        store.save_settings(&s).unwrap();
        assert_eq!(store.load_settings(), s);

        // Corrupt settings fall back to defaults instead of erroring.
        std::fs::write(root.join("settings.json"), b"{broken").unwrap();
        assert_eq!(store.load_settings(), Settings::default());
        std::fs::remove_dir_all(root).ok();
    }

    /// The forward-compat contract: documents with unknown fields and absent
    /// optional fields decode cleanly.
    #[test]
    fn tolerant_decode_of_older_and_newer_documents() {
        let (root, store) = temp_store("tolerant");
        let json = r#"{
            "schemaVersion": 1,
            "id": "0d4a11e0-0000-4000-8000-000000000001",
            "title": "from the future",
            "createdAt": "2026-06-12T10:00:00Z",
            "updatedAt": "2026-06-12T10:00:00Z",
            "duration": 1.0,
            "modelId": "tiny",
            "language": "en",
            "segments": [
                {"id": 0, "start": 0.0, "end": 1.0, "text": "hi", "someNewField": 42}
            ],
            "fieldFromVersion9": {"nested": true}
        }"#;
        std::fs::write(
            store
                .transcripts_dir()
                .join("0d4a11e0-0000-4000-8000-000000000001.json"),
            json,
        )
        .unwrap();
        let t = store.load("0d4a11e0-0000-4000-8000-000000000001").unwrap();
        assert_eq!(t.title, "from the future");
        assert_eq!(t.source_filename, None);
        assert_eq!(t.segments[0].original_text, None);
        std::fs::remove_dir_all(root).ok();
    }
}
