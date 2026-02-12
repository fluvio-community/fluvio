//! Hub FVM API Client

use anyhow::{Result};
use octocrab::Octocrab;
use semver::Version;

use crate::{
    REPO_OWNER, REPO_NAME,
    fvm::{Artifact, Channel, PackageSet},
};

// List of binaries that are installable via FVM
// We may consider a more flexible approach in the future
const FVM_INSTALLABLE_BINARIES: &[&str] = &["fluvio", "fluvio-run", "cdk", "smdk"];
/// HTTP Client for interacting with the Hub FVM API
#[derive(Debug, Default)]
pub struct Client;

impl Client {
    /// Internal helper: resolves the GitHub release and semantic version for
    /// a given FVM channel.
    async fn fetch_release_and_version(
        &self,
        channel: &Channel,
    ) -> Result<(octocrab::models::repos::Release, Version)> {
        let octocrab = Octocrab::builder().build()?;

        let (release, version) = match channel {
            Channel::Stable => {
                // we have to fetch last release id from github
                let release = octocrab
                    .repos(REPO_OWNER, REPO_NAME)
                    .releases()
                    .get_latest()
                    .await
                    .map_err(|e| anyhow::anyhow!("Unable to retrieve stable release: {e}"))?;
                let version = Version::parse(release.tag_name.trim_start_matches('v'))?;

                (release, version)
            }
            Channel::Tag(ver) => {
                let release_id = format!("v{}", ver);
                let release = octocrab
                    .repos(REPO_OWNER, REPO_NAME)
                    .releases()
                    .get_by_tag(&release_id)
                    .await
                    .map_err(|e| {
                        if let octocrab::Error::GitHub { source, .. } = &e {
                            anyhow::anyhow!(
                                "Unable to retrieve release for tag {release_id}: {}",
                                source.message
                            )
                        } else {
                            anyhow::anyhow!("Unable to retrieve release for tag {release_id}: {e}")
                        }
                    })?;
                (release, ver.clone())
            }
            Channel::Latest => {
                let release = octocrab
                    .repos(REPO_OWNER, REPO_NAME)
                    .releases()
                    .get_by_tag("dev")
                    .await
                    .map_err(|e| anyhow::anyhow!("Unable to retrieve release for tag dev: {e}"))?;

                // Derive the version for the `latest` (dev) channel from the
                // VERSION file in the fluvio repository at the same ref as the
                // dev release tag
                let content_items = octocrab
                    .repos(REPO_OWNER, REPO_NAME)
                    .get_content()
                    .path("VERSION")
                    .r#ref(release.tag_name.clone())
                    .send()
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Unable to retrieve VERSION file for dev release: {e}")
                    })?;

                let version_str = content_items
                    .items
                    .into_iter()
                    .next()
                    .and_then(|c| c.decoded_content())
                    .ok_or_else(|| {
                        anyhow::anyhow!("VERSION file for dev release is missing or empty")
                    })?;

                let version = Version::parse(version_str.trim()).map_err(|e| {
                    anyhow::anyhow!("Invalid version string in VERSION file for dev release: {e}")
                })?;

                (release, version)
            }
            Channel::Other(release) => {
                let release = octocrab
                    .repos(REPO_OWNER, REPO_NAME)
                    .releases()
                    .get_by_tag(release)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Unable to retrieve release for tag {release}: {e}")
                    })?;
                let version = Version::parse(release.tag_name.trim_start_matches('v'))?;
                (release, version)
            }
        };

        Ok((release, version))
    }

    /// Fetches a [`PackageSet`] from GitHub that includes only the
    /// "installable" binaries (e.g. fluvio, fluvio-run, cdk, smdk).
    pub async fn fetch_default_package_set(
        &self,
        channel: &Channel,
        arch: &str,
    ) -> Result<PackageSet> {
        // Start from the unfiltered package set (which includes all
        // arch-specific artifacts) and then filter down to the
        // "installable" binaries.
        let mut pkgset = self.fetch_package_set(channel, arch).await?;

        pkgset.artifacts.retain(|artifact| {
            FVM_INSTALLABLE_BINARIES
                .iter()
                .any(|bin| artifact.name == *bin || artifact.name == format!("{bin}.exe"))
        });

        if pkgset.artifacts.is_empty() {
            return Err(anyhow::anyhow!(
                "Release \"{}\" does not have installable artifacts for architecture: \"{arch}\"",
                pkgset.pkgset
            ));
        }

        Ok(pkgset)
    }

    /// Fetches a [`PackageSet`] from GitHub without filtering binaries by the
    /// `FVM_INSTALLABLE_BINARIES` list.
    pub async fn fetch_package_set(&self, channel: &Channel, arch: &str) -> Result<PackageSet> {
        let (release, version) = self.fetch_release_and_version(channel).await?;

        let artifacts: Vec<_> = release
            .assets
            .iter()
            .filter(|asset| asset.name.ends_with(&format!("{arch}.zip")))
            .map(|asset| Artifact {
                name: asset
                    .name
                    .trim_end_matches(&format!("-{arch}.zip"))
                    .to_string(),
                version: version.clone(),
                download_url: asset.browser_download_url.to_string(),
                sha256_digest: asset.digest.clone(),
            })
            .collect();

        if artifacts.is_empty() {
            return Err(anyhow::anyhow!(
                "Release \"{}\" does not have artifacts for architecture: \"{arch}\"",
                release.tag_name
            ));
        }

        let package_set = PackageSet {
            arch: arch.to_string(),
            pkgset: version,
            artifacts,
        };

        Ok(package_set)
    }
}
