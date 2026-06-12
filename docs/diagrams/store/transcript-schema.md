---
title: "On-disk transcript schema — JSON per transcript + sibling WAV"
models: "crates/core/src/store"
source_of_truth: code
last_verified: b1bd80d 2026-06-12
diagram_type: erDiagram
---

# Transcript store schema

One JSON file per transcript plus, for recordings, a sibling WAV under
`audio/`. The layout mirrors the schema proven in the original Swift app,
including its forward-compat lesson: **unknown/missing optional fields are
tolerated on decode** so older files keep loading as the schema grows.

```mermaid
erDiagram
    TRANSCRIPT {
        u32 schemaVersion "1"
        uuid id PK
        string title
        datetime createdAt "RFC 3339"
        datetime updatedAt "RFC 3339"
        string sourceFilename "import filename or null for recordings"
        f64 duration "seconds"
        string modelId "catalog id used"
        string language "ISO 639-1 or auto-detected"
        f64 transcriptionDurationSecs "wall-clock inference time"
        string audioRelativePath "optional — sibling WAV"
    }

    SEGMENT {
        u32 id PK "ordinal within transcript"
        f64 start "seconds"
        f64 end "seconds"
        string text "current (possibly edited) text"
        string originalText "optional — engine output before user edit"
    }

    SETTINGS {
        string defaultModelId "platform-aware default"
        string language "auto by default"
        string inputDeviceId "optional"
        string storageDir "optional override"
        bool fxEnabled "scan-lines + glow toggle"
    }

    TRANSCRIPT ||--|{ SEGMENT : "segments[]"
```

On-disk layout (inside the app-data dir, or the user's `storageDir` override):

```text
transcripts/<uuid>.json        one TRANSCRIPT document, segments embedded
audio/<uuid>.wav               16-bit PCM archive of recordings only
models/ggml-<id>.bin           verified whisper models
settings.json                  one SETTINGS document
```

Editing rule: the first user edit to a segment copies `text` →
`originalText`, then edits land in `text`. `originalText` is never
overwritten after that — the engine's words stay recoverable.
