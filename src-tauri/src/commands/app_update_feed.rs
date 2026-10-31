//! Bounded, status-aware download of the unsigned updater manifest.

use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_updater::{Update, Updater, UpdaterExt};
use url::Url;

use super::app_update::UpdateChannel;
use crate::error::{CommandError, CommandResult};

const MAX_FEED_BYTES: usize = 64 * 1024;
const MAX_ARTIFACT_BYTES: usize = 512 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

fn failed(error: impl std::fmt::Display) -> CommandError {
    CommandError::new("update", error.to_string())
}

fn updater_for(app: &AppHandle, channel: UpdateChannel) -> CommandResult<Updater> {
    let endpoint = Url::parse(channel.endpoint()).map_err(failed)?;
    app.updater_builder()
        .endpoints(vec![endpoint])
        .map_err(failed)?
        .build()
        .map_err(failed)
}

fn github_redirect(attempt: reqwest::redirect::Attempt<'_>) -> reqwest::redirect::Action {
    let url = attempt.url();
    let host = url.host_str();
    let github_host = matches!(
        host,
        Some("github.com")
            | Some("release-assets.githubusercontent.com")
            | Some("objects.githubusercontent.com")
            | Some("github-releases.githubusercontent.com")
    );
    let secure_url = url.scheme() == "https"
        && url.port().is_none_or(|port| port == 443)
        && url.username().is_empty()
        && url.password().is_none();
    if github_host && secure_url && attempt.previous().len() < 5 {
        attempt.follow()
    } else {
        attempt.stop()
    }
}

async fn manifest_bytes(endpoint: Url) -> CommandResult<Option<Vec<u8>>> {
    let client = github_client(REQUEST_TIMEOUT)?;
    let mut response = client.get(endpoint).send().await.map_err(failed)?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if response.status() != reqwest::StatusCode::OK {
        return Err(failed(format!(
            "Update feed returned HTTP {}.",
            response.status()
        )));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_FEED_BYTES as u64)
    {
        return Err(failed("Update feed exceeded the 64 KiB limit."));
    }

    let bytes = read_bounded(&mut response, MAX_FEED_BYTES, "Update feed").await?;
    Ok(Some(bytes))
}

fn github_client(timeout: Duration) -> CommandResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::custom(github_redirect))
        .build()
        .map_err(failed)
}

async fn read_bounded(
    response: &mut reqwest::Response,
    max_bytes: usize,
    label: &str,
) -> CommandResult<Vec<u8>> {
    let content_length = response.content_length();
    if content_length.is_some_and(|length| length > max_bytes as u64) {
        return Err(failed(format!("{label} exceeded the size limit.")));
    }
    let mut bytes = Vec::new();
    if let Some(length) = content_length {
        bytes
            .try_reserve_exact(length as usize)
            .map_err(|error| failed(format!("Could not allocate update data: {error}")))?;
    }
    while let Some(chunk) = response.chunk().await.map_err(failed)? {
        if bytes.len().saturating_add(chunk.len()) > max_bytes {
            return Err(failed(format!("{label} exceeded the size limit.")));
        }
        if content_length.is_none() {
            bytes
                .try_reserve_exact(chunk.len())
                .map_err(|error| failed(format!("Could not allocate update data: {error}")))?;
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Fetches and validates the channel manifest, then lets Tauri select the current platform entry.
pub(crate) async fn check(
    app: &AppHandle,
    channel: UpdateChannel,
) -> CommandResult<Option<Update>> {
    let updater = updater_for(app, channel)?;
    let endpoint = Url::parse(channel.endpoint()).map_err(failed)?;
    let Some(bytes) = manifest_bytes(endpoint).await? else {
        return Ok(None);
    };
    let manifest: serde_json::Value = serde_json::from_slice(&bytes).map_err(failed)?;
    let version = manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| failed("Update feed has no version."))?;
    let version = semver::Version::parse(version).map_err(failed)?;
    match channel {
        UpdateChannel::Stable if !version.pre.is_empty() => return Ok(None),
        UpdateChannel::Beta if version.pre.is_empty() => {
            return Err(failed(
                "Beta update feed did not contain a prerelease version.",
            ));
        }
        _ => {}
    }

    updater.restore_update(manifest).map_err(failed)
}

pub(crate) fn validate_artifact_url(update: &Update) -> CommandResult<()> {
    let url = &update.download_url;
    if url.scheme() != "https" || url.host_str() != Some("github.com") || url.port().is_some() {
        return Err(failed(
            "Update artifact URL is outside the trusted GitHub host.",
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(failed("Update artifact URL must not contain credentials."));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(failed(
            "Update artifact URL must not include a query or fragment.",
        ));
    }
    let segments: Vec<_> = url.path_segments().into_iter().flatten().collect();
    if segments.len() != 6
        || segments[0] != "Starnished66"
        || segments[1] != "compas-tools"
        || segments[2] != "releases"
        || segments[3] != "download"
        || segments[4] != format!("v{}", update.version)
    {
        return Err(failed(
            "Update artifact URL must point to this repository's versioned release assets.",
        ));
    }
    let asset = segments[5];
    #[cfg(target_os = "linux")]
    let valid_extension = asset.ends_with(".AppImage");
    #[cfg(target_os = "macos")]
    let valid_extension = asset.ends_with(".app.tar.gz");
    #[cfg(target_os = "windows")]
    let valid_extension = asset.ends_with(".exe");
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let valid_extension = false;
    if asset.is_empty() || !valid_extension {
        return Err(failed(
            "Update artifact has an unsupported installer format.",
        ));
    }
    Ok(())
}

/// Downloads one version-bound GitHub artifact with a 300 second deadline and a 512 MiB cap.
pub(crate) async fn download_artifact(
    update: &Update,
    mut on_chunk: impl FnMut(u64, Option<u64>),
) -> CommandResult<Vec<u8>> {
    validate_artifact_url(update)?;
    let client = github_client(DOWNLOAD_TIMEOUT)?;
    let mut response = client
        .get(update.download_url.clone())
        .send()
        .await
        .map_err(failed)?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(failed(format!(
            "Update artifact returned HTTP {}.",
            response.status()
        )));
    }
    let content_length = response.content_length();
    if content_length.is_some_and(|length| length > MAX_ARTIFACT_BYTES as u64) {
        return Err(failed("Update artifact exceeded the 512 MiB limit."));
    }
    let mut bytes = Vec::new();
    if let Some(length) = content_length {
        bytes
            .try_reserve_exact(length as usize)
            .map_err(|error| failed(format!("Could not allocate update data: {error}")))?;
    }
    while let Some(chunk) = response.chunk().await.map_err(failed)? {
        if bytes.len().saturating_add(chunk.len()) > MAX_ARTIFACT_BYTES {
            return Err(failed("Update artifact exceeded the 512 MiB limit."));
        }
        if content_length.is_none() {
            bytes
                .try_reserve_exact(chunk.len())
                .map_err(|error| failed(format!("Could not allocate update data: {error}")))?;
        }
        bytes.extend_from_slice(&chunk);
        on_chunk(bytes.len() as u64, content_length);
    }
    Ok(bytes)
}
