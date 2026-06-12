---
title: "Attestrum Transcription — process and module overview"
models: "crates/core, src-tauri/src, src/lib"
source_of_truth: code
last_verified: a5ad884 2026-06-12
diagram_type: flowchart
---

# App overview

One desktop process, two worlds: the **webview frontend** (Svelte 5) and the
**Rust backend**. They talk only through Tauri IPC — typed commands going in,
streamed events coming back. All engine logic lives in `crates/core`, which has
no Tauri dependency and is unit-testable headless; `src-tauri` is a thin shell
that wires core to the IPC surface.

```mermaid
flowchart TB
    subgraph webview["Webview — Svelte 5 + TS (src/)"]
        UI_LIB["Library pane<br/>(list / search / rename / delete)"]
        UI_ED["Editor pane<br/>(segments, click-to-seek, export)"]
        UI_REC["Record / Import surfaces<br/>(waveform, drop zone, model sheet)"]
        API["lib/api — typed invoke() wrappers<br/>+ event listeners"]
        UI_LIB --> API
        UI_ED --> API
        UI_REC --> API
    end

    subgraph shell["src-tauri — IPC shell only"]
        CMD["commands<br/>(Result&lt;T, AppError&gt;)"]
        EVT["event emitter<br/>(progress / segments / levels / position)"]
        STATE["managed AppState<br/>(engine, jobs, player, store)"]
        CMD --> STATE
        STATE --> EVT
    end

    subgraph core["crates/core — engine, no Tauri dep"]
        MODELS["models/<br/>catalog + downloader<br/>(resume, SHA-256, disk check)"]
        ENGINE["engine/<br/>whisper-rs wrapper,<br/>abort via AtomicBool"]
        AUDIO["audio/<br/>cpal capture · symphonia decode ·<br/>rubato resample · hound WAV · rodio playback"]
        STORE["store/<br/>JSON transcript CRUD + settings"]
        EXPORT["export/<br/>TXT · SRT · VTT · JSON"]
    end

    subgraph external["Outside the process"]
        HF["Hugging Face<br/>ggerganov/whisper.cpp<br/>(model downloads ONLY network call)"]
        FS["App-data dir<br/>models/ · transcripts/ · audio/"]
        DEV["Audio input devices"]
    end

    API -- "invoke (commands)" --> CMD
    EVT -- "emit (events)" --> API

    STATE --> MODELS
    STATE --> ENGINE
    STATE --> AUDIO
    STATE --> STORE
    STATE --> EXPORT

    MODELS -- "HTTPS (only here)" --> HF
    MODELS --> FS
    STORE --> FS
    AUDIO --> DEV
    ENGINE -- "16 kHz mono f32" --- AUDIO
```

Privacy invariant: the **only** network call in the entire app is the model
download (cyan path above). Audio, transcripts, and metadata never leave the
machine — `SECURITY.md` treats any violation as a security issue, not a bug.

Every module above is implemented; this diagram is now a derived view of
the code. Transcription job queues live in the shell's `AppState` (`jobs`
map + per-job threads), not in core `engine/`. File pickers come from
`tauri-plugin-dialog`; drag-drop uses the webview's built-in events.
