---
title: "Playback lifecycle — rodio player on a dedicated thread"
models: "crates/core/src/audio/playback.rs"
source_of_truth: code
last_verified: 064036e 2026-06-12
diagram_type: stateDiagram-v2
---

# Player state

Playback is Rust-side (rodio), never HTML5 audio — webview codec support is
inconsistent across WKWebView/WebView2, and rodio plays exactly the archive
WAV we wrote. The rodio output stream is not `Send`, so a dedicated thread
owns it (same pattern as mic capture); commands arrive over a channel, and a
~10 Hz `playback:position` event streams back while playing (plus one event
on every pause / seek / load so the UI never shows a stale position).

Only transcripts with a sibling archive WAV (`audioRelativePath`) are
playable — imports don't store audio in v1. Loading anything else fails
typed; loading while something is already loaded replaces it.

```mermaid
stateDiagram-v2
    [*] --> Unloaded

    Unloaded --> Paused : player_load(id)<br/>(open audio/<id>.wav, pos = 0)
    Unloaded --> Unloaded : load fails<br/>(no WAV / decode error → typed AppError)

    Paused --> Playing : player_play
    Playing --> Paused : player_pause
    Playing --> Paused : source ends<br/>(pos = duration, final event)

    Paused --> Paused : player_seek(secs)<br/>(clamped to [0, duration])
    Playing --> Playing : player_seek(secs)

    Paused --> Paused : player_load(other id)<br/>(replace source, pos = 0)
    Playing --> Paused : player_load(other id)

    note right of Playing
        emits playback:position
        {secs, playing} at ~10 Hz
    end note
```

The waveform scrubber uses `player_peaks(id)` — N max-amplitude buckets
precomputed from the archive WAV (pure function, no playback state).
