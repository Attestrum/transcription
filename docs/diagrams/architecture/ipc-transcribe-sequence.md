---
title: "IPC contract — import-and-transcribe round trip"
models: "src-tauri/src/lib.rs, src/lib/api"
source_of_truth: diagram
last_verified: 767220d 2026-06-12
diagram_type: sequenceDiagram
---

# IPC: import → transcribe → edit round trip

The canonical command/event exchange. Commands are request/response
(`Result<T, AppError>` → typed TS errors); everything long-running streams
back as events. Inference never blocks a command handler — `transcribe`
returns a `job_id` immediately and the work happens on a core-owned thread.

```mermaid
sequenceDiagram
    participant UI as Svelte UI
    participant API as lib/api (typed wrappers)
    participant SH as src-tauri shell
    participant CORE as crates/core (worker threads)

    UI->>API: drop file / pick file
    API->>SH: invoke transcribe(source, model_id, language)
    SH->>CORE: spawn job on engine thread
    SH-->>API: job_id

    Note over CORE: decode → resample → whisper full()

    loop as whisper emits (coalesced ≤ 50 ms)
        CORE-)SH: segment {index, start, end, text}
        SH-)API: emit transcribe:segment
        API-)UI: segment materializes (typewriter)
        CORE-)SH: progress {pct}
        SH-)API: emit transcribe:progress
    end

    alt completes
        CORE-)SH: done {transcript_id}
        SH-)API: emit transcribe:done
        API->>SH: invoke get_transcript(transcript_id)
        SH-->>API: Transcript JSON
        API-)UI: open in editor
    else user cancels
        UI->>API: cancel
        API->>SH: invoke cancel_job(job_id)
        SH->>CORE: set AtomicBool
        CORE-)SH: cancelled
        SH-)API: emit transcribe:cancelled
    else engine fails
        CORE-)SH: error {AppError}
        SH-)API: emit transcribe:error
        API-)UI: toast + retry affordance
    end
```

Command surface (full list): `list_models` / `download_model` /
`cancel_download` / `delete_model` · `list_input_devices` / `start_recording`
/ `stop_recording` · `transcribe` / `cancel_job` · `list_transcripts` /
`get_transcript` / `update_transcript` / `rename_transcript` /
`delete_transcript` · `player_load` / `player_play` / `player_pause` /
`player_seek` · `export_transcript` · `get_settings` / `set_settings` ·
`product_info`.

Event surface: `model:download:{progress,done,error}` · `record:level`
(carries `elapsedSecs`) · `transcribe:{segment,progress,done,error,cancelled}`
· `playback:position`.

Implementation status: everything above is implemented in
`src-tauri/src/commands.rs` + `src/lib/api/` EXCEPT the `player_*` commands
and `playback:position`, which land with the rodio playback work (M8). This
file stays `source_of_truth: diagram` until then.
