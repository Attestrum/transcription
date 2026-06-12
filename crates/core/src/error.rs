use std::path::PathBuf;

/// Unified error type for the core engine. The Tauri shell maps this into the
/// serialized `AppError` the frontend sees.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("unknown model id: {0}")]
    UnknownModel(String),

    #[error("insufficient disk space in {dir}: need {needed_bytes} more bytes, {available_bytes} available")]
    InsufficientDisk {
        dir: PathBuf,
        needed_bytes: u64,
        available_bytes: u64,
    },

    #[error("checksum mismatch for {model_id}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        model_id: String,
        expected: String,
        actual: String,
    },

    #[error("download failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("unexpected HTTP status {status} from {url}")]
    HttpStatus { status: u16, url: String },

    #[error("model file not found: {0}")]
    ModelNotFound(PathBuf),

    #[error("unsupported or unreadable media file: {0}")]
    UnsupportedMedia(PathBuf),

    #[error("decode failed: {0}")]
    Decode(String),

    #[error("file decoded to zero audio samples: {0}")]
    EmptyAudio(PathBuf),

    #[error("engine: {0}")]
    Engine(String),

    #[error("cancelled")]
    Cancelled,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
