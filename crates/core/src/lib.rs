//! Core engine for Attestrum Transcription.
//!
//! Everything that is not Tauri IPC glue lives here so it can be unit-tested
//! without spawning a webview: audio capture and decoding, Whisper inference,
//! the transcript store, and exporters. The `src-tauri` crate is a thin shell
//! over this one. Architecture contract: `docs/diagrams/architecture/app-overview.md`.

pub mod engine;
pub mod error;
pub mod models;

pub use error::CoreError;

/// User-facing product name, shared by the shell and exporters.
pub const PRODUCT_NAME: &str = "Attestrum Transcription";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_name_is_branded() {
        assert_eq!(PRODUCT_NAME, "Attestrum Transcription");
    }
}
