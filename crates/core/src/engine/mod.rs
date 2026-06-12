//! Whisper inference engine.
//!
//! Pipeline contract: `docs/diagrams/pipeline/transcription-flow.md`.

pub mod whisper;

pub use whisper::{Segment, TranscribeOptions, WhisperEngine};
