use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use super::get;
use crate::error::{Error, IoContext, Result};
use crate::fs::{persist, temp_sibling};

/// Streams `url` into `dest`, checking size and SHA-256 on the way. The file
/// appears at `dest` only once it is complete and verified; on any failure
/// nothing is left behind. `on_progress` receives `(done, total)` bytes.
pub async fn download_verified(
    url: &str,
    dest: &Path,
    size: u64,
    sha256: &str,
    on_progress: impl Fn(u64, u64),
) -> Result<()> {
    let tmp = temp_sibling(dest);
    let result = stream_to(url, &tmp, size, sha256, on_progress).await;
    let result = result.and_then(|()| persist(&tmp, dest));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

async fn stream_to(
    url: &str,
    tmp: &Path,
    size: u64,
    sha256: &str,
    on_progress: impl Fn(u64, u64),
) -> Result<()> {
    let mut response = get(url).await?;
    let mut file = tokio::fs::File::create(tmp).await.at(tmp)?;
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    while let Some(chunk) = response.chunk().await? {
        done += chunk.len() as u64;
        if done > size {
            return Err(Error::Verification(format!(
                "{url} is larger than {size} bytes"
            )));
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await.at(tmp)?;
        on_progress(done, size);
    }
    file.flush().await.at(tmp)?;
    if done != size {
        return Err(Error::Verification(format!(
            "{url} has {done} bytes, expected {size}"
        )));
    }
    if format!("{:x}", hasher.finalize()) != sha256 {
        return Err(Error::Verification(format!("{url} has the wrong SHA-256")));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
