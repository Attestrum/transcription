//! Microphone capture: cpal stream → mono downmix → 16 kHz resample, with a
//! ~30 Hz level meter and an incrementally-written archive WAV.
//!
//! Implements the record path of
//! `docs/diagrams/pipeline/transcription-flow.md`. The pipeline below the
//! device is a pure [`CaptureSink`] so it is unit-testable without audio
//! hardware; the cpal shell owns the (`!Send`) stream on a dedicated thread.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::resample::{MonoResampler, TARGET_SAMPLE_RATE};
use crate::error::CoreError;

/// Level-meter cadence: one update per this many output (16 kHz) frames,
/// ≈ 30 Hz.
const LEVEL_WINDOW: usize = (TARGET_SAMPLE_RATE / 30) as usize;

/// Flush the WAV header/data every this many output frames (≈ 1 s), so a
/// crash mid-recording leaves a readable file.
const WAV_FLUSH_INTERVAL: usize = TARGET_SAMPLE_RATE as usize;

/// An available audio input. `id` is the cpal device name — cpal exposes no
/// more stable identifier, and names are what settings persist.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct InputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// One level-meter sample over the last window.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelUpdate {
    pub rms: f32,
    pub peak: f32,
    /// Seconds of audio captured so far.
    pub elapsed_secs: f64,
}

/// A finished (or interrupted) recording.
#[derive(Debug)]
pub struct CaptureResult {
    /// 16 kHz mono f32 — ready for the whisper engine.
    pub samples: Vec<f32>,
    pub wav_path: PathBuf,
    pub duration_secs: f64,
    /// True when the stream died (device unplugged, etc.); `samples` and the
    /// WAV hold everything captured up to that point.
    pub interrupted: bool,
}

/// Enumerate input devices on the default host.
pub fn list_input_devices() -> Result<Vec<InputDevice>, CoreError> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();
    let devices = host
        .input_devices()
        .map_err(|e| CoreError::AudioDevice(e.to_string()))?;
    Ok(devices
        .filter_map(|d| d.name().ok())
        .map(|name| InputDevice {
            id: name.clone(),
            is_default: name == default_name,
            name,
        })
        .collect())
}

/// The device-independent capture pipeline: interleaved source-rate input in,
/// 16 kHz mono accumulation + incremental WAV + level callbacks out.
struct CaptureSink {
    channels: usize,
    resampler: Option<MonoResampler>,
    /// Mono samples at the source rate awaiting a full resampler chunk.
    pending: Vec<f32>,
    /// Accumulated 16 kHz mono output.
    samples: Vec<f32>,
    wav: Option<hound::WavWriter<BufWriter<File>>>,
    /// Output frames already written to the WAV (also drives level windows).
    wav_written: usize,
    /// Total source frames pushed — pins the exact output length at finalize.
    frames_in: u64,
    level_acc: LevelAccumulator,
    on_level: Box<dyn FnMut(LevelUpdate) + Send>,
    interrupted: bool,
}

struct LevelAccumulator {
    sum_sq: f64,
    peak: f32,
    count: usize,
}

impl LevelAccumulator {
    fn new() -> Self {
        Self {
            sum_sq: 0.0,
            peak: 0.0,
            count: 0,
        }
    }
}

impl CaptureSink {
    fn new(
        source_rate: u32,
        channels: usize,
        wav_path: &Path,
        on_level: Box<dyn FnMut(LevelUpdate) + Send>,
    ) -> Result<Self, CoreError> {
        if channels == 0 {
            return Err(CoreError::Capture("input stream has zero channels".into()));
        }
        let resampler = if source_rate != TARGET_SAMPLE_RATE {
            Some(MonoResampler::new(source_rate)?)
        } else {
            None
        };
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: TARGET_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let wav = hound::WavWriter::create(wav_path, spec)
            .map_err(|e| CoreError::Capture(format!("cannot create WAV: {e}")))?;
        Ok(Self {
            channels,
            resampler,
            pending: Vec::new(),
            samples: Vec::new(),
            wav: Some(wav),
            wav_written: 0,
            frames_in: 0,
            level_acc: LevelAccumulator::new(),
            on_level,
            interrupted: false,
        })
    }

    /// Feed one interleaved buffer from the device callback.
    fn push_interleaved(&mut self, data: &[f32]) -> Result<(), CoreError> {
        for frame in data.chunks_exact(self.channels) {
            self.pending
                .push(frame.iter().sum::<f32>() / self.channels as f32);
        }
        self.frames_in += (data.len() / self.channels) as u64;
        let before = self.samples.len();
        match &mut self.resampler {
            Some(r) => r.drain_full_chunks(&mut self.pending, &mut self.samples)?,
            None => self.samples.append(&mut self.pending),
        }
        self.consume_output(before)
    }

    /// WAV-write and level-meter everything newly appended to `samples`.
    fn consume_output(&mut self, from: usize) -> Result<(), CoreError> {
        let Some(wav) = self.wav.as_mut() else {
            return Ok(());
        };
        let mut flush_due = false;
        for &s in &self.samples[from..] {
            let clamped = s.clamp(-1.0, 1.0);
            wav.write_sample((clamped * 32767.0) as i16)
                .map_err(|e| CoreError::Capture(format!("WAV write: {e}")))?;
            self.wav_written += 1;
            if self.wav_written % WAV_FLUSH_INTERVAL == 0 {
                flush_due = true;
            }

            let acc = &mut self.level_acc;
            acc.sum_sq += (clamped as f64) * (clamped as f64);
            acc.peak = acc.peak.max(clamped.abs());
            acc.count += 1;
            if acc.count >= LEVEL_WINDOW {
                let update = LevelUpdate {
                    rms: (acc.sum_sq / acc.count as f64).sqrt() as f32,
                    peak: acc.peak,
                    elapsed_secs: self.wav_written as f64 / TARGET_SAMPLE_RATE as f64,
                };
                *acc = LevelAccumulator::new();
                (self.on_level)(update);
            }
        }
        if flush_due {
            wav.flush()
                .map_err(|e| CoreError::Capture(format!("WAV flush: {e}")))?;
        }
        Ok(())
    }

    /// Flush the resampler tail and close the WAV.
    fn finalize(mut self, wav_path: PathBuf) -> Result<CaptureResult, CoreError> {
        if let Some(mut r) = self.resampler.take() {
            let from = self.samples.len();
            let mut pending = std::mem::take(&mut self.pending);
            let mut out = std::mem::take(&mut self.samples);
            r.finish(&mut pending, self.frames_in, &mut out)?;
            self.samples = out;
            self.consume_output(from)?;
        }
        if let Some(wav) = self.wav.take() {
            wav.finalize()
                .map_err(|e| CoreError::Capture(format!("WAV finalize: {e}")))?;
        }
        let duration_secs = self.samples.len() as f64 / TARGET_SAMPLE_RATE as f64;
        Ok(CaptureResult {
            samples: self.samples,
            wav_path,
            duration_secs,
            interrupted: self.interrupted,
        })
    }
}

/// A recording in progress. Obtain with [`start_recording`]; call
/// [`RecordingSession::stop`] to finish and collect the result.
pub struct RecordingSession {
    stop: Arc<AtomicBool>,
    sink: Arc<Mutex<Option<CaptureSink>>>,
    wav_path: PathBuf,
    thread: JoinHandle<Result<(), CoreError>>,
}

/// Start capturing the named input device (`None` = system default) into
/// `wav_path`. `on_level` fires ≈ 30 times per second of captured audio.
pub fn start_recording(
    device_id: Option<&str>,
    wav_path: &Path,
    on_level: impl FnMut(LevelUpdate) + Send + 'static,
) -> Result<RecordingSession, CoreError> {
    let stop = Arc::new(AtomicBool::new(false));
    let sink: Arc<Mutex<Option<CaptureSink>>> = Arc::new(Mutex::new(None));
    let device_id = device_id.map(str::to_owned);
    let wav_path_owned = wav_path.to_owned();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), CoreError>>();

    let thread = std::thread::Builder::new()
        .name("mic-capture".into())
        .spawn({
            let stop = Arc::clone(&stop);
            let sink = Arc::clone(&sink);
            move || capture_thread(device_id, wav_path_owned, on_level, stop, sink, ready_tx)
        })
        .map_err(|e| CoreError::Capture(format!("spawn capture thread: {e}")))?;

    // Surface device/stream setup failures synchronously.
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(RecordingSession {
            stop,
            sink,
            wav_path: wav_path.to_owned(),
            thread,
        }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => {
            let _ = thread.join();
            Err(CoreError::Capture(
                "capture thread died during setup".into(),
            ))
        }
    }
}

impl RecordingSession {
    /// Stop the stream and return everything captured.
    pub fn stop(self) -> Result<CaptureResult, CoreError> {
        self.stop.store(true, Ordering::Relaxed);
        match self.thread.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(CoreError::Capture("capture thread panicked".into())),
        }
        let sink = self
            .sink
            .lock()
            .map_err(|_| CoreError::Capture("capture state poisoned".into()))?
            .take()
            .ok_or_else(|| CoreError::Capture("capture state missing".into()))?;
        sink.finalize(self.wav_path)
    }
}

/// Body of the dedicated capture thread — owns the cpal stream (which is not
/// `Send`) from build to drop.
fn capture_thread(
    device_id: Option<String>,
    wav_path: PathBuf,
    on_level: impl FnMut(LevelUpdate) + Send + 'static,
    stop: Arc<AtomicBool>,
    sink_slot: Arc<Mutex<Option<CaptureSink>>>,
    ready_tx: mpsc::Sender<Result<(), CoreError>>,
) -> Result<(), CoreError> {
    let setup = || -> Result<(cpal::Device, cpal::SupportedStreamConfig), CoreError> {
        let host = cpal::default_host();
        let device = match &device_id {
            Some(id) => host
                .input_devices()
                .map_err(|e| CoreError::AudioDevice(e.to_string()))?
                .find(|d| d.name().is_ok_and(|n| &n == id))
                .ok_or_else(|| CoreError::AudioDevice(format!("input device not found: {id}")))?,
            None => host
                .default_input_device()
                .ok_or_else(|| CoreError::AudioDevice("no default input device".into()))?,
        };
        let config = device
            .default_input_config()
            .map_err(|e| CoreError::AudioDevice(e.to_string()))?;
        Ok((device, config))
    };

    let (device, supported) = match setup() {
        Ok(x) => x,
        Err(e) => {
            let _ = ready_tx.send(Err(e));
            return Ok(());
        }
    };
    let channels = supported.channels() as usize;
    let source_rate = supported.sample_rate().0;
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    match CaptureSink::new(source_rate, channels, &wav_path, Box::new(on_level)) {
        Ok(s) => *sink_slot.lock().unwrap() = Some(s),
        Err(e) => {
            let _ = ready_tx.send(Err(e));
            return Ok(());
        }
    }

    let errored = Arc::new(AtomicBool::new(false));
    let stream = build_stream(
        &device,
        &config,
        sample_format,
        Arc::clone(&sink_slot),
        Arc::clone(&errored),
    );
    let stream = match stream {
        Ok(s) => s,
        Err(e) => {
            let _ = ready_tx.send(Err(e));
            return Ok(());
        }
    };
    if let Err(e) = stream.play() {
        let _ = ready_tx.send(Err(CoreError::Capture(format!("stream start: {e}"))));
        return Ok(());
    }
    let _ = ready_tx.send(Ok(()));

    while !stop.load(Ordering::Relaxed) && !errored.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(25));
    }
    if errored.load(Ordering::Relaxed) {
        if let Some(sink) = sink_slot.lock().unwrap().as_mut() {
            sink.interrupted = true;
        }
    }
    drop(stream);
    Ok(())
}

/// Build the input stream for whichever sample format the device negotiated,
/// converting to f32 in the callback.
fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    sink: Arc<Mutex<Option<CaptureSink>>>,
    errored: Arc<AtomicBool>,
) -> Result<cpal::Stream, CoreError> {
    fn data_fn<T: cpal::SizedSample>(
        sink: Arc<Mutex<Option<CaptureSink>>>,
        errored: Arc<AtomicBool>,
    ) -> impl FnMut(&[T], &cpal::InputCallbackInfo)
    where
        f32: cpal::FromSample<T>,
    {
        use cpal::Sample;
        let mut scratch: Vec<f32> = Vec::new();
        move |data, _| {
            scratch.clear();
            scratch.extend(data.iter().map(|&s| f32::from_sample(s)));
            if let Some(sink) = sink.lock().unwrap().as_mut() {
                if sink.push_interleaved(&scratch).is_err() {
                    errored.store(true, Ordering::Relaxed);
                }
            }
        }
    }
    let err_fn = {
        let errored = Arc::clone(&errored);
        move |_err: cpal::StreamError| {
            errored.store(true, Ordering::Relaxed);
        }
    };
    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(config, data_fn::<f32>(sink, errored), err_fn, None)
        }
        cpal::SampleFormat::I16 => {
            device.build_input_stream(config, data_fn::<i16>(sink, errored), err_fn, None)
        }
        cpal::SampleFormat::U16 => {
            device.build_input_stream(config, data_fn::<u16>(sink, errored), err_fn, None)
        }
        other => {
            return Err(CoreError::Capture(format!(
                "unsupported input sample format: {other:?}"
            )))
        }
    };
    stream.map_err(|e| CoreError::Capture(format!("stream build: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn temp_wav(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("attestrum-capture-{tag}.wav"))
    }

    /// 440 Hz sine, interleaved stereo at the given rate.
    fn stereo_sine(rate: u32, secs: f64) -> Vec<f32> {
        let frames = (rate as f64 * secs) as usize;
        let mut v = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let s =
                (2.0 * std::f64::consts::PI * 440.0 * i as f64 / rate as f64).sin() as f32 * 0.35;
            v.push(s);
            v.push(s);
        }
        v
    }

    fn zero_crossings_per_sec(samples: &[f32], duration: f64) -> f64 {
        let mut last = 0.0f32;
        let mut crossings = 0usize;
        for &s in samples.iter().filter(|s| s.abs() > 0.01) {
            if last != 0.0 && s.signum() != last.signum() {
                crossings += 1;
            }
            last = s;
        }
        crossings as f64 / duration
    }

    #[test]
    fn sink_downmixes_resamples_and_archives() {
        let wav_path = temp_wav("48k-stereo");
        let levels = Arc::new(AtomicUsize::new(0));
        let levels_in_cb = Arc::clone(&levels);
        let mut sink = CaptureSink::new(
            48_000,
            2,
            &wav_path,
            Box::new(move |u: LevelUpdate| {
                assert!(u.rms > 0.0 && u.peak >= u.rms);
                levels_in_cb.fetch_add(1, Ordering::Relaxed);
            }),
        )
        .unwrap();

        // Feed 2 s in device-callback-sized pieces.
        let input = stereo_sine(48_000, 2.0);
        for chunk in input.chunks(960) {
            sink.push_interleaved(chunk).unwrap();
        }
        let result = sink.finalize(wav_path.clone()).unwrap();

        let expected = 2.0 * TARGET_SAMPLE_RATE as f64;
        assert_eq!(result.samples.len() as f64, expected);
        assert!(!result.interrupted);
        assert!((result.duration_secs - 2.0).abs() < 1e-9);
        let zc = zero_crossings_per_sec(&result.samples, result.duration_secs);
        assert!((zc - 880.0).abs() / 880.0 < 0.05, "zc/s {zc}");
        // ~30 level windows per second.
        let n_levels = levels.load(Ordering::Relaxed);
        assert!((55..=62).contains(&n_levels), "level updates: {n_levels}");

        // The WAV holds the same audio (16-bit quantized).
        let mut reader = hound::WavReader::open(&wav_path).unwrap();
        assert_eq!(reader.spec().sample_rate, TARGET_SAMPLE_RATE);
        assert_eq!(reader.spec().channels, 1);
        let wav_samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32767.0)
            .collect();
        assert_eq!(wav_samples.len(), result.samples.len());
        let max_diff = wav_samples
            .iter()
            .zip(&result.samples)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(max_diff < 2.0 / 32767.0, "max diff {max_diff}");
        std::fs::remove_file(&wav_path).ok();
    }

    #[test]
    fn sink_passthrough_at_target_rate_is_exact() {
        let wav_path = temp_wav("16k-mono");
        let mut sink =
            CaptureSink::new(TARGET_SAMPLE_RATE, 1, &wav_path, Box::new(|_| {})).unwrap();
        let frames = 12_345;
        let input: Vec<f32> = (0..frames)
            .map(|i| (i % 100) as f32 / 100.0 - 0.5)
            .collect();
        sink.push_interleaved(&input).unwrap();
        let result = sink.finalize(wav_path.clone()).unwrap();
        assert_eq!(result.samples.len(), frames);
        assert_eq!(result.samples, input);
        std::fs::remove_file(&wav_path).ok();
    }

    /// The mid-stream interruption path: whatever was pushed before the
    /// stream died must survive in both the result and the WAV.
    #[test]
    fn interrupted_capture_keeps_partial_audio() {
        let wav_path = temp_wav("interrupted");
        let mut sink = CaptureSink::new(44_100, 2, &wav_path, Box::new(|_| {})).unwrap();
        sink.push_interleaved(&stereo_sine(44_100, 0.5)).unwrap();
        sink.interrupted = true; // what the cpal error callback effects
        let result = sink.finalize(wav_path.clone()).unwrap();
        assert!(result.interrupted);
        let expected = 0.5 * TARGET_SAMPLE_RATE as f64;
        assert!((result.samples.len() as f64 - expected).abs() <= 1.0);
        let reader = hound::WavReader::open(&wav_path).unwrap();
        assert_eq!(reader.len() as usize, result.samples.len());
        std::fs::remove_file(&wav_path).ok();
    }

    #[test]
    fn zero_channel_sink_is_rejected() {
        // CaptureSink isn't Debug (boxed callback), so unwrap_err can't be
        // used here.
        let Err(err) = CaptureSink::new(48_000, 0, &temp_wav("zero-chan"), Box::new(|_| {})) else {
            panic!("expected Capture error");
        };
        assert!(matches!(err, CoreError::Capture(_)), "got {err:?}");
    }

    #[test]
    fn unwritable_wav_path_is_reported() {
        let Err(err) = CaptureSink::new(
            48_000,
            1,
            Path::new("/nonexistent-dir/never.wav"),
            Box::new(|_| {}),
        ) else {
            panic!("expected Capture error");
        };
        assert!(matches!(err, CoreError::Capture(_)), "got {err:?}");
    }

    /// CI runners have no audio stack; this only asserts the call returns
    /// instead of panicking, on any host.
    #[test]
    fn list_input_devices_does_not_panic() {
        let _ = list_input_devices();
    }

    #[test]
    #[ignore = "requires a real microphone and input permission"]
    fn records_half_a_second_from_default_device() {
        let wav_path = temp_wav("live");
        let session = start_recording(None, &wav_path, |_| {}).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let result = session.stop().unwrap();
        assert!(!result.interrupted);
        assert!(result.duration_secs > 0.3, "got {}", result.duration_secs);
        assert!(wav_path.is_file());
        std::fs::remove_file(&wav_path).ok();
    }
}
