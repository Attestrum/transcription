//! Shared mono resampler to the whisper PCM rate, used by both the file
//! importer and the mic-capture sink.

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Indexing, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

use crate::error::CoreError;

/// The PCM contract shared with the whisper engine.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Resampler input chunk size in frames.
const RESAMPLE_CHUNK: usize = 1024;

/// Mono sinc resampler from a source rate to [`TARGET_SAMPLE_RATE`], wrapping
/// rubato's fixed-input-size `Async` with chunk buffering, delay trimming,
/// and end-of-stream flush.
pub(crate) struct MonoResampler {
    inner: Async<f32>,
    ratio: f64,
    /// Frames of resampler group delay not yet trimmed from the output.
    delay_to_trim: usize,
    scratch: Vec<f32>,
}

impl MonoResampler {
    pub(crate) fn new(source_rate: u32) -> Result<Self, CoreError> {
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
    pub(crate) fn drain_full_chunks(
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
    pub(crate) fn finish(
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
