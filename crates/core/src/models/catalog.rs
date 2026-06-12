//! Whisper model catalog.
//!
//! Sizes and SHA-256 digests are pinned from the Hugging Face LFS metadata for
//! `ggerganov/whisper.cpp` (fetched 2026-06-12). A model file is only ever
//! loaded after its digest verified — see `docs/diagrams/models/model-download.md`.

/// One downloadable whisper model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ModelSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub filename: &'static str,
    pub size_bytes: u64,
    /// Lowercase hex SHA-256 of the model file.
    pub sha256: &'static str,
}

/// Base URL the catalog downloads from. Tests override the URL at the
/// `download_to` layer instead of mocking DNS.
pub const HF_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

pub const CATALOG: &[ModelSpec] = &[
    ModelSpec {
        id: "tiny",
        display_name: "Tiny — fastest, rough draft quality",
        filename: "ggml-tiny.bin",
        size_bytes: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
    },
    ModelSpec {
        id: "base",
        display_name: "Base — fast, low-end hardware",
        filename: "ggml-base.bin",
        size_bytes: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    },
    ModelSpec {
        id: "small",
        display_name: "Small — balanced speed and accuracy",
        filename: "ggml-small.bin",
        size_bytes: 487_601_967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
    },
    ModelSpec {
        id: "large-v3-turbo",
        display_name: "Large v3 Turbo — best accuracy, needs a fast machine",
        filename: "ggml-large-v3-turbo.bin",
        size_bytes: 1_624_555_275,
        sha256: "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
    },
];

/// Look up a model by catalog id.
pub fn spec(id: &str) -> Option<&'static ModelSpec> {
    CATALOG.iter().find(|m| m.id == id)
}

/// Download URL for a model.
pub fn url(spec: &ModelSpec) -> String {
    format!("{HF_BASE_URL}/{}", spec.filename)
}

/// Platform-aware default: Apple Silicon runs `large-v3-turbo` on Metal at
/// acceptable speed; everything else (notably CPU-only Windows) defaults to
/// `small`.
pub fn default_model_id() -> &'static str {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "large-v3-turbo"
    } else {
        "small"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_are_unique_and_resolvable() {
        for m in CATALOG {
            assert_eq!(spec(m.id).unwrap().filename, m.filename);
        }
        let mut ids: Vec<_> = CATALOG.iter().map(|m| m.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len());
    }

    #[test]
    fn digests_are_lowercase_hex_sha256() {
        for m in CATALOG {
            assert_eq!(m.sha256.len(), 64, "{} digest length", m.id);
            assert!(
                m.sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{} digest must be lowercase hex",
                m.id
            );
        }
    }

    #[test]
    fn unknown_id_is_none() {
        assert!(spec("medium").is_none());
    }

    #[test]
    fn urls_point_at_hf_resolve_main() {
        let m = spec("tiny").unwrap();
        assert_eq!(
            url(m),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"
        );
    }

    #[test]
    fn default_model_is_in_catalog() {
        assert!(spec(default_model_id()).is_some());
    }
}
