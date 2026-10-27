//! The one HTTP client and the one verified-download routine.

mod download;

use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};

use crate::error::{Error, Result};

pub use download::download_verified;

/// Largest metadata document (plugin index, release JSON) we will read.
pub const METADATA_LIMIT: usize = 1 << 20;

pub fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent(concat!("compas-tools/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .build()
            .expect("static client configuration is valid")
    })
}

pub(crate) async fn get(url: &str) -> Result<reqwest::Response> {
    let response = client().get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        if let Some(error) = rate_limit_error(status, response.headers(), now_secs()) {
            return Err(error);
        }
        return Err(Error::HttpStatus {
            status: status.as_u16(),
            url: url.to_string(),
        });
    }
    Ok(response)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// GitHub answers 403 or 429 with `X-RateLimit-Remaining: 0` once the hourly
/// allowance is used up; `X-RateLimit-Reset` says when it renews.
fn rate_limit_error(status: StatusCode, headers: &HeaderMap, now: u64) -> Option<Error> {
    let header = |name: &str| headers.get(name)?.to_str().ok()?.parse::<u64>().ok();
    let limited =
        matches!(status.as_u16(), 403 | 429) && header("x-ratelimit-remaining") == Some(0);
    limited.then(|| Error::RateLimited {
        retry_in_secs: header("x-ratelimit-reset").map_or(0, |reset| reset.saturating_sub(now)),
    })
}

/// Fetches a small document, refusing anything over `limit` bytes.
pub async fn get_bytes(url: &str, limit: usize) -> Result<Vec<u8>> {
    let mut response = get(url).await?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > limit {
            return Err(Error::Verification(format!(
                "{url} is larger than {limit} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests;
