//! Whisper model catalog and downloader.
//!
//! Lifecycle contract: `docs/diagrams/models/model-download.md`.

pub mod catalog;
pub mod download;

pub use catalog::{default_model_id, spec, ModelSpec, CATALOG, HF_BASE_URL};
pub use download::{
    delete_model, download_model, model_path, model_state, DownloadProgress, ModelState,
};
