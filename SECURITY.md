# Security Policy

## Reporting a vulnerability

Email **security@attestrum.com** with the details. Please include:

- A description of the vulnerability and the affected component or version.
- Reproduction steps or a proof-of-concept where possible.
- Your suggested remediation, if any.
- Whether you would like credit in the eventual advisory.

You will receive an acknowledgement within 48 hours. We aim to land a fix in `main` within 14 days of confirmation; issues that require a coordinated upstream fix (whisper.cpp, Tauri) may take longer and we will keep you informed.

Please do **not** disclose publicly until we have published an advisory or the 90-day disclosure window has elapsed, whichever comes first.

## What counts as a security issue

- Any path by which audio, transcripts, or metadata leave the machine. The product promise is that nothing does; any violation is a security issue, not a bug.
- Release-pipeline integrity: anything that lets an artifact ship without the expected Sigstore signature from this repository's GitHub Actions identity, or with a wrong `SHA256SUMS` entry.
- Model-download integrity: checksum-verification bypasses or paths that load an unverified model file.
- Memory-safety issues in the native audio/inference layers.
- Tauri IPC exposure beyond the documented command surface.

Regular bugs that don't touch the above should be filed as GitHub issues, not security reports.
