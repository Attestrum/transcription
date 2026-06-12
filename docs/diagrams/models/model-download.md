---
title: "Whisper model lifecycle — download, resume, verify"
models: "crates/core/src/models"
source_of_truth: code
last_verified: b774630 2026-06-12
diagram_type: stateDiagram-v2
---

# Model download lifecycle

Models are lazy-downloaded from `huggingface.co/ggerganov/whisper.cpp` with
SHA-256 pinned in the catalog. The catalog (sizes and digests pinned from the
HF LFS metadata, 2026-06-12):

| id | file | size | platform default |
|---|---|---|---|
| `tiny` | ggml-tiny.bin | 74 MB | — (instant preview) |
| `base` | ggml-base.bin | 141 MB | — |
| `small` | ggml-small.bin | 465 MB | Windows |
| `large-v3-turbo` | ggml-large-v3-turbo.bin | 1.55 GB | macOS Apple Silicon |

```mermaid
stateDiagram-v2
    [*] --> NotDownloaded

    NotDownloaded --> Downloading : download_model(id)<br/>(disk-space pre-check first)
    NotDownloaded --> Failed : insufficient disk space

    state Downloading {
        [*] --> Fetching
        Fetching : reqwest GET → <file>.partial
        Fetching : streaming SHA-256 as bytes arrive
        Fetching : emits model⁚download⁚progress (bytes, total, bps)
        Fetching --> Resuming : connection lost
        Resuming : HTTP Range from .partial length
        Resuming --> Fetching
    }

    Downloading --> Verifying : body complete
    Downloading --> NotDownloaded : cancel_download(id)<br/>(.partial kept for resume)
    Downloading --> Failed : network error after retries

    Verifying : compare streamed SHA-256<br/>against pinned catalog digest
    Verifying --> Ready : match → atomic rename<br/>.partial → ggml-&lt;id&gt;.bin
    Verifying --> Failed : MISMATCH → delete .partial<br/>(never load an unverified model)

    Ready --> NotDownloaded : delete_model(id)
    Failed --> Downloading : retry
    Ready --> [*]
```

Two rules the states encode: a model file only ever exists under its final
name **after** digest verification (atomic rename is the commit point), and a
checksum mismatch destroys the partial — there is no "load anyway" path.
