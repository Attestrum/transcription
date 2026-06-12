//! The serialized error the frontend sees. Every command returns
//! `Result<T, AppError>`; the TS client narrows on `kind`.

use attestrum_transcription_core::CoreError;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    UnknownModel,
    InsufficientDisk,
    ChecksumMismatch,
    Network,
    ModelNotFound,
    UnsupportedMedia,
    Decode,
    EmptyAudio,
    AudioDevice,
    Capture,
    Playback,
    TranscriptNotFound,
    Store,
    Engine,
    Cancelled,
    Io,
    /// A conflicting operation is already running (e.g. a second
    /// `start_recording` while one is live).
    Busy,
    /// The request itself was malformed (unknown job id, bad path).
    BadRequest,
}

impl AppError {
    pub fn busy(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Busy,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::BadRequest,
            message: message.into(),
        }
    }
}

impl From<CoreError> for AppError {
    fn from(e: CoreError) -> Self {
        let kind = match &e {
            CoreError::UnknownModel(_) => ErrorKind::UnknownModel,
            CoreError::InsufficientDisk { .. } => ErrorKind::InsufficientDisk,
            CoreError::ChecksumMismatch { .. } => ErrorKind::ChecksumMismatch,
            CoreError::Network(_) | CoreError::HttpStatus { .. } => ErrorKind::Network,
            CoreError::ModelNotFound(_) => ErrorKind::ModelNotFound,
            CoreError::UnsupportedMedia(_) => ErrorKind::UnsupportedMedia,
            CoreError::Decode(_) => ErrorKind::Decode,
            CoreError::EmptyAudio(_) => ErrorKind::EmptyAudio,
            CoreError::AudioDevice(_) => ErrorKind::AudioDevice,
            CoreError::Capture(_) => ErrorKind::Capture,
            CoreError::Playback(_) => ErrorKind::Playback,
            CoreError::TranscriptNotFound(_) => ErrorKind::TranscriptNotFound,
            CoreError::Store(_) => ErrorKind::Store,
            CoreError::Engine(_) => ErrorKind::Engine,
            CoreError::Cancelled => ErrorKind::Cancelled,
            CoreError::Io(_) => ErrorKind::Io,
        };
        Self {
            kind,
            message: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_map_to_stable_kinds() {
        let err: AppError = CoreError::Cancelled.into();
        assert_eq!(err.kind, ErrorKind::Cancelled);
        let err: AppError = CoreError::UnknownModel("x".into()).into();
        assert_eq!(err.kind, ErrorKind::UnknownModel);
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "unknown_model");
        assert!(json["message"].as_str().unwrap().contains('x'));
    }
}
