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
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::test_support::{sha256_hex, TestServer};

    #[tokio::test]
    async fn writes_verified_file_and_reports_progress() {
        let body = vec![7u8; 100_000];
        let server = TestServer::start([("/f", body.clone())]).await;
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("f");
        let last = Cell::new(0);
        download_verified(
            &server.url("/f"),
            &dest,
            100_000,
            &sha256_hex(&body),
            |d, t| {
                assert_eq!(t, 100_000);
                last.set(d);
            },
        )
        .await
        .unwrap();
        assert_eq!(fs::read(&dest).unwrap(), body);
        assert_eq!(last.get(), 100_000);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn rejects_bad_hash_size_and_status_without_leaving_files() {
        let server = TestServer::start([("/f", b"hello".to_vec())]).await;
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("f");
        let good = sha256_hex(b"hello");

        let wrong_hash = sha256_hex(b"other");
        for (path, size, sha) in [
            ("/f", 5, wrong_hash.as_str()),
            ("/f", 4, good.as_str()),
            ("/f", 6, good.as_str()),
            ("/missing", 5, good.as_str()),
        ] {
            let result = download_verified(&server.url(path), &dest, size, sha, |_, _| {}).await;
            assert!(result.is_err(), "{path} {size}");
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
        }
    }

    #[tokio::test]
    async fn failed_download_keeps_existing_destination() {
        let server = TestServer::start([("/f", b"new".to_vec())]).await;
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("f");
        fs::write(&dest, b"old").unwrap();
        let bad = sha256_hex(b"x");
        assert!(
            download_verified(&server.url("/f"), &dest, 3, &bad, |_, _| {})
                .await
                .is_err()
        );
        assert_eq!(fs::read(&dest).unwrap(), b"old");
    }
}
