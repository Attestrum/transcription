//! The transcript and settings documents of
//! `docs/diagrams/store/transcript-schema.md`. Field names serialize in
//! camelCase to match the diagram (and the Swift app's proven layout);
//! optional fields default on decode so older documents keep loading.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::models;

/// Current transcript document version.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// Import filename; `None` for mic recordings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_filename: Option<String>,
    /// Total audio length in seconds.
    pub duration: f64,
    /// Catalog id of the model that produced the transcript.
    pub model_id: String,
    /// ISO 639-1 code, or what whisper auto-detected.
    pub language: String,
    /// Wall-clock inference time in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_duration_secs: Option<f64>,
    /// Sibling WAV under `audio/` — present for recordings only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_relative_path: Option<String>,
    pub segments: Vec<TranscriptSegment>,
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    /// Ordinal within the transcript.
    pub id: u32,
    /// Seconds from the start of the audio.
    pub start: f64,
    pub end: f64,
    /// Current (possibly user-edited) text.
    pub text: String,
    /// Engine output preserved on first edit; never overwritten after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_text: Option<String>,
}

impl Transcript {
    /// A new transcript stamped now, with a fresh UUID.
    pub fn new(
        title: String,
        source_filename: Option<String>,
        duration: f64,
        model_id: String,
        language: String,
        segments: Vec<TranscriptSegment>,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            schema_version: SCHEMA_VERSION,
            id: uuid::Uuid::new_v4().to_string(),
            title,
            created_at: now,
            updated_at: now,
            source_filename,
            duration,
            model_id,
            language,
            transcription_duration_secs: None,
            audio_relative_path: None,
            segments,
        }
    }

    /// Apply a user edit to one segment, preserving the engine's words: the
    /// first edit copies `text` into `originalText`, later edits only touch
    /// `text`. Returns false when the segment id doesn't exist.
    pub fn edit_segment(&mut self, segment_id: u32, new_text: &str) -> bool {
        let Some(seg) = self.segments.iter_mut().find(|s| s.id == segment_id) else {
            return false;
        };
        if seg.text == new_text {
            return true;
        }
        if seg.original_text.is_none() {
            seg.original_text = Some(seg.text.clone());
        }
        seg.text = new_text.to_owned();
        self.updated_at = OffsetDateTime::now_utc();
        true
    }

    /// The library-row summary.
    pub fn meta(&self) -> TranscriptMeta {
        TranscriptMeta {
            id: self.id.clone(),
            title: self.title.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            duration: self.duration,
            model_id: self.model_id.clone(),
            language: self.language.clone(),
            has_audio: self.audio_relative_path.is_some(),
        }
    }
}

/// Library-listing summary of a transcript (no segments).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptMeta {
    pub id: String,
    pub title: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub duration: f64,
    pub model_id: String,
    pub language: String,
    pub has_audio: bool,
}

/// `settings.json` — every field defaults so a missing or partial file
/// yields a working configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Catalog id picked for new transcriptions.
    pub default_model_id: String,
    /// "auto" lets whisper detect.
    pub language: String,
    /// cpal device name; `None` = system default.
    pub input_device_id: Option<String>,
    /// Storage-root override; `None` = the platform app-data dir.
    pub storage_dir: Option<String>,
    /// Scan-lines + glow toggle (doubles as reduced-motion).
    pub fx_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_model_id: models::default_model_id().to_owned(),
            language: "auto".into(),
            input_device_id: None,
            storage_dir: None,
            fx_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_segment() -> Transcript {
        Transcript::new(
            "t".into(),
            None,
            1.0,
            "tiny".into(),
            "en".into(),
            vec![TranscriptSegment {
                id: 0,
                start: 0.0,
                end: 1.0,
                text: "their going".into(),
                original_text: None,
            }],
        )
    }

    #[test]
    fn first_edit_preserves_original_later_edits_do_not_overwrite_it() {
        let mut t = one_segment();
        assert!(t.edit_segment(0, "they're going"));
        assert_eq!(t.segments[0].original_text.as_deref(), Some("their going"));
        assert!(t.edit_segment(0, "they are going"));
        assert_eq!(t.segments[0].original_text.as_deref(), Some("their going"));
        assert_eq!(t.segments[0].text, "they are going");
    }

    #[test]
    fn editing_unknown_segment_reports_false() {
        let mut t = one_segment();
        assert!(!t.edit_segment(99, "nope"));
    }

    #[test]
    fn identical_edit_is_a_no_op() {
        let mut t = one_segment();
        let before = t.updated_at;
        assert!(t.edit_segment(0, "their going"));
        assert_eq!(t.segments[0].original_text, None);
        assert_eq!(t.updated_at, before);
    }

    #[test]
    fn settings_default_model_is_in_catalog() {
        let s = Settings::default();
        assert!(crate::models::spec(&s.default_model_id).is_some());
    }
}
