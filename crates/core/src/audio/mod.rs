//! Audio input for the transcription pipeline — the `audio/` half of
//! `docs/diagrams/pipeline/transcription-flow.md`.
//!
//! `import` decodes media files (symphonia → mono downmix → rubato resample
//! → 16 kHz f32); `capture` records the microphone through the same resample
//! contract while archiving a WAV; `resample` is the shared rate converter.

pub mod capture;
pub mod import;
pub(crate) mod resample;

pub use capture::{
    list_input_devices, start_recording, CaptureResult, InputDevice, LevelUpdate, RecordingSession,
};
pub use import::{import_file, ImportedAudio};
pub use resample::TARGET_SAMPLE_RATE;
