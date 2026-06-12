# Attestrum Transcription

Private, on-device speech-to-text for macOS and Windows. Open source, with releases you can verify.

Your audio never leaves your machine. Transcription runs locally on [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — no cloud, no account, no telemetry.

> **Status: pre-release.** The first downloadable release (v0.1.0) is under active development. Watch the [releases page](https://github.com/Attestrum/transcription/releases).

## Install

Downloads for macOS (Apple Silicon) and Windows (x64) ship on the [releases page](https://github.com/Attestrum/transcription/releases/latest) and at [attestrum.com](https://attestrum.com).

## Verify your download

Every release binary is signed in public CI by this repository's GitHub Actions identity — not by a person, not on a laptop. Verify with stock [cosign](https://docs.sigstore.dev/cosign/system_config/installation/):

```sh
cosign verify-blob \
  --bundle <asset>.sigstore.json \
  --certificate-identity-regexp '^https://github\.com/Attestrum/transcription/' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  <asset>
```

A `SHA256SUMS` file accompanies every release.

## Build from source

Requirements: [Rust](https://rustup.rs) (pinned via `rust-toolchain.toml`), Node 22+, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

```sh
npm install
npm run tauri dev      # run in development
npm run tauri build    # produce a release bundle
```

The workspace splits into `crates/core` (all engine logic — audio, inference, storage, export — unit-testable without a webview) and `src-tauri` (the thin desktop shell).

## License

Apache-2.0 OR MIT, at your option. Copyright © 2026 Hyper Beam Media LLC.

---

Part of [Attestrum](https://attestrum.com) — tools for provenance you can check yourself.
