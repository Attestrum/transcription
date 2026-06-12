//! Whisper inference wrapper.
//!
//! Implements the engine half of `docs/diagrams/pipeline/transcription-flow.md`:
//! a blocking `transcribe` over 16 kHz mono f32 PCM that streams segments to a
//! callback as whisper emits them, reports progress, and aborts cooperatively
//! via an `AtomicBool`. The caller (the IPC shell's job runner) owns the
//! dedicated thread this runs on — nothing here spawns.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::error::CoreError;

/// One transcribed segment. Timestamps are seconds from the start of the audio.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Options for one transcription run.
#[derive(Debug, Clone, Default)]
pub struct TranscribeOptions {
    /// ISO 639-1 language code; `None` lets whisper auto-detect.
    pub language: Option<String>,
}

/// A loaded whisper model, reusable across transcription runs (loading the
/// model is the expensive part — keep one of these alive per selected model).
pub struct WhisperEngine {
    context: WhisperContext,
}

impl WhisperEngine {
    /// Load a verified model file from disk.
    pub fn load(model_path: &Path) -> Result<Self, CoreError> {
        if !model_path.is_file() {
            return Err(CoreError::ModelNotFound(model_path.to_owned()));
        }
        let path_str = model_path
            .to_str()
            .ok_or_else(|| CoreError::Engine("model path is not valid UTF-8".into()))?;
        let context =
            WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
                .map_err(|e| CoreError::Engine(format!("failed to load model: {e}")))?;
        Ok(Self { context })
    }

    /// Transcribe 16 kHz mono f32 PCM, blocking until done or cancelled.
    ///
    /// `on_segment` fires as whisper emits each segment (the streaming UI
    /// path); `on_progress` receives 0–100. The returned Vec is the complete,
    /// authoritative segment list re-read from whisper state after the run.
    pub fn transcribe(
        &self,
        pcm_16k_mono: &[f32],
        options: &TranscribeOptions,
        cancel: Arc<AtomicBool>,
        on_segment: impl FnMut(Segment) + Send + 'static,
        on_progress: impl FnMut(i32) + Send + 'static,
    ) -> Result<Vec<Segment>, CoreError> {
        let mut state = self
            .context
            .create_state()
            .map_err(|e| CoreError::Engine(format!("failed to create state: {e}")))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        // `set_language` borrows the &str for the params' lifetime; keep the
        // owned string alive alongside params.
        let language = options.language.clone();
        params.set_language(Some(language.as_deref().unwrap_or("auto")));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let mut on_segment = on_segment;
        params.set_segment_callback_safe(move |data: whisper_rs::SegmentCallbackData| {
            on_segment(Segment {
                start: centiseconds_to_secs(data.start_timestamp),
                end: centiseconds_to_secs(data.end_timestamp),
                text: data.text.trim().to_owned(),
            });
        });

        let mut on_progress = on_progress;
        params.set_progress_callback_safe(move |pct: i32| on_progress(pct));

        // whisper-rs 0.16.0's `set_abort_callback_safe` is unsound: its
        // trampoline casts the double-boxed closure to the caller's closure
        // type (the sibling segment/progress trampolines correctly cast to the
        // boxed-dyn type), so the callback reads garbage and aborts every run
        // (whisper error -6). Shim the raw C callback over the AtomicBool
        // directly instead.
        unsafe extern "C" fn abort_when_flagged(user_data: *mut std::ffi::c_void) -> bool {
            // SAFETY: `user_data` is the data pointer of the `Arc<AtomicBool>`
            // held by this frame for the whole `state.full()` call below.
            unsafe { (*(user_data as *const AtomicBool)).load(Ordering::Relaxed) }
        }
        unsafe {
            params.set_abort_callback(Some(abort_when_flagged));
            params.set_abort_callback_user_data(Arc::as_ptr(&cancel) as *mut std::ffi::c_void);
        }

        match state.full(params, pcm_16k_mono) {
            Ok(_) => {}
            Err(e) => {
                // whisper reports an abort as a generic failure; distinguish
                // a user cancel from a real engine error.
                if cancel.load(Ordering::Relaxed) {
                    return Err(CoreError::Cancelled);
                }
                return Err(CoreError::Engine(format!("inference failed: {e}")));
            }
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled);
        }

        // Authoritative read-back of everything whisper decoded.
        let n = state.full_n_segments();
        let mut segments = Vec::with_capacity(n as usize);
        for i in 0..n {
            let seg = state
                .get_segment(i)
                .ok_or_else(|| CoreError::Engine(format!("segment {i} out of bounds")))?;
            let text = seg
                .to_str_lossy()
                .map_err(|e| CoreError::Engine(format!("segment {i} text: {e}")))?;
            segments.push(Segment {
                start: centiseconds_to_secs(seg.start_timestamp()),
                end: centiseconds_to_secs(seg.end_timestamp()),
                text: text.trim().to_owned(),
            });
        }
        Ok(segments)
    }
}

/// whisper timestamps are in centiseconds (units of 10 ms).
fn centiseconds_to_secs(t: i64) -> f64 {
    t as f64 / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    #[test]
    fn missing_model_file_is_reported() {
        // WhisperContext isn't Debug, so unwrap_err can't be used here.
        let Err(err) = WhisperEngine::load(Path::new("/nonexistent/ggml-void.bin")) else {
            panic!("expected ModelNotFound");
        };
        assert!(matches!(err, CoreError::ModelNotFound(_)), "got {err:?}");
    }

    #[test]
    fn centiseconds_convert_to_seconds() {
        assert_eq!(centiseconds_to_secs(0), 0.0);
        assert_eq!(centiseconds_to_secs(150), 1.5);
        assert_eq!(centiseconds_to_secs(360_000), 3600.0);
    }

    fn fixture_pcm() -> Vec<f32> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jfk.wav");
        let mut reader = hound::WavReader::open(path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, 16_000);
        assert_eq!(spec.channels, 1);
        reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / i16::MAX as f32)
            .collect()
    }

    fn tiny_model_path() -> Option<PathBuf> {
        // CI (and a prepared dev box) provides the real tiny model here; the
        // ignored tests below are run explicitly with --ignored once it exists.
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/models/ggml-tiny.bin");
        p.is_file().then_some(p)
    }

    #[test]
    #[ignore = "requires tests/models/ggml-tiny.bin (downloaded in CI)"]
    fn tiny_model_transcribes_jfk_fixture() {
        let model = tiny_model_path().expect("ggml-tiny.bin missing — download it first");
        let engine = WhisperEngine::load(&model).unwrap();

        let streamed: Arc<Mutex<Vec<Segment>>> = Arc::default();
        let streamed_in_cb = Arc::clone(&streamed);
        let progress_seen = Arc::new(AtomicBool::new(false));
        let progress_in_cb = Arc::clone(&progress_seen);

        let segments = engine
            .transcribe(
                &fixture_pcm(),
                &TranscribeOptions {
                    language: Some("en".into()),
                },
                Arc::new(AtomicBool::new(false)),
                move |s| streamed_in_cb.lock().unwrap().push(s),
                move |_| progress_in_cb.store(true, Ordering::Relaxed),
            )
            .unwrap();

        let full_text = segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        assert!(
            full_text.contains("ask not what your country can do for you"),
            "unexpected transcription: {full_text:?}"
        );
        // Streaming callback saw the same segments as the final read-back.
        assert_eq!(*streamed.lock().unwrap(), segments);
        assert!(segments[0].start >= 0.0 && segments.last().unwrap().end > 5.0);
    }

    #[test]
    #[ignore = "requires tests/models/ggml-tiny.bin (downloaded in CI)"]
    fn pre_cancelled_run_returns_cancelled() {
        let model = tiny_model_path().expect("ggml-tiny.bin missing — download it first");
        let engine = WhisperEngine::load(&model).unwrap();
        let err = engine
            .transcribe(
                &fixture_pcm(),
                &TranscribeOptions::default(),
                Arc::new(AtomicBool::new(true)),
                |_| {},
                |_| {},
            )
            .unwrap_err();
        assert!(matches!(err, CoreError::Cancelled), "got {err:?}");
    }
}
