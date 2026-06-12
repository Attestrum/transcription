---
title: "Export formats — transcript to TXT / SRT / VTT / JSON"
models: "crates/core/src/export"
source_of_truth: code
last_verified: 4afe42d 2026-06-12
diagram_type: flowchart
---

# Export formats

One transcript document, four deterministic renderings — two exports of the
same transcript are byte-identical (golden-file tested under
`crates/core/tests/golden/export/`). Formats mirror the original Swift app's
proven output, minus speaker support (cut from v1).

```mermaid
flowchart LR
    T(["Transcript<br/>(store document)"])

    subgraph export["export/ — export(transcript, format)"]
        TXT["TXT<br/>segment texts joined by spaces,<br/>trailing newline"]
        SRT["SRT<br/>1-based index ·<br/>HH:MM:SS,mmm --&gt; HH:MM:SS,mmm<br/>(comma milliseconds)"]
        VTT["VTT<br/>WEBVTT header ·<br/>HH:MM:SS.mmm --&gt; HH:MM:SS.mmm<br/>(dot milliseconds)"]
        JSON["JSON — OpenAI-whisper shape<br/>created_at · duration · language ·<br/>model · segments[] · source · text<br/>(fields alphabetical = serde order)"]
    end

    T --> TXT
    T --> SRT
    T --> VTT
    T --> JSON

    JSON -.- NOTE["segments[].speaker pre-declared null:<br/>adding diarization later is non-breaking.<br/>source = 'mic' when a sibling WAV exists,<br/>else 'file'"]
```

Rules the goldens pin:

- Timestamps are `round(seconds × 1000)` milliseconds; SRT and VTT differ
  ONLY in the millisecond separator (`,` vs `.`). Hours are always present.
- Segment text is trimmed in every format; empty segments are dropped from
  the joined `text` fields but keep their SRT/VTT cues.
- Intentional format changes regenerate goldens via `UPDATE_GOLDEN=1` — a
  golden diff in review IS the format-change review.
