//! Download API for downloading the artifacts from the server

use std::path::{Path, PathBuf};
use std::io::{Cursor, copy};
use std::fs::File;

use anyhow::{Error, Result};
use async_trait::async_trait;
use http::StatusCode;
use sha2::{Digest, Sha256};
use tracing::instrument;

use crate::fvm::Artifact;
use crate::htclient;

#[async_trait]
pub trait Download {
    /// Downloads the artifact to the specified directory
    ///
    /// Checksum validation, when metadata is available, is performed against
    /// the raw bytes returned from the artifact's `download_url` (for example
    /// a `.zip` archive) **before** any extraction. The checksum does not
    /// currently apply to any binary extracted from an archive.
    ///
    /// Returns the path to the downloaded (and, if applicable, extracted)
    /// artifact.
    async fn download(&self, target_dir: PathBuf) -> Result<PathBuf>;
}

#[async_trait]
impl Download for Artifact {
    #[instrument(skip(self, target_dir))]
    async fn download(&self, target_dir: PathBuf) -> Result<PathBuf> {
        tracing::info!(
            name = self.name,
            download_url = ?self.download_url,
            "Downloading artifact"
        );

        let res = htclient::get(&self.download_url)
            .await
            .map_err(|err| Error::msg(err.to_string()))?;

        let status = http::StatusCode::from_u16(res.status().as_u16())?;
        if status == StatusCode::OK {
            let content_type = res
                .headers()
                .get(http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_ascii_lowercase());

            let bytes = res.into_body();

            // delegate to helper which is easier to test
            return process_downloaded_bytes(&bytes, content_type, self, &target_dir);
        }

        Err(Error::msg(format!(
            "Server responded with Status Code {} for url {}",
            res.status(),
            self.download_url,
        )))
    }
}

/// Internal helper that implements the logic for handling downloaded bytes.
/// Extracts files if zip, validates checksum if provided, writes final file
/// to `target_dir` and returns the path.
fn process_downloaded_bytes(
    bytes: &[u8],
    content_type: Option<String>,
    artifact: &Artifact,
    target_dir: &Path,
) -> Result<PathBuf> {
    let out_path = target_dir.join(&artifact.name);

    if let Some(expected_digest) = &artifact.sha256_digest {
        let expected = expected_digest.trim();
        let expected = expected
            .strip_prefix("sha256:")
            .unwrap_or(expected)
            .to_ascii_lowercase();

        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let actual = format!("{:x}", hasher.finalize());

        if actual != expected {
            let msg = format!(
                "DANGER: Downloaded artifact checksum did not match for {}",
                artifact.name
            );
            tracing::error!(
                name = artifact.name,
                %expected,
                %actual,
                digest_scope = "archive",
                "Checksum validation failed for downloaded artifact (archive) bytes",
            );

            return Err(Error::msg(msg));
        }

        tracing::debug!(
            name = artifact.name,
            %expected,
            %actual,
            digest_scope = "archive",
            "Checksum validation succeeded for downloaded artifact (archive) bytes",
        );
    }

    let mut file = File::create(&out_path)?;

    let is_zip_ct = content_type.as_deref().is_some_and(|ct| ct.contains("zip"));

    if is_zip_ct || is_zip_archive(bytes) {
        // if the artifact is a zip file, we need to unzip it first
        let reader = std::io::Cursor::new(&bytes);
        let mut zip = zip::ZipArchive::new(reader)?;
        if zip.is_empty() {
            return Err(Error::msg("Downloaded zip archive is empty"));
        }

        let mut selected_index: Option<usize> = None;

        // look file entries to find the one that matches the artifact name
        for i in 0..zip.len() {
            let file_in_zip = zip.by_index(i)?;

            if file_in_zip.is_dir() {
                continue;
            }

            let entry_name = file_in_zip.name();

            if selected_index.is_none() {
                selected_index = Some(i);
            }

            if entry_name.ends_with(&artifact.name) {
                selected_index = Some(i);
                break;
            }
        }

        let selected_index = selected_index.ok_or_else(|| {
            Error::msg("Downloaded zip archive does not contain any file entries")
        })?;

        let mut zipped_file = zip.by_index(selected_index)?;
        let expected_size = zipped_file.size();
        let written = copy(&mut zipped_file, &mut file)?;

        if written == 0 {
            return Err(Error::msg("Downloaded zip entry is empty"));
        }

        if written != expected_size {
            return Err(Error::msg(
                "Extracted file size does not match zip entry size",
            ));
        }
    } else {
        let mut buf = Cursor::new(&bytes);
        let written = copy(&mut buf, &mut file)?;

        if written == 0 {
            return Err(Error::msg("Downloaded artifact is empty"));
        }
    }

    tracing::debug!(
        name = artifact.name,
        out_path = ?out_path.display(),
        "Artifact downloaded",
    );

    Ok(out_path)
}

fn is_zip_archive(bytes: &[u8]) -> bool {
    const ZIP_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
    bytes.len() >= ZIP_MAGIC.len() && bytes[..ZIP_MAGIC.len()] == ZIP_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::io::Write;
    use sha2::{Digest, Sha256};

    use zip::write::FileOptions;

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    #[test]
    fn extracts_correct_entry_from_multi_file_zip() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path().to_path_buf();

        // create zip with multiple files, one of them ends with "myartifact"
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options: FileOptions<'_, ()> = FileOptions::default();

            zip.start_file("bin/other", options).unwrap();
            zip.write_all(b"other-content").unwrap();

            zip.start_file("bin/myartifact", options).unwrap();
            zip.write_all(b"expected-binary-data").unwrap();

            zip.finish().unwrap();
        }
        let bytes = buffer.into_inner();

        let digest = sha256_hex(&bytes);

        let artifact = Artifact {
            name: "myartifact".to_string(),
            version: semver::Version::new(0, 0, 0),
            download_url: "http://example.com".to_string(),
            sha256_digest: Some(format!("sha256:{}", digest)),
        };

        let out = process_downloaded_bytes(
            &bytes,
            Some("application/zip".to_string()),
            &artifact,
            &target_dir,
        )
        .unwrap();

        let content = std::fs::read(out).unwrap();
        assert_eq!(content, b"expected-binary-data");
    }

    #[test]
    fn fails_on_checksum_mismatch() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path().to_path_buf();

        let bytes = b"notmatching".to_vec();
        // compute different digest to ensure mismatch
        let artifact = Artifact {
            name: "foo".to_string(),
            version: semver::Version::new(0, 0, 0),
            download_url: "http://example.com".to_string(),
            sha256_digest: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            ),
        };

        let res = process_downloaded_bytes(
            &bytes,
            Some("application/octet-stream".to_string()),
            &artifact,
            &target_dir,
        );
        assert!(res.is_err());
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("checksum") || msg.contains("DANGER"));
    }

    #[test]
    fn fails_on_empty_zip() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path().to_path_buf();

        // create empty zip
        let mut buffer = Cursor::new(Vec::new());
        {
            let zip = zip::ZipWriter::new(&mut buffer);
            zip.finish().unwrap();
        }
        let bytes = buffer.into_inner();

        let artifact = Artifact {
            name: "something".to_string(),
            version: semver::Version::new(0, 0, 0),
            download_url: "http://example.com".to_string(),
            sha256_digest: None,
        };

        let res = process_downloaded_bytes(
            &bytes,
            Some("application/zip".to_string()),
            &artifact,
            &target_dir,
        );
        assert!(res.is_err());
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("zip archive is empty"));
    }

    #[test]
    fn fails_on_empty_zip_entry() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path().to_path_buf();

        // create zip with an empty file entry named "emptyfile"
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options: FileOptions<'_, ()> = FileOptions::default();
            zip.start_file("emptyfile", options).unwrap();
            zip.finish().unwrap();
        }
        let bytes = buffer.into_inner();

        let artifact = Artifact {
            name: "emptyfile".to_string(),
            version: semver::Version::new(0, 0, 0),
            download_url: "http://example.com".to_string(),
            sha256_digest: None,
        };

        let res = process_downloaded_bytes(
            &bytes,
            Some("application/zip".to_string()),
            &artifact,
            &target_dir,
        );
        assert!(res.is_err());
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("zip entry is empty"));
    }
}
