//! The one HTTP client and the one verified-download routine.

mod download;

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::Client;

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
        return Err(Error::HttpStatus {
            status: status.as_u16(),
            url: url.to_string(),
        });
    }
    Ok(response)
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
