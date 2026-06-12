//! Managed application state behind every IPC command.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use attestrum_transcription_core::audio::RecordingSession;
use attestrum_transcription_core::engine::WhisperEngine;
use attestrum_transcription_core::store::Store;

/// A stopped recording waiting to be transcribed (or discarded by the next
/// `start_recording`).
pub struct FinishedRecording {
    /// 16 kHz mono — fed straight to the engine without re-decoding.
    pub samples: Vec<f32>,
    /// Temp WAV inside the store's audio dir; renamed to the transcript id
    /// when a transcription of it saves.
    pub wav_path: PathBuf,
}

pub struct AppState {
    /// Arc so transcription worker threads can save without holding `State`.
    pub store: Arc<Mutex<Store>>,
    pub http: reqwest::Client,
    /// Loaded whisper contexts, keyed by model id — loading is the expensive
    /// part, so each model loads once per app run.
    pub engines: Mutex<HashMap<String, Arc<WhisperEngine>>>,
    /// In-flight model downloads, keyed by model id → cancel flag.
    pub downloads: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// In-flight transcription jobs → cancel flag. Arc so the worker thread
    /// can deregister itself on completion.
    pub jobs: Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>>,
    next_job_id: AtomicU64,
    pub recording: Mutex<Option<RecordingSession>>,
    pub last_recording: Mutex<Option<FinishedRecording>>,
}

impl AppState {
    pub fn new(store: Store) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            http: reqwest::Client::new(),
            engines: Mutex::new(HashMap::new()),
            downloads: Mutex::new(HashMap::new()),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            next_job_id: AtomicU64::new(1),
            recording: Mutex::new(None),
            last_recording: Mutex::new(None),
        }
    }

    pub fn next_job_id(&self) -> u64 {
        self.next_job_id.fetch_add(1, Ordering::Relaxed)
    }
}
