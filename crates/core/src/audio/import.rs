//! Media-file import: symphonia demux/decode → mono downmix → rubato
//! resample, producing the 16 kHz mono f32 PCM the whisper engine consumes.
//!
//! Implements the file-import path of
//! `docs/diagrams/pipeline/transcription-flow.md`. Blocking; the caller owns
//! the thread (same contract as `engine::WhisperEngine::transcribe`).

use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Indexing, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
use symphonia::core::codecs::audio::{AudioCodecParameters, AudioDecoderOptions};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::error::CoreError;

/// The PCM contract shared with the whisper engine.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Resampler input chunk size in frames.
const RESAMPLE_CHUNK: usize = 1024;

/// 16 kHz mono f32 PCM decoded from a media file.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedAudio {
    pub samples: Vec<f32>,
    pub duration_secs: f64,
}

/// Decode any supported container (wav / mp3 / m4a / mp4 / mov / ogg / flac /
/// mkv) to 16 kHz mono f32 PCM, blocking until done or cancelled.
///
/// `on_progress` receives 0.0–1.0 when the container declares its length;
/// containers without a frame count produce no progress callbacks.
pub fn import_file(
    path: &Path,
    cancel: Arc<AtomicBool>,
    mut on_progress: impl FnMut(f32),
) -> Result<ImportedAudio, CoreError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    // Any probe failure means "we cannot read this file as media" — the
    // distinction between unknown container and corrupt header is not
    // actionable for the user.
    let mut reader = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|_| CoreError::UnsupportedMedia(path.to_owned()))?;

    let track = reader
        .default_track(TrackType::Audio)
        .ok_or_else(|| CoreError::UnsupportedMedia(path.to_owned()))?;
    let track_id = track.id;
    let total_frames = track.num_frames;
    let codec_params: AudioCodecParameters = match &track.codec_params {
        Some(CodecParameters::Audio(p)) => p.clone(),
        _ => return Err(CoreError::UnsupportedMedia(path.to_owned())),
    };

    // `verify: true` makes decoders check per-frame checksums where the
    // codec carries them, so corruption surfaces as a decode error instead
    // of silently importing garbage audio.
    let decoder_opts = AudioDecoderOptions::default().verify(true);
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&codec_params, &decoder_opts)
        .map_err(|_| CoreError::UnsupportedMedia(path.to_owned()))?;

    // Decode state. The source spec is taken from the first decoded buffer
    // (codec params may omit it); the resampler is built lazily from it.
    let mut source_rate: Option<u32> = None;
    let mut channels = 0usize;
    let mut interleaved: Vec<f32> = Vec::new();
    let mut pending: Vec<f32> = Vec::new(); // mono, source rate
    let mut resampler: Option<MonoResampler> = None;
    let mut out: Vec<f32> = Vec::new(); // mono, 16 kHz
    let mut frames_in: u64 = 0;
    let mut last_progress = -1.0f32;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled);
        }
        let packet = match reader.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            // A truncated file ends the stream; whatever decoded so far is
            // the import (matches the "partial save" stance of the diagram).
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(e) => return Err(CoreError::Decode(e.to_string())),
        };
        if packet.track_id != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            // Per symphonia's contract a DecodeError is recoverable — skip
            // the corrupt packet and continue with the next one.
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(CoreError::Decode(e.to_string())),
        };
        if decoded.is_empty() {
            continue;
        }

        let spec = decoded.spec();
        match source_rate {
            None => {
                source_rate = Some(spec.rate());
                channels = spec.channels().count();
                if channels == 0 {
                    return Err(CoreError::UnsupportedMedia(path.to_owned()));
                }
                if spec.rate() != TARGET_SAMPLE_RATE {
                    resampler = Some(MonoResampler::new(spec.rate())?);
                }
            }
            Some(rate) if rate != spec.rate() || channels != spec.channels().count() => {
                return Err(CoreError::Decode(
                    "sample rate or channel layout changed mid-stream".into(),
                ));
            }
            Some(_) => {}
        }

        decoded.copy_to_vec_interleaved::<f32>(&mut interleaved);
        for frame in interleaved.chunks_exact(channels) {
            pending.push(frame.iter().sum::<f32>() / channels as f32);
        }
        frames_in += decoded.frames() as u64;

        match &mut resampler {
            Some(r) => r.drain_full_chunks(&mut pending, &mut out)?,
            None => out.append(&mut pending),
        }

        if let Some(total) = total_frames {
            if total > 0 {
                let p = (frames_in as f32 / total as f32).clamp(0.0, 1.0);
                if p - last_progress >= 0.01 {
                    last_progress = p;
                    on_progress(p);
                }
            }
        }
    }

    // Nothing decoded — whether the file was truly empty or every frame was
    // skipped as corrupt, the user-facing outcome is the same: no audio.
    if frames_in == 0 {
        return Err(CoreError::EmptyAudio(path.to_owned()));
    }

    if let Some(mut r) = resampler {
        r.finish(&mut pending, frames_in, &mut out)?;
    }

    if last_progress >= 0.0 {
        on_progress(1.0);
    }
    let duration_secs = out.len() as f64 / TARGET_SAMPLE_RATE as f64;
    Ok(ImportedAudio {
        samples: out,
        duration_secs,
    })
}

/// Mono sinc resampler from a source rate to [`TARGET_SAMPLE_RATE`], wrapping
/// rubato's fixed-input-size `Async` with chunk buffering, delay trimming,
/// and end-of-stream flush.
struct MonoResampler {
    inner: Async<f32>,
    ratio: f64,
    /// Frames of resampler group delay not yet trimmed from the output.
    delay_to_trim: usize,
    scratch: Vec<f32>,
}

impl MonoResampler {
    fn new(source_rate: u32) -> Result<Self, CoreError> {
        let ratio = TARGET_SAMPLE_RATE as f64 / source_rate as f64;
        let params = SincInterpolationParameters {
            sinc_len: 128,
            f_cutoff: rubato::calculate_cutoff(128, WindowFunction::Blackman2),
            oversampling_factor: 256,
            interpolation: SincInterpolationType::Linear,
            window: WindowFunction::Blackman2,
        };
        let inner =
            Async::<f32>::new_sinc(ratio, 1.1, &params, RESAMPLE_CHUNK, 1, FixedAsync::Input)
                .map_err(|e| CoreError::Decode(format!("resampler construction: {e}")))?;
        let delay_to_trim = inner.output_delay();
        let scratch = vec![0.0; inner.output_frames_max()];
        Ok(Self {
            inner,
            ratio,
            delay_to_trim,
            scratch,
        })
    }

    /// Resample as many full input chunks as `pending` holds, draining them.
    fn drain_full_chunks(
        &mut self,
        pending: &mut Vec<f32>,
        out: &mut Vec<f32>,
    ) -> Result<(), CoreError> {
        let mut consumed = 0;
        while pending.len() - consumed >= RESAMPLE_CHUNK {
            let chunk = &pending[consumed..consumed + RESAMPLE_CHUNK];
            self.run(chunk, None, out)?;
            consumed += RESAMPLE_CHUNK;
        }
        pending.drain(..consumed);
        Ok(())
    }

    /// Resample the final partial chunk, then flush silence through the
    /// resampler until the full expected output length is available, leaving
    /// `out` trimmed to exactly `round(total_input_frames * ratio)`.
    fn finish(
        &mut self,
        pending: &mut Vec<f32>,
        total_frames_in: u64,
        out: &mut Vec<f32>,
    ) -> Result<(), CoreError> {
        let expected = (total_frames_in as f64 * self.ratio).round() as usize;
        if !pending.is_empty() {
            let last = std::mem::take(pending);
            self.run(&last, Some(last.len()), out)?;
        }
        while out.len() < expected {
            self.run(&[], Some(0), out)?;
        }
        out.truncate(expected);
        Ok(())
    }

    /// One `process_into_buffer` call; appends produced frames to `out`,
    /// trimming the initial group delay.
    fn run(
        &mut self,
        input: &[f32],
        partial_len: Option<usize>,
        out: &mut Vec<f32>,
    ) -> Result<(), CoreError> {
        // The adapter requires `chunk_size` readable frames even for partial
        // input; pad with silence.
        let padded;
        let input = if input.len() < RESAMPLE_CHUNK {
            padded = {
                let mut v = input.to_vec();
                v.resize(RESAMPLE_CHUNK, 0.0);
                v
            };
            &padded[..]
        } else {
            input
        };
        let in_adapter = InterleavedSlice::new(input, 1, RESAMPLE_CHUNK)
            .map_err(|e| CoreError::Decode(format!("resampler input: {e}")))?;
        let scratch_frames = self.scratch.len();
        let mut out_adapter = InterleavedSlice::new_mut(&mut self.scratch, 1, scratch_frames)
            .map_err(|e| CoreError::Decode(format!("resampler output: {e}")))?;
        let indexing = Indexing {
            input_offset: 0,
            output_offset: 0,
            partial_len,
            active_channels_mask: None,
        };
        let (_, written) = self
            .inner
            .process_into_buffer(&in_adapter, &mut out_adapter, Some(&indexing))
            .map_err(|e| CoreError::Decode(format!("resample: {e}")))?;
        let produced = &self.scratch[..written];
        let skip = self.delay_to_trim.min(produced.len());
        self.delay_to_trim -= skip;
        out.extend_from_slice(&produced[skip..]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/import")
            .join(name)
    }

    fn import(name: &str) -> ImportedAudio {
        import_file(&fixture(name), Arc::new(AtomicBool::new(false)), |_| {}).unwrap()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    /// Sign changes among samples above a small noise floor — a 440 Hz tone
    /// at any sample rate has ~880 per second.
    fn zero_crossings(samples: &[f32]) -> usize {
        let mut last = 0.0f32;
        let mut crossings = 0;
        for &s in samples.iter().filter(|s| s.abs() > 0.01) {
            if last != 0.0 && s.signum() != last.signum() {
                crossings += 1;
            }
            last = s;
        }
        crossings
    }

    /// The 2 s 440 Hz stereo 44.1 kHz tone, decoded and resampled, must come
    /// out as ~2 s of 16 kHz mono with the tone intact. Lossy encoders add
    /// priming/padding, hence the tolerances.
    fn assert_tone(audio: &ImportedAudio) {
        let n = audio.samples.len() as f64;
        let expected = 2.0 * TARGET_SAMPLE_RATE as f64;
        assert!(
            (n - expected).abs() / expected < 0.10,
            "length {} vs expected {}",
            n,
            expected
        );
        assert!(
            (audio.duration_secs - 2.0).abs() < 0.2,
            "duration {}",
            audio.duration_secs
        );
        let level = rms(&audio.samples);
        assert!(level > 0.15 && level < 0.5, "rms {level}");
        let per_sec = zero_crossings(&audio.samples) as f64 / audio.duration_secs;
        assert!(
            (per_sec - 880.0).abs() / 880.0 < 0.08,
            "zero crossings/sec {per_sec}"
        );
    }

    #[test]
    fn imports_wav() {
        assert_tone(&import("tone.wav"));
    }

    #[test]
    fn imports_mp3() {
        assert_tone(&import("tone.mp3"));
    }

    #[test]
    fn imports_m4a() {
        assert_tone(&import("tone.m4a"));
    }

    #[test]
    fn imports_mp4() {
        assert_tone(&import("tone.mp4"));
    }

    #[test]
    fn imports_mov() {
        assert_tone(&import("tone.mov"));
    }

    #[test]
    fn imports_ogg() {
        assert_tone(&import("tone.ogg"));
    }

    #[test]
    fn imports_flac() {
        assert_tone(&import("tone.flac"));
    }

    #[test]
    fn imports_mkv() {
        assert_tone(&import("tone.mkv"));
    }

    /// jfk.wav is already 16 kHz mono — the passthrough path must preserve
    /// it sample-for-sample.
    #[test]
    fn passthrough_is_lossless() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jfk.wav");
        let mut reader = hound::WavReader::open(&path).unwrap();
        let expected: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect();
        let audio = import_file(&path, Arc::new(AtomicBool::new(false)), |_| {}).unwrap();
        assert_eq!(audio.samples.len(), expected.len());
        let max_diff = audio
            .samples
            .iter()
            .zip(&expected)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(max_diff < 1e-4, "max sample diff {max_diff}");
    }

    #[test]
    fn progress_is_monotonic_and_reaches_one() {
        let mut seen: Vec<f32> = Vec::new();
        import_file(
            &fixture("tone.wav"),
            Arc::new(AtomicBool::new(false)),
            |p| seen.push(p),
        )
        .unwrap();
        assert!(
            !seen.is_empty(),
            "wav declares its length; expected progress"
        );
        assert!(
            seen.windows(2).all(|w| w[0] <= w[1]),
            "not monotonic: {seen:?}"
        );
        assert_eq!(*seen.last().unwrap(), 1.0);
    }

    #[test]
    fn garbage_file_is_unsupported() {
        let path = std::env::temp_dir().join("attestrum-import-garbage.mp3");
        std::fs::write(&path, b"this is not media content at all............").unwrap();
        let err = import_file(&path, Arc::new(AtomicBool::new(false)), |_| {}).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, CoreError::UnsupportedMedia(_)), "got {err:?}");
    }

    #[test]
    fn missing_file_is_io_error() {
        let err = import_file(
            Path::new("/nonexistent/never.wav"),
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::Io(_)), "got {err:?}");
    }

    /// Valid FLAC header, every frame body corrupted: the probe succeeds but
    /// nothing decodes — reported as zero audio rather than garbage samples.
    #[test]
    fn corrupt_stream_yields_no_audio() {
        let err = import_file(
            &fixture("corrupt.flac"),
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::EmptyAudio(_)), "got {err:?}");
    }

    /// Two complete OGG streams concatenated ("chained" OGG, legal per the
    /// spec but unsupported in v1) — the reader signals a reset, which must
    /// surface as a typed decode failure, not a silent half-import.
    #[test]
    fn chained_ogg_is_decode_error() {
        let err = import_file(
            &fixture("chained.ogg"),
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::Decode(_)), "got {err:?}");
    }

    #[test]
    fn zero_frame_wav_is_empty_audio() {
        let path = std::env::temp_dir().join("attestrum-import-empty.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        hound::WavWriter::create(&path, spec)
            .unwrap()
            .finalize()
            .unwrap();
        let err = import_file(&path, Arc::new(AtomicBool::new(false)), |_| {}).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(
            matches!(
                err,
                CoreError::EmptyAudio(_) | CoreError::UnsupportedMedia(_)
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn pre_cancelled_import_returns_cancelled() {
        let err = import_file(
            &fixture("tone.wav"),
            Arc::new(AtomicBool::new(true)),
            |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::Cancelled), "got {err:?}");
    }
}
