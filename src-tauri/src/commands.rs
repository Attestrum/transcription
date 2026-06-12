//! The IPC command surface of
//! `docs/diagrams/architecture/ipc-transcribe-sequence.md`. Commands are
//! request/response; everything long-running streams back as events:
//! `model:download:{progress,done,error}` · `record:level` ·
//! `transcribe:{segment,progress,done,error,cancelled}`.
//!
//! Player commands (`player_*`) land with the rodio playback work (M8).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use attestrum_transcription_core::audio::{self, InputDevice, LevelUpdate};
use attestrum_transcription_core::engine::{Segment, TranscribeOptions, WhisperEngine};
use attestrum_transcription_core::export::{self, ExportFormat};
use attestrum_transcription_core::models::{self, DownloadProgress, ModelState};
use attestrum_transcription_core::store::{
    Settings, Transcript, TranscriptMeta, TranscriptSegment,
};
use attestrum_transcription_core::CoreError;

use crate::error::AppError;
use crate::state::{AppState, FinishedRecording};

type CmdResult<T> = Result<T, AppError>;

// ---------------------------------------------------------------- product

#[derive(Serialize)]
pub struct ProductInfo {
    name: &'static str,
    version: &'static str,
}

#[tauri::command]
pub fn product_info() -> ProductInfo {
    ProductInfo {
        name: attestrum_transcription_core::PRODUCT_NAME,
        version: env!("CARGO_PKG_VERSION"),
    }
}

// ----------------------------------------------------------------- models

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: &'static str,
    pub display_name: &'static str,
    pub size_bytes: u64,
    pub state: ModelState,
    pub is_default: bool,
}

#[tauri::command]
pub fn list_models(state: State<'_, AppState>) -> CmdResult<Vec<ModelInfo>> {
    let models_dir = state.store.lock().unwrap().models_dir();
    Ok(models::CATALOG
        .iter()
        .map(|spec| ModelInfo {
            id: spec.id,
            display_name: spec.display_name,
            size_bytes: spec.size_bytes,
            state: models::model_state(&models_dir, spec),
            is_default: spec.id == models::default_model_id(),
        })
        .collect())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadEvent {
    model_id: String,
    bytes: u64,
    total: u64,
}

/// Kick off (or resume) a model download. Returns immediately; progress
/// streams as `model:download:progress` (throttled to ~10 Hz).
#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> CmdResult<()> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut downloads = state.downloads.lock().unwrap();
        if downloads.contains_key(&model_id) {
            return Err(AppError::busy(format!("{model_id} is already downloading")));
        }
        downloads.insert(model_id.clone(), Arc::clone(&cancel));
    }
    let models_dir = state.store.lock().unwrap().models_dir();
    let client = state.http.clone();

    let result = {
        let app = app.clone();
        let id_for_request = model_id.clone();
        let model_id = model_id.clone();
        let mut last_emit = Instant::now() - Duration::from_secs(1);
        models::download_model(
            &client,
            &models_dir,
            &id_for_request,
            &cancel,
            move |p: DownloadProgress| {
                if last_emit.elapsed() >= Duration::from_millis(100) || p.bytes == p.total {
                    last_emit = Instant::now();
                    let _ = app.emit(
                        "model:download:progress",
                        DownloadEvent {
                            model_id: model_id.clone(),
                            bytes: p.bytes,
                            total: p.total,
                        },
                    );
                }
            },
        )
        .await
    };

    state.downloads.lock().unwrap().remove(&model_id);
    match result {
        Ok(_) => {
            let _ = app.emit("model:download:done", &model_id);
            Ok(())
        }
        Err(e) => {
            let err: AppError = e.into();
            let _ = app.emit("model:download:error", (&model_id, &err));
            Err(err)
        }
    }
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>, model_id: String) -> CmdResult<()> {
    match state.downloads.lock().unwrap().get(&model_id) {
        Some(cancel) => {
            cancel.store(true, Ordering::Relaxed);
            Ok(())
        }
        None => Err(AppError::bad_request(format!(
            "no download in flight for {model_id}"
        ))),
    }
}

#[tauri::command]
pub fn delete_model(state: State<'_, AppState>, model_id: String) -> CmdResult<()> {
    let spec = models::spec(&model_id).ok_or_else(|| CoreError::UnknownModel(model_id.clone()))?;
    let models_dir = state.store.lock().unwrap().models_dir();
    models::delete_model(&models_dir, spec)?;
    Ok(())
}

// -------------------------------------------------------------- recording

#[tauri::command]
pub fn list_input_devices() -> CmdResult<Vec<InputDevice>> {
    Ok(audio::list_input_devices()?)
}

#[tauri::command]
pub fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> CmdResult<()> {
    let mut recording = state.recording.lock().unwrap();
    if recording.is_some() {
        return Err(AppError::busy("a recording is already in progress"));
    }
    let wav_path = state
        .store
        .lock()
        .unwrap()
        .audio_dir()
        .join(format!("{}.wav", uuid::Uuid::new_v4()));
    let session = audio::start_recording(
        device_id.as_deref(),
        &wav_path,
        move |level: LevelUpdate| {
            let _ = app.emit("record:level", level);
        },
    )?;
    *recording = Some(session);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingInfo {
    pub duration_secs: f64,
    pub interrupted: bool,
}

/// Stop the live recording. The audio stays in memory (and on disk as a
/// temp WAV) for the next `transcribe {type: "recording"}` call.
#[tauri::command]
pub fn stop_recording(state: State<'_, AppState>) -> CmdResult<RecordingInfo> {
    let session = state
        .recording
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| AppError::bad_request("no recording in progress"))?;
    let result = session.stop()?;
    let info = RecordingInfo {
        duration_secs: result.duration_secs,
        interrupted: result.interrupted,
    };
    *state.last_recording.lock().unwrap() = Some(FinishedRecording {
        samples: result.samples,
        wav_path: result.wav_path,
    });
    Ok(info)
}

// ------------------------------------------------------------- transcribe

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TranscribeSource {
    /// Decode this media file.
    File { path: String },
    /// Use the most recently stopped recording.
    Recording,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SegmentEvent {
    job_id: u64,
    segments: Vec<Segment>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    job_id: u64,
    /// "decode" while the import pipeline runs, then "transcribe".
    phase: &'static str,
    pct: i32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoneEvent {
    job_id: u64,
    transcript_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobErrorEvent {
    job_id: u64,
    error: AppError,
}

/// Start a transcription job; returns the job id immediately. Segments
/// stream as `transcribe:segment` (coalesced ≤ 50 ms), progress as
/// `transcribe:progress`, and the job ends with exactly one of
/// `transcribe:{done,cancelled,error}`.
#[tauri::command]
pub fn transcribe(
    app: AppHandle,
    state: State<'_, AppState>,
    source: TranscribeSource,
    model_id: String,
    language: Option<String>,
) -> CmdResult<u64> {
    let job_id = state.next_job_id();
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .jobs
        .lock()
        .unwrap()
        .insert(job_id, Arc::clone(&cancel));

    // Resolve everything that can fail fast BEFORE spawning, so the caller
    // gets a typed error instead of an error event for setup problems.
    let spec = models::spec(&model_id).ok_or_else(|| CoreError::UnknownModel(model_id.clone()))?;
    let (models_dir, audio_dir) = {
        let store = state.store.lock().unwrap();
        (store.models_dir(), store.audio_dir())
    };
    let model_path = models::model_path(&models_dir, spec)
        .ok_or_else(|| CoreError::ModelNotFound(models_dir.join(spec.filename)))?;

    let engine = {
        let mut engines = state.engines.lock().unwrap();
        match engines.get(&model_id) {
            Some(e) => Arc::clone(e),
            None => {
                let e = Arc::new(WhisperEngine::load(&model_path)?);
                engines.insert(model_id.clone(), Arc::clone(&e));
                e
            }
        }
    };

    let input = match source {
        TranscribeSource::File { path } => JobInput::File(PathBuf::from(path)),
        TranscribeSource::Recording => {
            let rec = state
                .last_recording
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| AppError::bad_request("no finished recording available"))?;
            JobInput::Recording(rec)
        }
    };

    let ctx = JobContext {
        app,
        job_id,
        cancel,
        engine,
        model_id,
        language,
        audio_dir,
        store: Arc::clone(&state.store),
    };
    std::thread::Builder::new()
        .name(format!("transcribe-{job_id}"))
        .spawn({
            let jobs = Arc::clone(&state.jobs);
            move || {
                run_job(ctx, input);
                jobs.lock().unwrap().remove(&job_id);
            }
        })
        .map_err(|e| CoreError::Engine(format!("spawn job thread: {e}")))?;

    Ok(job_id)
}

#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, job_id: u64) -> CmdResult<()> {
    match state.jobs.lock().unwrap().get(&job_id) {
        Some(cancel) => {
            cancel.store(true, Ordering::Relaxed);
            Ok(())
        }
        None => Err(AppError::bad_request(format!("no job {job_id}"))),
    }
}

enum JobInput {
    File(PathBuf),
    Recording(FinishedRecording),
}

struct JobContext {
    app: AppHandle,
    job_id: u64,
    cancel: Arc<AtomicBool>,
    engine: Arc<WhisperEngine>,
    model_id: String,
    language: Option<String>,
    audio_dir: PathBuf,
    store: Arc<std::sync::Mutex<attestrum_transcription_core::store::Store>>,
}

/// The worker behind `transcribe` — decode, run whisper with coalesced
/// segment events, save the transcript, emit the terminal event.
fn run_job(ctx: JobContext, input: JobInput) {
    let started = Instant::now();
    match run_job_inner(&ctx, input, started) {
        Ok(transcript_id) => {
            let _ = ctx.app.emit(
                "transcribe:done",
                DoneEvent {
                    job_id: ctx.job_id,
                    transcript_id,
                },
            );
        }
        Err(e) if matches!(e.kind, crate::error::ErrorKind::Cancelled) => {
            let _ = ctx.app.emit("transcribe:cancelled", ctx.job_id);
        }
        Err(error) => {
            let _ = ctx.app.emit(
                "transcribe:error",
                JobErrorEvent {
                    job_id: ctx.job_id,
                    error,
                },
            );
        }
    }
}

fn run_job_inner(ctx: &JobContext, input: JobInput, started: Instant) -> Result<String, AppError> {
    let (samples, title, source_filename, recording_wav) = match input {
        JobInput::File(path) => {
            let app = ctx.app.clone();
            let job_id = ctx.job_id;
            let imported = audio::import_file(&path, Arc::clone(&ctx.cancel), move |p| {
                let _ = app.emit(
                    "transcribe:progress",
                    ProgressEvent {
                        job_id,
                        phase: "decode",
                        pct: (p * 100.0) as i32,
                    },
                );
            })?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Imported audio".into());
            let title = path
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Imported audio".into());
            (imported.samples, title, Some(name), None)
        }
        JobInput::Recording(rec) => {
            let title = recording_title();
            (rec.samples, title, None, Some(rec.wav_path))
        }
    };

    // Coalesce segment events: whisper can emit several segments in quick
    // succession on fast hardware; batch anything within 50 ms.
    let pending = Arc::new(std::sync::Mutex::new(Vec::<Segment>::new()));
    let on_segment = {
        let app = ctx.app.clone();
        let job_id = ctx.job_id;
        let pending = Arc::clone(&pending);
        let mut last_emit = Instant::now();
        move |seg: Segment| {
            let mut buf = pending.lock().unwrap();
            buf.push(seg);
            if last_emit.elapsed() >= Duration::from_millis(50) {
                last_emit = Instant::now();
                let segments = std::mem::take(&mut *buf);
                drop(buf);
                let _ = app.emit("transcribe:segment", SegmentEvent { job_id, segments });
            }
        }
    };
    let on_progress = {
        let app = ctx.app.clone();
        let job_id = ctx.job_id;
        move |pct: i32| {
            let _ = app.emit(
                "transcribe:progress",
                ProgressEvent {
                    job_id,
                    phase: "transcribe",
                    pct,
                },
            );
        }
    };

    let options = TranscribeOptions {
        language: ctx.language.clone(),
    };
    let segments = ctx.engine.transcribe(
        &samples,
        &options,
        Arc::clone(&ctx.cancel),
        on_segment,
        on_progress,
    )?;

    // Flush whatever the coalescer still holds.
    {
        let mut buf = pending.lock().unwrap();
        if !buf.is_empty() {
            let segments = std::mem::take(&mut *buf);
            let _ = ctx.app.emit(
                "transcribe:segment",
                SegmentEvent {
                    job_id: ctx.job_id,
                    segments,
                },
            );
        }
    }

    let duration = samples.len() as f64 / audio::TARGET_SAMPLE_RATE as f64;
    let mut transcript = Transcript::new(
        title,
        source_filename,
        duration,
        ctx.model_id.clone(),
        ctx.language.clone().unwrap_or_else(|| "auto".into()),
        segments
            .into_iter()
            .enumerate()
            .map(|(i, s)| TranscriptSegment {
                id: i as u32,
                start: s.start,
                end: s.end,
                text: s.text,
                original_text: None,
            })
            .collect(),
    );
    transcript.transcription_duration_secs = Some(started.elapsed().as_secs_f64());

    // A recording's temp WAV becomes the transcript's sibling archive.
    if let Some(tmp_wav) = recording_wav {
        let rel = format!("{}.wav", transcript.id);
        std::fs::rename(&tmp_wav, ctx.audio_dir.join(&rel)).map_err(CoreError::Io)?;
        transcript.audio_relative_path = Some(rel);
    }

    ctx.store.lock().unwrap().save(&transcript)?;
    Ok(transcript.id)
}

/// "Recording 2026-06-12 15:04"
fn recording_title() -> String {
    let now = time::OffsetDateTime::now_utc();
    format!(
        "Recording {:04}-{:02}-{:02} {:02}:{:02}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute()
    )
}

// ---------------------------------------------------------------- library

#[tauri::command]
pub fn list_transcripts(state: State<'_, AppState>) -> CmdResult<Vec<TranscriptMeta>> {
    Ok(state.store.lock().unwrap().list()?)
}

#[tauri::command]
pub fn get_transcript(state: State<'_, AppState>, id: String) -> CmdResult<Transcript> {
    Ok(state.store.lock().unwrap().load(&id)?)
}

/// Apply a user edit to one segment (first edit preserves `originalText`).
/// Returns the updated transcript.
#[tauri::command]
pub fn update_transcript(
    state: State<'_, AppState>,
    id: String,
    segment_id: u32,
    text: String,
) -> CmdResult<Transcript> {
    let store = state.store.lock().unwrap();
    let mut t = store.load(&id)?;
    if !t.edit_segment(segment_id, &text) {
        return Err(AppError::bad_request(format!(
            "transcript {id} has no segment {segment_id}"
        )));
    }
    store.save(&t)?;
    Ok(t)
}

#[tauri::command]
pub fn rename_transcript(
    state: State<'_, AppState>,
    id: String,
    title: String,
) -> CmdResult<Transcript> {
    let store = state.store.lock().unwrap();
    let mut t = store.load(&id)?;
    t.title = title;
    t.updated_at = time::OffsetDateTime::now_utc();
    store.save(&t)?;
    Ok(t)
}

#[tauri::command]
pub fn delete_transcript(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    Ok(state.store.lock().unwrap().delete(&id)?)
}

// ----------------------------------------------------------------- export

#[tauri::command]
pub fn export_transcript(
    state: State<'_, AppState>,
    id: String,
    format: ExportFormat,
    dest_path: String,
) -> CmdResult<()> {
    let t = state.store.lock().unwrap().load(&id)?;
    let rendered = export::export(&t, format)?;
    std::fs::write(&dest_path, rendered).map_err(CoreError::Io)?;
    Ok(())
}

// --------------------------------------------------------------- settings

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> CmdResult<Settings> {
    Ok(state.store.lock().unwrap().load_settings())
}

/// Persist settings. A changed `storageDir` takes effect on next launch.
#[tauri::command]
pub fn set_settings(state: State<'_, AppState>, settings: Settings) -> CmdResult<()> {
    Ok(state.store.lock().unwrap().save_settings(&settings)?)
}
