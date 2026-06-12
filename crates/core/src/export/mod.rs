//! Transcript exporters: TXT, SRT, VTT, JSON.
//!
//! Formats mirror the original Swift app's proven output byte-for-byte
//! (minus speaker support, cut from v1): SRT uses comma millisecond
//! separators, VTT uses dots, JSON follows the de-facto OpenAI-whisper shape
//! with `speaker` pre-declared as null so adding diarization later is
//! non-breaking. Output is deterministic — two exports of the same
//! transcript are byte-identical (golden-file tested).

use serde::Serialize;
use time::format_description::well_known::Rfc3339;

use crate::error::CoreError;
use crate::store::Transcript;

/// Export formats available in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Txt,
    Srt,
    Vtt,
    Json,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::Json => "json",
        }
    }
}

/// Render `transcript` in the given format.
pub fn export(transcript: &Transcript, format: ExportFormat) -> Result<String, CoreError> {
    match format {
        ExportFormat::Txt => Ok(txt(transcript)),
        ExportFormat::Srt => Ok(srt(transcript)),
        ExportFormat::Vtt => Ok(vtt(transcript)),
        ExportFormat::Json => json(transcript),
    }
}

fn txt(t: &Transcript) -> String {
    let mut out = t
        .segments
        .iter()
        .map(|s| s.text.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    out.push('\n');
    out
}

fn srt(t: &Transcript) -> String {
    let mut out = String::new();
    for (i, seg) in t.segments.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            timestamp(seg.start, ','),
            timestamp(seg.end, ','),
            seg.text.trim()
        ));
    }
    out
}

fn vtt(t: &Transcript) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for seg in &t.segments {
        out.push_str(&format!(
            "{} --> {}\n{}\n\n",
            timestamp(seg.start, '.'),
            timestamp(seg.end, '.'),
            seg.text.trim()
        ));
    }
    out
}

/// `HH:MM:SS<sep>mmm` — SRT separates milliseconds with a comma, VTT with a
/// dot; everything else is identical.
fn timestamp(secs: f64, sep: char) -> String {
    let total_ms = (secs * 1000.0).round() as i64;
    let h = total_ms / 3_600_000;
    let m = (total_ms / 60_000) % 60;
    let s = (total_ms / 1000) % 60;
    let ms = total_ms % 1000;
    format!("{h:02}:{m:02}:{s:02}{sep}{ms:03}")
}

/// The de-facto OpenAI-whisper JSON shape. Fields are declared in
/// alphabetical order, which is what serde emits — keeping output stable
/// and diff-friendly.
#[derive(Serialize)]
struct JsonExport<'a> {
    created_at: String,
    duration: f64,
    language: &'a str,
    model: &'a str,
    segments: Vec<JsonSegment<'a>>,
    source: &'static str,
    text: String,
}

#[derive(Serialize)]
struct JsonSegment<'a> {
    end: f64,
    id: u32,
    /// Always null in v1 — pre-declared so adding diarization later does not
    /// break downstream parsers.
    speaker: Option<&'a str>,
    start: f64,
    text: &'a str,
}

fn json(t: &Transcript) -> Result<String, CoreError> {
    let doc = JsonExport {
        created_at: t
            .created_at
            .format(&Rfc3339)
            .map_err(|e| CoreError::Store(format!("format createdAt: {e}")))?,
        duration: t.duration,
        language: &t.language,
        model: &t.model_id,
        segments: t
            .segments
            .iter()
            .map(|s| JsonSegment {
                end: s.end,
                id: s.id,
                speaker: None,
                start: s.start,
                text: s.text.trim(),
            })
            .collect(),
        source: if t.audio_relative_path.is_some() {
            "mic"
        } else {
            "file"
        },
        text: t
            .segments
            .iter()
            .map(|s| s.text.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    };
    serde_json::to_string_pretty(&doc).map_err(|e| CoreError::Store(format!("export json: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::TranscriptSegment;
    use time::macros::datetime;

    /// A fixed transcript so exports are byte-stable across runs.
    fn golden_transcript() -> Transcript {
        let mut t = Transcript::new(
            "JFK Inaugural (excerpt)".into(),
            Some("jfk.wav".into()),
            11.0,
            "tiny".into(),
            "en".into(),
            vec![
                TranscriptSegment {
                    id: 0,
                    start: 0.0,
                    end: 5.46,
                    text: " And so my fellow Americans,".into(),
                    original_text: None,
                },
                TranscriptSegment {
                    id: 1,
                    start: 5.46,
                    end: 10.92,
                    text: "ask not what your country can do for you.".into(),
                    original_text: Some("ask not what your country can do for you".into()),
                },
            ],
        );
        t.id = "0d4a11e0-0000-4000-8000-00000000abcd".into();
        t.created_at = datetime!(2026-06-12 10:00:00 UTC);
        t.updated_at = t.created_at;
        t
    }

    fn golden_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/export")
            .join(name)
    }

    /// Byte-identity against the committed golden files. To regenerate after
    /// an intentional format change: delete the file and re-run with
    /// UPDATE_GOLDEN=1.
    fn assert_golden(name: &str, rendered: &str) {
        let path = golden_path(name);
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, rendered).unwrap();
        }
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing golden {name}; run with UPDATE_GOLDEN=1"));
        assert_eq!(rendered, expected, "{name} drifted from golden");
    }

    #[test]
    fn txt_matches_golden() {
        assert_golden(
            "basic.txt",
            &export(&golden_transcript(), ExportFormat::Txt).unwrap(),
        );
    }

    #[test]
    fn srt_matches_golden() {
        assert_golden(
            "basic.srt",
            &export(&golden_transcript(), ExportFormat::Srt).unwrap(),
        );
    }

    #[test]
    fn vtt_matches_golden() {
        assert_golden(
            "basic.vtt",
            &export(&golden_transcript(), ExportFormat::Vtt).unwrap(),
        );
    }

    #[test]
    fn json_matches_golden() {
        assert_golden(
            "basic.json",
            &export(&golden_transcript(), ExportFormat::Json).unwrap(),
        );
    }

    #[test]
    fn timestamps_carry_hours_and_round_milliseconds() {
        assert_eq!(timestamp(0.0, ','), "00:00:00,000");
        assert_eq!(timestamp(1.5, ','), "00:00:01,500");
        assert_eq!(timestamp(3661.0015, '.'), "01:01:01.002");
        assert_eq!(timestamp(7322.999, ','), "02:02:02,999");
    }

    #[test]
    fn json_is_valid_and_source_reflects_recording() {
        let mut t = golden_transcript();
        t.audio_relative_path = Some(format!("{}.wav", t.id));
        let rendered = export(&t, ExportFormat::Json).unwrap();
        let v: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(v["source"], "mic");
        assert_eq!(v["segments"][1]["speaker"], serde_json::Value::Null);
        assert_eq!(v["segments"][0]["text"], "And so my fellow Americans,");
    }

    #[test]
    fn empty_transcript_exports_cleanly() {
        let mut t = golden_transcript();
        t.segments.clear();
        assert_eq!(export(&t, ExportFormat::Txt).unwrap(), "\n");
        assert_eq!(export(&t, ExportFormat::Srt).unwrap(), "");
        assert_eq!(export(&t, ExportFormat::Vtt).unwrap(), "WEBVTT\n\n");
        let v: serde_json::Value =
            serde_json::from_str(&export(&t, ExportFormat::Json).unwrap()).unwrap();
        assert_eq!(v["text"], "");
    }
}
