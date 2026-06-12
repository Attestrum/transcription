# Changelog

All notable user-facing changes to Attestrum Transcription. Release-oriented;
for the full record see the git log.

## [Unreleased]

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
