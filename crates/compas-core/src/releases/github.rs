use std::collections::HashMap;

use serde::Deserialize;

use super::{Asset, Release};
use crate::error::{Error, Result};
use crate::fs::is_sha256;
use crate::net::{get_bytes, METADATA_LIMIT};

pub const API_BASE: &str = "https://api.github.com";
const REPO: &str = "Starnished66/compas-player";
const SUMS_LIMIT: usize = 64 << 10;

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    html_url: String,
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

pub async fn fetch_latest() -> Result<Release> {
    fetch_latest_from(API_BASE).await
}

/// The latest release and its `SHA256SUMS`, which must be among its assets.
pub async fn fetch_latest_from(api_base: &str) -> Result<Release> {
    let bytes = get_bytes(&format!("{api_base}/repos/{REPO}/releases/latest"), METADATA_LIMIT).await?;
    let api: ApiRelease =
        serde_json::from_slice(&bytes).map_err(|e| Error::InvalidRelease(e.to_string()))?;
    let sums = api
        .assets
        .iter()
        .find(|a| a.name == "SHA256SUMS")
        .ok_or_else(|| Error::InvalidRelease(format!("{} has no SHA256SUMS", api.tag_name)))?;
    let sums = get_bytes(&sums.browser_download_url, SUMS_LIMIT).await?;

    Ok(Release {
        name: api.name.filter(|n| !n.is_empty()).unwrap_or_else(|| api.tag_name.clone()),
        tag: api.tag_name,
        notes: api.body.unwrap_or_default(),
        published_at: api.published_at.unwrap_or_default(),
        url: api.html_url,
        assets: api
            .assets
            .into_iter()
            .map(|a| Asset { name: a.name, size: a.size, url: a.browser_download_url })
            .collect(),
        checksums: parse_sums(&String::from_utf8_lossy(&sums)),
    })
}

/// `sha256sum` output: `<64 hex>  <name>` (or `<hex> *<name>` in binary mode).
fn parse_sums(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (sha, name) = line.split_once(' ')?;
            let name = name.trim_start_matches([' ', '*']);
            (is_sha256(sha) && !name.is_empty())
                .then(|| (name.to_string(), sha.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sha256sum_lines() {
        let sha = "a".repeat(64);
        let sums = parse_sums(&format!("{sha}  one\n{sha} *two\nnot a line\n{}  bad\n", "g".repeat(64)));
        assert_eq!(sums.len(), 2);
        assert_eq!(sums["one"], sha);
        assert_eq!(sums["two"], sha);
    }
}
