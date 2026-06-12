//! Audio input for the transcription pipeline.
//!
//! `import` covers the file-import path of
//! `docs/diagrams/pipeline/transcription-flow.md` (symphonia decode → mono
//! downmix → rubato resample → 16 kHz f32). Mic capture and the archive WAV
//! writer land with M5.

pub mod import;

pub use import::{import_file, ImportedAudio, TARGET_SAMPLE_RATE};
