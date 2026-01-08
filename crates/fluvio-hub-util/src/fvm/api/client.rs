//! Hub FVM API Client

use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::fvm::{Channel, PackageSet, PackageSetRecord};

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

/// HTTP Client for interacting with the Hub FVM API
pub struct Client {
    api_url: Url,
}

impl Client {
    /// Creates a new [`Client`] with the default Hub API URL
    pub fn new(url: &str) -> Result<Self> {
        let api_url = url.parse::<Url>()?;

        Ok(Self { api_url })
    }

    /// Fetches a [`PackageSet`] from the Hub with the specific [`Channel`]
    pub async fn fetch_package_set(&self, channel: &Channel, arch: &str) -> Result<PackageSet> {
        use crate::htclient::ResponseExt;
        use std::collections::HashMap;

        let url = self.make_fetch_package_set_url(channel)?;
        let res = crate::htclient::get(url)
            .await
            .map_err(|err| Error::msg(err.to_string()))?;
        let res_status = res.status();

        if res_status.is_success() {
            let manifest = res.json::<HashMap<String, PackageSetRecord>>().map_err(|err| {
                tracing::debug!(?err, "Failed to parse manifest from GitHub releases");
                Error::msg("Failed to parse manifest file")
            })?;

            let pkgset_record = manifest
                .get(arch)
                .ok_or_else(|| Error::msg(format!("Architecture '{}' not found in manifest", arch)))?;

            tracing::info!(?pkgset_record, "Found PackageSet");
            return Ok(pkgset_record.clone().into());
        }

        let error = res.json::<ApiError>().map_err(|err| {
            tracing::debug!(?err, "Failed to parse API Error");
            Error::msg(format!("Server responded with status code {res_status}"))
        })?;

        tracing::debug!(?error, "Server responded with not successful status code");

        Err(anyhow::anyhow!(error.message))
    }

    /// Builds the URL to fetch a [`PackageSet`] manifest from GitHub releases
    /// using the [`Client`]'s `api_url`.
    ///
    /// For example: https://github.com/fluvio-community/fluvio/releases/download/v0.18.1/manifest.json
    fn make_fetch_package_set_url(&self, channel: &Channel) -> Result<Url> {
        let version = match channel {
            Channel::Stable => "stable",
            Channel::Latest => "latest",
            Channel::Tag(v) => &format!("v{}", v),
            Channel::Other(s) => s.as_str(),
        };

        let url = Url::parse(&format!(
            "{}/releases/download/{}/manifest.json",
            self.api_url,
            version
        ))?;

        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use url::Url;
    use semver::Version;

    use super::{Client, Channel};

    #[test]
    fn creates_a_default_client() {
        let client = Client::new("https://github.com/fluvio-community/fluvio").unwrap();

        assert_eq!(
            client.api_url,
            Url::parse("https://github.com/fluvio-community/fluvio").unwrap()
        );
    }

    #[test]
    fn builds_urls_for_fetching_pkgsets() {
        // Scenario: Using Stable Channel

        let client = Client::new("https://github.com/fluvio-community/fluvio").unwrap();
        let url = client
            .make_fetch_package_set_url(&Channel::Stable)
            .unwrap();

        assert_eq!(
            url.as_str(),
            "https://github.com/fluvio-community/fluvio/releases/download/stable/manifest.json",
            "failed on Scenario Using Stable Channel"
        );

        // Scenario: Using Latest Channel

        let client = Client::new("https://github.com/fluvio-community/fluvio").unwrap();
        let url = client
            .make_fetch_package_set_url(&Channel::Latest)
            .unwrap();

        assert_eq!(
            url.as_str(),
            "https://github.com/fluvio-community/fluvio/releases/download/latest/manifest.json",
            "failed on Scenario Using Latest Channel"
        );

        // Scenario: Using Tag

        let client = Client::new("https://github.com/fluvio-community/fluvio").unwrap();
        let url = client
            .make_fetch_package_set_url(&Channel::Tag(Version::from_str("0.10.14").unwrap()))
            .unwrap();

        assert_eq!(
            url.as_str(),
            "https://github.com/fluvio-community/fluvio/releases/download/v0.10.14/manifest.json",
            "failed on Scenario Using Tag"
        );
    }
}
