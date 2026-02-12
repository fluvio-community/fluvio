use std::path::PathBuf;

use anyhow::{bail, Result};
use semver::Version;
use tempfile::TempDir;

use fluvio_artifacts_util::fvm::{Client as FvmClient, Channel as FvmChannel, Download as _};

use crate::common::executable::{remove_fvm_binary_if_exists, set_executable_mode};

use super::notify::Notify;
use super::workdir::fvm_bin_path;
use super::TARGET;

/// Updates Manager for the Fluvio Version Manager
pub struct UpdateManager {
    notify: Notify,
}

impl UpdateManager {
    pub fn new(notify: &Notify) -> Self {
        Self {
            notify: notify.to_owned(),
        }
    }

    pub async fn update(&self, version: &Version) -> Result<()> {
        self.notify.info(format!("Downloading fvm@{version}"));
        let (_tmp_dir, new_fvm_bin) = self.download(version).await?;

        self.notify.info(format!("Installing fvm@{version}"));
        self.install(&new_fvm_bin).await?;
        self.notify
            .done(format!("Installed fvm@{version} with success"));

        Ok(())
    }

    /// Downloads Fluvio Version Manager binary into a temporary directory
    async fn download(&self, version: &Version) -> Result<(TempDir, PathBuf)> {
        let tmp_dir = TempDir::new()?;
        let channel = FvmChannel::Tag(version.clone());
        let client = FvmClient;

        // Fetch the unfiltered package set for the requested version and
        // current target so that the `fvm` binary artifact is included.
        let package_set = client.fetch_package_set(&channel, TARGET).await?;

        // Locate the FVM artifact within the package set
        let Some(fvm_artifact) = package_set
            .artifacts
            .iter()
            .find(|artifact| artifact.name == "fvm")
        else {
            bail!("FVM artifact not found in package set for version {version}");
        };

        // Require a SHA-256 digest for the FVM artifact so that integrity
        // verification is enforced during download. If the digest is missing,
        // we abort the self-update rather than proceeding unchecked.
        if fvm_artifact.sha256_digest.is_none() {
            bail!(
                "Integrity verification unavailable for FVM artifact (missing sha256 digest) for version {version}. Please use a newer version of FVM to update."
            );
        }

        let out_path = fvm_artifact.download(tmp_dir.path().to_path_buf()).await?;

        set_executable_mode(&out_path)?;

        Ok((tmp_dir, out_path))
    }

    async fn install(&self, new_fvm_bin: &PathBuf) -> Result<()> {
        let old_fvm_bin = fvm_bin_path()?;

        if !new_fvm_bin.exists() {
            tracing::warn!(?new_fvm_bin, "New fvm binary not found. Aborting update.");
            bail!("Failed to update FVM due to missing binary");
        }

        remove_fvm_binary_if_exists()?;

        tracing::warn!(src=?new_fvm_bin, dst=?old_fvm_bin , "Copying new fvm binary");
        std::fs::copy(new_fvm_bin, &old_fvm_bin)?;

        Ok(())
    }
}
