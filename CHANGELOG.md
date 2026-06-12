# Changelog

All notable user-facing changes to Attestrum Transcription. Release-oriented;
for the full record see the git log.

## [Unreleased]

- Design: replaced the terminal/CRT aesthetic with a faithful port of the
  original Austin's Transcriptor native-macOS design — system fonts, flat
  dark surfaces with hairline borders, system-blue accent, overlay traffic
  lights, sidebar with Today/Yesterday/Earlier groups and inline delete,
  dashed drop-zone home, capsule metadata chips, slider player bar, and
  modal record/download sheets. Recording can now be cancelled (audio
  discarded) as well as stopped-and-transcribed.
- Record + transcribe: ● REC opens record mode (live mirrored-bar waveform,
  big timer) and stop flows straight into transcription with segments
  typing in as whisper emits them; + IMPORT and window-wide drag-drop
  transcribe media files. A missing model opens the download sheet —
  terminal-style progress with live speed and an explicit SHA-256
  VERIFIED ✓ before anything runs — then the job starts automatically.
- Editor + playback: recordings play back through the Rust side (rodio over
  the archive WAV — never the webview's audio stack), with click-to-seek
  timestamps, a playing-segment highlight, a waveform-strip scrubber built
  from precomputed peaks, inline segment editing that preserves the engine's
  original words, inline title rename, and find-in-transcript.
- CI: diagram gate — frontmatter completeness, `last_verified` validity and
  freshness, code-diagram path references, drift (code named in a diagram's
  `models:` changing without the diagram), and a Mermaid parse of every
  diagram. Diagram-vs-code drift now fails the build.
- UI shell: top bar with the terminal status line (IDLE / REC / TRANSCRIBING
  + blinking cursor), prompt-styled library pane with search and ASCII empty
  states, editor pane with green clickable timestamps, status bar with the
  FX toggle (scan-lines + glow; honors prefers-reduced-motion).
- IPC layer: the full command surface (models, recording, transcription
  jobs with streaming segment/progress events, library CRUD, export,
  settings) plus a typed TypeScript client (`src/lib/api/`). Transcription
  runs on dedicated threads, segments coalesce ≤ 50 ms, and every command
  returns a typed error the UI can narrow on.
- Transcript store: one JSON document per transcript (camelCase schema v1,
  tolerant decode for forward compatibility), atomic writes, sibling-WAV
  lifecycle, settings with safe defaults, and the edit rule that preserves
  the engine's original words on first edit.
- Exporters: TXT, SRT, VTT, and OpenAI-whisper-shape JSON, byte-stable and
  golden-file tested, matching the original app's proven formatting.
- Microphone capture: device picker enumeration, any input rate/layout
  downmixed and resampled to 16 kHz mono, live level meter (~30 Hz), and an
  incrementally-flushed archive WAV so an interrupted recording keeps
  everything captured so far.
- Audio import: wav / mp3 / m4a / mp4 / mov / ogg / flac / mkv decode to the
  whisper PCM contract (16 kHz mono f32) via symphonia and rubato, with
  progress, cancellation, per-frame checksum verification, and typed errors
  for unsupported, corrupt, and empty media.
- Whisper engine: streaming segment callbacks, progress reporting, and
  cooperative cancellation over whisper.cpp (Metal on macOS, CPU on Windows),
  with real-model integration tests run in CI on both platforms.
- macOS builds now require macOS 11+ (Apple Silicon baseline; whisper.cpp
  uses `std::filesystem`, unavailable below 10.15).
- Project scaffold: Tauri 2 + Svelte 5 workspace, `crates/core` engine split,
  Attestrum design tokens, dual Apache-2.0/MIT license.
- CI: fmt/clippy/test gates, frontend check/build, macOS/Windows build smoke.
- Release pipeline: tag-triggered macOS (Apple Silicon) dmg + Windows x64 NSIS
  builds, every binary Sigstore-signed in public CI by the org's GitHub
  Actions identity, with an in-workflow identity assertion and `SHA256SUMS`.
- Design contract: Mermaid diagram set under `docs/diagrams/` (architecture,
  transcription pipeline, model lifecycle, IPC contract, transcript schema,
  release pipeline).
