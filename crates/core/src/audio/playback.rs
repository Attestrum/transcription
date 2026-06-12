//! Archive-WAV playback on a dedicated thread.
//!
//! Implements `docs/diagrams/player/player-state.md`: rodio's output stream
//! is not `Send`, so a thread owns it (same pattern as mic capture) and
//! commands arrive over a channel. Positions stream back through a callback
//! at ~10 Hz while playing, plus once after every load / play / pause /
//! seek so the UI never shows a stale position.
//!
//! The audio device opens lazily on the first `load`, so constructing the
//! player on a box with no output device only fails when playback is
//! actually attempted.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::error::CoreError;

/// One position sample.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPosition {
    pub secs: f64,
    pub playing: bool,
}

enum Cmd {
    Load {
        path: PathBuf,
        duration_secs: f64,
        reply: mpsc::Sender<Result<(), CoreError>>,
    },
    Play {
        reply: mpsc::Sender<Result<(), CoreError>>,
    },
    Pause {
        reply: mpsc::Sender<Result<(), CoreError>>,
    },
    Seek {
        secs: f64,
        reply: mpsc::Sender<Result<(), CoreError>>,
    },
    Shutdown,
}

/// Handle to the player thread. Dropping it shuts the thread down.
pub struct PlayerHandle {
    tx: mpsc::Sender<Cmd>,
    thread: Option<JoinHandle<()>>,
}

impl PlayerHandle {
    /// Spawn the player thread. `on_position` fires from that thread.
    pub fn spawn(
        on_position: impl Fn(PlaybackPosition) + Send + 'static,
    ) -> Result<Self, CoreError> {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let thread = std::thread::Builder::new()
            .name("playback".into())
            .spawn(move || player_thread(rx, on_position))
            .map_err(|e| CoreError::Playback(format!("spawn player thread: {e}")))?;
        Ok(Self {
            tx,
            thread: Some(thread),
        })
    }

    fn request(
        &self,
        make: impl FnOnce(mpsc::Sender<Result<(), CoreError>>) -> Cmd,
    ) -> Result<(), CoreError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(make(reply_tx))
            .map_err(|_| CoreError::Playback("player thread is gone".into()))?;
        reply_rx
            .recv()
            .map_err(|_| CoreError::Playback("player thread died".into()))?
    }

    /// Load a WAV for playback, replacing whatever is loaded; starts paused
    /// at position 0. `duration_secs` comes from the transcript document and
    /// bounds seeks.
    pub fn load(&self, path: &Path, duration_secs: f64) -> Result<(), CoreError> {
        self.request(|reply| Cmd::Load {
            path: path.to_owned(),
            duration_secs,
            reply,
        })
    }

    pub fn play(&self) -> Result<(), CoreError> {
        self.request(|reply| Cmd::Play { reply })
    }

    pub fn pause(&self) -> Result<(), CoreError> {
        self.request(|reply| Cmd::Pause { reply })
    }

    pub fn seek(&self, secs: f64) -> Result<(), CoreError> {
        self.request(|reply| Cmd::Seek { secs, reply })
    }
}

impl Drop for PlayerHandle {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Everything the thread knows about the current source.
struct Loaded {
    path: PathBuf,
    duration_secs: f64,
    /// Set once the source ran to its end (rodio's queue is then empty;
    /// play/seek must re-append the decoder).
    ended: bool,
}

fn player_thread(rx: mpsc::Receiver<Cmd>, on_position: impl Fn(PlaybackPosition)) {
    // Both open lazily on first Load.
    let mut device: Option<rodio::stream::MixerDeviceSink> = None;
    let mut player: Option<rodio::Player> = None;
    let mut loaded: Option<Loaded> = None;

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Cmd::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => return,

            Ok(Cmd::Load {
                path,
                duration_secs,
                reply,
            }) => {
                let result = (|| -> Result<(), CoreError> {
                    if device.is_none() {
                        let sink = rodio::stream::DeviceSinkBuilder::open_default_sink()
                            .map_err(|e| CoreError::Playback(format!("audio output: {e}")))?;
                        player = Some(rodio::Player::connect_new(sink.mixer()));
                        device = Some(sink);
                    }
                    let p = player.as_ref().expect("player exists when device does");
                    p.clear(); // also pauses
                    append_decoder(p, &path)?;
                    loaded = Some(Loaded {
                        path,
                        duration_secs,
                        ended: false,
                    });
                    Ok(())
                })();
                let ok = result.is_ok();
                let _ = reply.send(result);
                if ok {
                    on_position(PlaybackPosition {
                        secs: 0.0,
                        playing: false,
                    });
                }
            }

            Ok(Cmd::Play { reply }) => {
                let result = (|| -> Result<(), CoreError> {
                    let (p, l) = current(&player, &mut loaded)?;
                    if l.ended {
                        // The queue drained; restart from the top.
                        append_decoder(p, &l.path)?;
                        l.ended = false;
                    }
                    p.play();
                    Ok(())
                })();
                let ok = result.is_ok();
                let _ = reply.send(result);
                if ok {
                    emit(&player, &loaded, &on_position);
                }
            }

            Ok(Cmd::Pause { reply }) => {
                let result = current(&player, &mut loaded).map(|(p, _)| p.pause());
                let ok = result.is_ok();
                let _ = reply.send(result);
                if ok {
                    emit(&player, &loaded, &on_position);
                }
            }

            Ok(Cmd::Seek { secs, reply }) => {
                let result = (|| -> Result<(), CoreError> {
                    let (p, l) = current(&player, &mut loaded)?;
                    let was_playing = !p.is_paused() && !l.ended;
                    if l.ended {
                        // Re-arm the drained queue, paused, then seek.
                        p.pause();
                        append_decoder(p, &l.path)?;
                        l.ended = false;
                    }
                    let target = secs.clamp(0.0, l.duration_secs);
                    p.try_seek(Duration::from_secs_f64(target))
                        .map_err(|e| CoreError::Playback(format!("seek: {e}")))?;
                    if was_playing {
                        p.play();
                    }
                    Ok(())
                })();
                let ok = result.is_ok();
                let _ = reply.send(result);
                if ok {
                    emit(&player, &loaded, &on_position);
                }
            }

            // Tick: stream positions while playing; emit the final position
            // exactly once when the source runs out.
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let (Some(p), Some(l)) = (player.as_ref(), loaded.as_mut()) else {
                    continue;
                };
                if l.ended {
                    continue;
                }
                if p.empty() {
                    l.ended = true;
                    on_position(PlaybackPosition {
                        secs: l.duration_secs,
                        playing: false,
                    });
                } else if !p.is_paused() {
                    on_position(PlaybackPosition {
                        secs: p.get_pos().as_secs_f64(),
                        playing: true,
                    });
                }
            }
        }
    }
}

fn append_decoder(player: &rodio::Player, path: &Path) -> Result<(), CoreError> {
    let file = File::open(path)?;
    let decoder = rodio::Decoder::try_from(file)
        .map_err(|e| CoreError::Playback(format!("decode {}: {e}", path.display())))?;
    player.append(decoder);
    Ok(())
}

fn current<'a>(
    player: &'a Option<rodio::Player>,
    loaded: &'a mut Option<Loaded>,
) -> Result<(&'a rodio::Player, &'a mut Loaded), CoreError> {
    match (player.as_ref(), loaded.as_mut()) {
        (Some(p), Some(l)) => Ok((p, l)),
        _ => Err(CoreError::Playback("nothing loaded".into())),
    }
}

fn emit(
    player: &Option<rodio::Player>,
    loaded: &Option<Loaded>,
    on_position: &impl Fn(PlaybackPosition),
) {
    if let (Some(p), Some(l)) = (player.as_ref(), loaded.as_ref()) {
        let secs = if l.ended {
            l.duration_secs
        } else {
            p.get_pos().as_secs_f64()
        };
        on_position(PlaybackPosition {
            secs,
            playing: !p.is_paused() && !l.ended && !p.empty(),
        });
    }
}

/// Max-amplitude buckets for the waveform scrubber — pure function over the
/// archive WAV, no playback state.
pub fn waveform_peaks(wav_path: &Path, buckets: usize) -> Result<Vec<f32>, CoreError> {
    if buckets == 0 {
        return Ok(Vec::new());
    }
    let mut reader = hound::WavReader::open(wav_path)
        .map_err(|e| CoreError::Playback(format!("open {}: {e}", wav_path.display())))?;
    let total = reader.len() as usize;
    if total == 0 {
        return Ok(vec![0.0; buckets]);
    }
    let mut peaks = vec![0.0f32; buckets];
    for (i, sample) in reader.samples::<i16>().enumerate() {
        let s = sample.map_err(|e| CoreError::Playback(format!("read WAV: {e}")))? as f32 / 32768.0;
        let bucket = (i * buckets / total).min(buckets - 1);
        peaks[bucket] = peaks[bucket].max(s.abs());
    }
    Ok(peaks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_wav(name: &str, samples: &[i16]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("attestrum-playback-{name}.wav"));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for &s in samples {
            w.write_sample(s).unwrap();
        }
        w.finalize().unwrap();
        path
    }

    #[test]
    fn peaks_bucket_maxima_are_normalized_and_positioned() {
        // 4 buckets over 8 samples; the loudest sample of each pair wins.
        let path = write_wav("peaks", &[100, -16384, 32767, 0, -8192, 50, 0, 0]);
        let peaks = waveform_peaks(&path, 4).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(peaks.len(), 4);
        assert!((peaks[0] - 0.5).abs() < 0.01, "got {}", peaks[0]);
        assert!((peaks[1] - 1.0).abs() < 0.01, "got {}", peaks[1]);
        assert!((peaks[2] - 0.25).abs() < 0.01, "got {}", peaks[2]);
        assert_eq!(peaks[3], 0.0);
    }

    #[test]
    fn peaks_with_more_buckets_than_samples_do_not_panic() {
        let path = write_wav("tiny", &[1000, -2000]);
        let peaks = waveform_peaks(&path, 16).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(peaks.len(), 16);
        assert!(peaks.iter().any(|&p| p > 0.0));
    }

    #[test]
    fn peaks_of_missing_file_is_typed() {
        let err = waveform_peaks(Path::new("/nonexistent/never.wav"), 8).unwrap_err();
        assert!(matches!(err, CoreError::Playback(_)), "got {err:?}");
    }

    #[test]
    fn zero_buckets_is_empty() {
        assert_eq!(
            waveform_peaks(Path::new("/irrelevant"), 0).unwrap(),
            Vec::<f32>::new()
        );
    }

    /// No device is touched until load — the handle works headless.
    #[test]
    fn commands_without_load_are_typed_and_shutdown_is_clean() {
        let handle = PlayerHandle::spawn(|_| {}).unwrap();
        let err = handle.play().unwrap_err();
        assert!(matches!(err, CoreError::Playback(_)), "got {err:?}");
        let err = handle.seek(1.0).unwrap_err();
        assert!(matches!(err, CoreError::Playback(_)), "got {err:?}");
        drop(handle); // joins the thread
    }

    #[test]
    #[ignore = "requires a real audio output device"]
    fn loads_plays_seeks_and_reports_positions() {
        use std::sync::{Arc, Mutex};
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jfk.wav");
        let positions: Arc<Mutex<Vec<PlaybackPosition>>> = Arc::default();
        let positions_in_cb = Arc::clone(&positions);
        let handle = PlayerHandle::spawn(move |p| positions_in_cb.lock().unwrap().push(p)).unwrap();

        handle.load(&path, 11.0).unwrap();
        handle.play().unwrap();
        std::thread::sleep(Duration::from_millis(500));
        handle.pause().unwrap();
        handle.seek(5.0).unwrap();

        let seen = positions.lock().unwrap();
        assert!(seen.len() >= 3, "positions: {seen:?}");
        assert!(seen.iter().any(|p| p.playing && p.secs > 0.0));
        let last = seen.last().unwrap();
        assert!(
            !last.playing && (last.secs - 5.0).abs() < 0.2,
            "got {last:?}"
        );
    }
}
