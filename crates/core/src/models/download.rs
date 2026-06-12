//! Model downloader.
//!
//! Implements the lifecycle in `docs/diagrams/models/model-download.md`:
//! download to `<file>.partial` with a streaming SHA-256, resume via HTTP
//! Range (re-hashing the existing partial first), verify the digest against
//! the pinned catalog value, and only then atomically rename into place. A
//! mismatch destroys the partial — there is no path that loads an unverified
//! model.
//!
//! Network access: this module performs HTTPS requests (the only network
//! calls in the application). The caller supplies the `reqwest::Client`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::CoreError;
use crate::models::catalog::{self, ModelSpec};

/// Progress snapshot delivered to the caller's callback per received chunk.
/// The IPC shell throttles these before emitting to the UI.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct DownloadProgress {
    pub bytes: u64,
    pub total: u64,
}

/// Installed-ness of a catalog model on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "bytes")]
pub enum ModelState {
    NotDownloaded,
    /// A `.partial` file exists with this many bytes; download can resume.
    Partial(u64),
    Ready,
}

/// Report the on-disk state of a model.
pub fn model_state(models_dir: &Path, spec: &ModelSpec) -> ModelState {
    if models_dir.join(spec.filename).is_file() {
        return ModelState::Ready;
    }
    match std::fs::metadata(partial_path(models_dir, spec)) {
        Ok(meta) => ModelState::Partial(meta.len()),
        Err(_) => ModelState::NotDownloaded,
    }
}

/// Path of the verified model file, if present.
pub fn model_path(models_dir: &Path, spec: &ModelSpec) -> Option<PathBuf> {
    let p = models_dir.join(spec.filename);
    p.is_file().then_some(p)
}

/// Delete a model (and any partial) from disk.
pub fn delete_model(models_dir: &Path, spec: &ModelSpec) -> Result<(), CoreError> {
    for p in [
        models_dir.join(spec.filename),
        partial_path(models_dir, spec),
    ] {
        match std::fs::remove_file(&p) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Download a catalog model into `models_dir`, resuming if a partial exists.
/// Returns the path of the verified model file.
pub async fn download_model(
    client: &reqwest::Client,
    models_dir: &Path,
    model_id: &str,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<PathBuf, CoreError> {
    let spec =
        catalog::spec(model_id).ok_or_else(|| CoreError::UnknownModel(model_id.to_owned()))?;
    download_to(
        client,
        &catalog::url(spec),
        models_dir,
        spec,
        cancel,
        &mut on_progress,
    )
    .await
}

/// URL-explicit worker behind [`download_model`]; tests point it at a local
/// HTTP server instead of Hugging Face.
pub(crate) async fn download_to(
    client: &reqwest::Client,
    url: &str,
    models_dir: &Path,
    spec: &ModelSpec,
    cancel: &AtomicBool,
    on_progress: &mut impl FnMut(DownloadProgress),
) -> Result<PathBuf, CoreError> {
    let final_path = models_dir.join(spec.filename);
    if final_path.is_file() {
        // Present under the final name means it already passed verification.
        return Ok(final_path);
    }
    tokio::fs::create_dir_all(models_dir).await?;
    let partial = partial_path(models_dir, spec);

    // Resume: re-hash whatever the partial already holds so the streaming
    // digest covers the whole file.
    let mut hasher = Sha256::new();
    let mut offset: u64 = 0;
    if let Ok(meta) = tokio::fs::metadata(&partial).await {
        let mut existing = tokio::fs::File::open(&partial).await?;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = existing.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        offset = meta.len();
    }

    ensure_disk_space(models_dir, spec.size_bytes.saturating_sub(offset))?;

    let mut request = client.get(url);
    if offset > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
    }
    let response = request.send().await?;
    let status = response.status();

    let mut file = match status {
        reqwest::StatusCode::PARTIAL_CONTENT if offset > 0 => {
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&partial)
                .await?
        }
        reqwest::StatusCode::OK => {
            // Server ignored (or wasn't sent) the Range header: start over.
            if offset > 0 {
                hasher = Sha256::new();
                offset = 0;
            }
            tokio::fs::File::create(&partial).await?
        }
        _ => {
            return Err(CoreError::HttpStatus {
                status: status.as_u16(),
                url: url.to_owned(),
            });
        }
    };

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            // Keep the partial: a cancel is resumable, only a checksum
            // mismatch destroys data.
            file.flush().await?;
            return Err(CoreError::Cancelled);
        }
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        offset += chunk.len() as u64;
        on_progress(DownloadProgress {
            bytes: offset,
            total: spec.size_bytes,
        });
    }
    file.sync_all().await?;
    drop(file);

    let actual = hex::encode(hasher.finalize());
    if actual != spec.sha256 {
        tokio::fs::remove_file(&partial).await?;
        return Err(CoreError::ChecksumMismatch {
            model_id: spec.id.to_owned(),
            expected: spec.sha256.to_owned(),
            actual,
        });
    }

    // The atomic rename is the commit point: the final name only ever exists
    // after the digest verified.
    tokio::fs::rename(&partial, &final_path).await?;
    Ok(final_path)
}

fn partial_path(models_dir: &Path, spec: &ModelSpec) -> PathBuf {
    models_dir.join(format!("{}.partial", spec.filename))
}

/// Fail early when the filesystem can't hold the remaining bytes (plus a
/// 64 MB safety margin so the download doesn't strand the disk at zero).
fn ensure_disk_space(dir: &Path, needed_bytes: u64) -> Result<(), CoreError> {
    const MARGIN: u64 = 64 * 1024 * 1024;
    let available_bytes = fs4::available_space(dir)?;
    if available_bytes < needed_bytes.saturating_add(MARGIN) {
        return Err(CoreError::InsufficientDisk {
            dir: dir.to_owned(),
            needed_bytes,
            available_bytes,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    /// A model spec whose digest matches `payload()` below.
    fn test_spec() -> ModelSpec {
        ModelSpec {
            id: "test-model",
            display_name: "Test model",
            filename: "ggml-test.bin",
            size_bytes: PAYLOAD_LEN as u64,
            // SHA-256 of payload(), computed in `payload_digest()` and
            // asserted to match in `payload_digest_matches_spec`.
            sha256: "582fe38f0fbe2b24843d936fbe7a586447022089d6167c7ca239959fa80320c0",
        }
    }

    const PAYLOAD_LEN: usize = 256 * 1024 + 13;

    fn payload() -> Vec<u8> {
        (0..PAYLOAD_LEN).map(|i| (i % 251) as u8).collect()
    }

    fn payload_digest() -> String {
        hex::encode(Sha256::digest(payload()))
    }

    #[test]
    fn payload_digest_matches_spec() {
        assert_eq!(payload_digest(), test_spec().sha256);
    }

    /// Minimal HTTP file server with Range support, one request per call.
    fn serve_payload(requests: usize) -> (String, std::thread::JoinHandle<Vec<Option<String>>>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model.bin", server.server_addr());
        let handle = std::thread::spawn(move || {
            let body = payload();
            let mut seen_ranges = Vec::new();
            for _ in 0..requests {
                let request = server.recv().unwrap();
                let range = request
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Range"))
                    .map(|h| h.value.as_str().to_owned());
                let (status, slice) = match range.as_deref() {
                    Some(r) => {
                        let start: usize = r
                            .trim_start_matches("bytes=")
                            .trim_end_matches('-')
                            .parse()
                            .unwrap();
                        (206, &body[start..])
                    }
                    None => (200, &body[..]),
                };
                seen_ranges.push(range);
                let response = tiny_http::Response::from_data(slice.to_vec())
                    .with_status_code(tiny_http::StatusCode(status));
                request.respond(response).unwrap();
            }
            seen_ranges
        });
        (url, handle)
    }

    fn read_file(path: &Path) -> Vec<u8> {
        let mut buf = Vec::new();
        std::fs::File::open(path)
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
        buf
    }

    #[tokio::test]
    async fn fresh_download_verifies_and_renames() {
        let dir = tempdir();
        let (url, server) = serve_payload(1);
        let spec = test_spec();
        let cancel = AtomicBool::new(false);
        let mut last = DownloadProgress { bytes: 0, total: 0 };

        let path = download_to(
            &reqwest::Client::new(),
            &url,
            dir.path(),
            &spec,
            &cancel,
            &mut |p| last = p,
        )
        .await
        .unwrap();

        assert_eq!(path, dir.path().join(spec.filename));
        assert_eq!(read_file(&path), payload());
        assert_eq!(last.bytes, spec.size_bytes);
        assert_eq!(model_state(dir.path(), &spec), ModelState::Ready);
        assert!(!dir.path().join("ggml-test.bin.partial").exists());
        assert_eq!(server.join().unwrap(), vec![None]);
    }

    #[tokio::test]
    async fn resume_appends_from_partial_and_digest_covers_whole_file() {
        let dir = tempdir();
        let spec = test_spec();
        let split = 100_000;
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(
            dir.path().join("ggml-test.bin.partial"),
            &payload()[..split],
        )
        .unwrap();
        assert_eq!(
            model_state(dir.path(), &spec),
            ModelState::Partial(split as u64)
        );

        let (url, server) = serve_payload(1);
        let cancel = AtomicBool::new(false);
        let path = download_to(
            &reqwest::Client::new(),
            &url,
            dir.path(),
            &spec,
            &cancel,
            &mut |_| {},
        )
        .await
        .unwrap();

        assert_eq!(read_file(&path), payload());
        // The server must have been asked for exactly the missing suffix.
        assert_eq!(
            server.join().unwrap(),
            vec![Some(format!("bytes={split}-"))]
        );
    }

    #[tokio::test]
    async fn checksum_mismatch_destroys_partial() {
        let dir = tempdir();
        let mut spec = test_spec();
        spec.sha256 = "00000000000000000000000000000000000000000000000000000000deadbeef";
        let (url, server) = serve_payload(1);
        let cancel = AtomicBool::new(false);

        let err = download_to(
            &reqwest::Client::new(),
            &url,
            dir.path(),
            &spec,
            &cancel,
            &mut |_| {},
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, CoreError::ChecksumMismatch { .. }),
            "got {err:?}"
        );
        assert_eq!(model_state(dir.path(), &spec), ModelState::NotDownloaded);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn cancel_keeps_partial_for_resume() {
        let dir = tempdir();
        let spec = test_spec();
        let (url, server) = serve_payload(1);
        let cancel = Arc::new(AtomicBool::new(false));

        let cancel_in_cb = Arc::clone(&cancel);
        let err = download_to(
            &reqwest::Client::new(),
            &url,
            dir.path(),
            &spec,
            &cancel,
            // Cancel as soon as the first chunk lands.
            &mut move |_| cancel_in_cb.store(true, Ordering::Relaxed),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, CoreError::Cancelled), "got {err:?}");
        match model_state(dir.path(), &spec) {
            ModelState::Partial(n) => assert!(n > 0),
            other => panic!("expected partial after cancel, got {other:?}"),
        }
        let _ = server.join();
    }

    #[tokio::test]
    async fn existing_verified_file_short_circuits_without_network() {
        let dir = tempdir();
        let spec = test_spec();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join(spec.filename), payload()).unwrap();
        let cancel = AtomicBool::new(false);

        // Unroutable URL proves no request is made.
        let path = download_to(
            &reqwest::Client::new(),
            "http://192.0.2.1:1/never",
            dir.path(),
            &spec,
            &cancel,
            &mut |_| {},
        )
        .await
        .unwrap();
        assert_eq!(read_file(&path), payload());
    }

    #[tokio::test]
    async fn http_error_status_is_reported() {
        let dir = tempdir();
        let spec = test_spec();
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/gone", server.server_addr());
        let handle = std::thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(tiny_http::Response::from_data(b"gone".to_vec()).with_status_code(404))
                .unwrap();
        });
        let cancel = AtomicBool::new(false);

        let err = download_to(
            &reqwest::Client::new(),
            &url,
            dir.path(),
            &spec,
            &cancel,
            &mut |_| {},
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, CoreError::HttpStatus { status: 404, .. }),
            "got {err:?}"
        );
        handle.join().unwrap();
    }

    #[test]
    fn impossible_disk_requirement_errors() {
        let dir = tempdir();
        let err = ensure_disk_space(dir.path(), u64::MAX - (64 * 1024 * 1024)).unwrap_err();
        assert!(
            matches!(err, CoreError::InsufficientDisk { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn delete_model_removes_file_and_partial() {
        let dir = tempdir();
        let spec = test_spec();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join(spec.filename), b"x").unwrap();
        std::fs::write(dir.path().join("ggml-test.bin.partial"), b"y").unwrap();
        delete_model(dir.path(), &spec).unwrap();
        assert_eq!(model_state(dir.path(), &spec), ModelState::NotDownloaded);
        // Deleting an absent model is not an error.
        delete_model(dir.path(), &spec).unwrap();
    }

    /// Self-cleaning temp dir without an extra dev-dependency.
    struct TempDir(PathBuf);
    impl TempDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn tempdir() -> TempDir {
        // Parallel test threads can observe the same SystemTime nanos, so a
        // process-wide counter — not a timestamp — guarantees uniqueness.
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "attestrum-transcription-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}
