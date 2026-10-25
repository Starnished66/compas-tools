use std::fs;
use std::path::{Path, PathBuf};

use super::index::Preview;
use super::Catalog;
use crate::error::{Error, IoContext, Result};
use crate::fs::sha256_file;
use crate::net::download_verified;

/// Returns the cached preview image of a plugin, downloading and verifying it
/// first if needed. The file name is its SHA-256, so a stale file can never be
/// mistaken for a new image.
pub async fn ensure_preview(catalog: &Catalog, id: &str, cache_dir: &Path) -> Result<PathBuf> {
    let preview = catalog
        .get(id)?
        .preview
        .as_ref()
        .ok_or_else(|| Error::NotFound(format!("preview of {id}")))?;
    let path = cache_dir.join(format!("{}.jpg", preview.sha256));
    if !is_valid(&path, preview) {
        fs::create_dir_all(cache_dir).at(cache_dir)?;
        let url = catalog.asset_url(&preview.asset);
        download_verified(&url, &path, preview.size, &preview.sha256, |_, _| {}).await?;
    }
    Ok(path)
}

fn is_valid(path: &Path, preview: &Preview) -> bool {
    fs::metadata(path).is_ok_and(|m| m.len() == preview.size)
        && sha256_file(path).is_ok_and(|sha| sha == preview.sha256)
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{publish, spec};
    use super::*;
    
    #[tokio::test]
    async fn downloads_once_and_repairs_a_corrupt_cache_file() {
        let image = b"\xff\xd8 pretend jpeg \xff\xd9".to_vec();
        let s = spec("p", "1.0", &[(".plugins/P.lua", "x", false)]).with_preview(&image);
        let (_server, catalog) = publish(&[s]).await;

        let cache = tempfile::tempdir().unwrap();
        let path = ensure_preview(&catalog, "p", cache.path()).await.unwrap();
        assert_eq!(fs::read(&path).unwrap(), image);

        fs::write(&path, b"corrupt").unwrap();
        let path = ensure_preview(&catalog, "p", cache.path()).await.unwrap();
        assert_eq!(fs::read(&path).unwrap(), image);

        assert!(matches!(
            ensure_preview(&catalog, "missing", cache.path()).await,
            Err(Error::NotFound(_))
        ));
    }
}
