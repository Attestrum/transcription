---
title: "Transcription pipeline — record and import paths to stored transcript"
models: "crates/core/src/audio, crates/core/src/engine, crates/core/src/store"
source_of_truth: code
last_verified: b1bd80d 2026-06-12
diagram_type: flowchart
---

# Transcription pipeline

Both input paths converge on one contract: **16 kHz mono f32 PCM** into the
whisper engine. v1 transcribes after recording stops, but segments stream to
the UI as whisper emits them, so the result *feels* live (true while-recording
transcription is v0.2).

All three subgraphs are implemented and this diagram is now a derived view:
`engine/` (`crates/core/src/engine/whisper.rs` — the job queue lives in the
IPC shell's job runner, not the core engine; the engine is a blocking call
the runner's dedicated thread drives), `audio/`
(`crates/core/src/audio/import.rs` file import,
`crates/core/src/audio/capture.rs` mic capture with archive WAV + ~30 Hz
levels), and `store/` (`crates/core/src/store/`, schema in
`docs/diagrams/store/transcript-schema.md`).

```mermaid
flowchart LR
    subgraph inputs["Input paths"]
        MIC["Mic record<br/>(cpal stream,<br/>device-native rate)"]
        FILE["File import<br/>(mp3 / m4a / wav / mp4 /<br/>mov / ogg / flac / mkv)"]
    end

    subgraph audio["audio/"]
        DECODE["symphonia<br/>demux + decode"]
        DOWNMIX["downmix to mono"]
        RESAMPLE["rubato<br/>resample → 16 kHz f32"]
        WAVOUT["hound<br/>archive WAV (recordings)"]
    end

    PCM(["16 kHz mono f32 PCM"])

    subgraph shell["src-tauri job runner (dedicated thread)"]
        JOB["job queue<br/>(one job_id per run)"]
    end

    subgraph engine["engine/"]
        WHISPER["whisper-rs full()<br/>Metal on macOS · CPU on Windows"]
        CB_SEG["new_segment callback"]
        CB_PROG["progress callback"]
        CB_ABORT["abort callback<br/>(checks AtomicBool)"]
    end

    subgraph out["Results"]
        EVENTS["events → UI<br/>transcribe:segment (coalesced ≤ 50 ms)<br/>transcribe:progress / done / error"]
        STORE["store/<br/>transcript JSON + sibling WAV"]
    end

    MIC --> DOWNMIX
    MIC --> WAVOUT
    FILE --> DECODE --> DOWNMIX --> RESAMPLE --> PCM
    PCM --> JOB --> WHISPER
    WHISPER --> CB_SEG --> EVENTS
    WHISPER --> CB_PROG --> EVENTS
    CB_ABORT -.->|"cancel_job(job_id)"| WHISPER
    CB_SEG --> STORE
```

Error paths each surface as a typed `AppError` to the UI and have an
exercising test: unsupported container, decode failure mid-stream, device
disappearing during capture (partial WAV is saved), cancellation (job ends
`cancelled`, partial segments discarded), and model-file-missing (UI routes to
the model sheet).
