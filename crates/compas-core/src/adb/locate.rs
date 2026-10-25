//! Finding adb: the user's own, or Google's platform-tools downloaded once.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::Adb;
use crate::error::{Error, IoContext, Result};
use crate::net;
use crate::progress::{OnProgress, Phase, Progress};

/// Overrides the search; this is also how tests and demos point the app at a fake adb.
pub const ENV_OVERRIDE: &str = "COMPAS_ADB";
/// The platform-tools archive is about 15 MB.
const DOWNLOAD_LIMIT: usize = 200 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolsSource {
    /// `COMPAS_ADB`.
    Override,
    /// Found on the user's `PATH`.
    System,
    /// Downloaded by this app into its data folder.
    Downloaded,
}

#[derive(Debug, Clone)]
pub struct Located {
    pub adb: Adb,
    pub source: ToolsSource,
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "adb.exe"
    } else {
        "adb"
    }
}

fn find_on_path() -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(exe_name()))
        .find(|candidate| candidate.is_file())
}

/// A system adb when there is one, else the one this app downloaded into `tools_dir`.
pub fn locate(tools_dir: &Path) -> Option<Located> {
    locate_in(tools_dir, std::env::var_os(ENV_OVERRIDE), find_on_path())
}

fn locate_in(
    tools_dir: &Path,
    override_path: Option<std::ffi::OsString>,
    on_path: Option<PathBuf>,
) -> Option<Located> {
    if let Some(path) = override_path {
        return Some(Located {
            adb: Adb::new(path),
            source: ToolsSource::Override,
        });
    }
    if let Some(path) = on_path {
        return Some(Located {
            adb: Adb::new(path),
            source: ToolsSource::System,
        });
    }
    let downloaded = tools_dir.join("platform-tools").join(exe_name());
    downloaded.is_file().then(|| Located {
        adb: Adb::new(downloaded),
        source: ToolsSource::Downloaded,
    })
}

/// The platform-tools release this app downloads. Pinned, so what is installed is always
/// a build somebody has looked at, not whatever "latest" is today.
pub const PLATFORM_TOOLS_VERSION: &str = "37.0.1";

/// A pinned platform-tools archive. The size and SHA-256 are those of the file served by
/// `dl.google.com`; the size and SHA-1 also match Google's published repository manifest
/// (`repository2-3.xml`), which is how the hashes were checked.
#[derive(Debug, Clone)]
pub struct ToolsArchive {
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

/// The pinned archive for an operating system (`std::env::consts::OS`).
pub fn tools_archive(os: &str) -> ToolsArchive {
    let (name, size, sha256) = match os {
        "windows" => (
            "win",
            8_044_989,
            "45f4d63113e895ebde0c90f194099a4676b6ac653bd28d54314a9e022bbc1a99",
        ),
        "macos" => (
            "darwin",
            16_110_554,
            "ee39ad5967e95c2a07f04dbcbde96b1a0c916ba376096db5d2f498b7727a5d1d",
        ),
        _ => (
            "linux",
            9_054_187,
            "d230f13842f60f782a8645f9c813f8f845bf36089ea7289f28c48f17979313f1",
        ),
    };
    ToolsArchive {
        url: format!(
            "https://dl.google.com/android/repository/platform-tools_r{PLATFORM_TOOLS_VERSION}-{name}.zip"
        ),
        size,
        sha256: sha256.to_string(),
    }
}

/// Downloads the pinned platform-tools over HTTPS and checks their size and SHA-256 before
/// anything is unpacked. They are unpacked beside the old copy and tested with `adb version`;
/// only then does the new copy replace it, so a failed update leaves the old one working.
pub async fn download_platform_tools(
    archive: &ToolsArchive,
    tools_dir: &Path,
    on_progress: OnProgress<'_>,
) -> Result<Adb> {
    on_progress(Progress {
        phase: Phase::Downloading,
        done: 0,
        total: 1,
    });
    let bytes = net::get_bytes(&archive.url, DOWNLOAD_LIMIT).await?;
    on_progress(Progress {
        phase: Phase::Downloading,
        done: 1,
        total: 1,
    });

    let hash = format!("{:x}", Sha256::digest(&bytes));
    if bytes.len() as u64 != archive.size || hash != archive.sha256 {
        return Err(Error::Verification(
            "the platform-tools download does not match its published checksum".into(),
        ));
    }

    let staging = tools_dir.join(".platform-tools-new");
    let _ = fs::remove_dir_all(&staging);
    let installed = install_staged(&bytes, &staging, tools_dir).await;
    let _ = fs::remove_dir_all(&staging);
    installed
}

async fn install_staged(bytes: &[u8], staging: &Path, tools_dir: &Path) -> Result<Adb> {
    unpack(bytes, staging)?;
    let fresh = staging.join("platform-tools");
    let probe = Adb::new(fresh.join(exe_name()));
    let version = probe.version().await?;
    if !version.contains("Android Debug Bridge") {
        return Err(Error::Verification("the download is not adb".into()));
    }

    let target = tools_dir.join("platform-tools");
    let old = tools_dir.join(".platform-tools-old");
    let _ = fs::remove_dir_all(&old);
    let had_old = target.exists();
    if had_old {
        fs::rename(&target, &old).at(&target)?;
    }
    if let Err(error) = fs::rename(&fresh, &target) {
        if had_old {
            let _ = fs::rename(&old, &target);
        }
        return Err(Error::IoAt {
            path: target,
            source: error,
        });
    }
    let _ = fs::remove_dir_all(&old);
    Ok(Adb::new(target.join(exe_name())))
}

/// Extracts the archive below `dir`, refusing entries that would leave it.
fn unpack(archive: &[u8], dir: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))
        .map_err(|e| Error::Verification(format!("the download is not a zip: {e}")))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| Error::Verification(e.to_string()))?;
        let Some(relative) = entry.enclosed_name() else {
            return Err(Error::Verification("the archive has an unsafe path".into()));
        };
        let path = dir.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&path).at(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).at(parent)?;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).at(&path)?;
        fs::write(&path, bytes).at(&path)?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).at(&path)?;
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::test_support::TestServer;

    fn zip_with(entries: &[(&str, &str, u32)]) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut out);
            for (name, content, mode) in entries {
                let options = zip::write::SimpleFileOptions::default().unix_permissions(*mode);
                zip.start_file(*name, options).unwrap();
                zip.write_all(content.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        out.into_inner()
    }

    const FAKE_ADB: &str = "#!/bin/sh\necho 'Android Debug Bridge version 1.0.41'\n";

    fn archive(server: &TestServer, bytes: &[u8]) -> ToolsArchive {
        ToolsArchive {
            url: server.url("/tools.zip"),
            size: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }

    #[test]
    fn the_pinned_archives_are_versioned_https_downloads_with_checksums() {
        for (os, name) in [("linux", "linux"), ("macos", "darwin"), ("windows", "win")] {
            let archive = tools_archive(os);
            assert_eq!(
                archive.url,
                format!("https://dl.google.com/android/repository/platform-tools_r{PLATFORM_TOOLS_VERSION}-{name}.zip")
            );
            assert_eq!(archive.sha256.len(), 64);
            assert!(archive.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(archive.size > 1_000_000);
        }
    }

    #[tokio::test]
    async fn downloads_verifies_unpacks_and_checks_the_version() {
        let bytes = zip_with(&[
            ("platform-tools/adb", FAKE_ADB, 0o755),
            ("platform-tools/NOTICE.txt", "notice", 0o644),
        ]);
        let server = TestServer::start([("/tools.zip", bytes.clone())]).await;
        let dir = tempfile::tempdir().unwrap();

        let adb = download_platform_tools(&archive(&server, &bytes), dir.path(), &|_| {})
            .await
            .unwrap();
        assert!(adb
            .version()
            .await
            .unwrap()
            .contains("Android Debug Bridge"));
        assert_eq!(adb.exe(), dir.path().join("platform-tools/adb"));
        assert!(dir.path().join("platform-tools/NOTICE.txt").exists());
        assert!(!dir.path().join(".platform-tools-new").exists());
    }

    #[tokio::test]
    async fn a_wrong_checksum_or_size_is_refused_before_anything_is_unpacked() {
        let bytes = zip_with(&[("platform-tools/adb", FAKE_ADB, 0o755)]);
        let server = TestServer::start([("/tools.zip", bytes.clone())]).await;
        let dir = tempfile::tempdir().unwrap();

        let mut wrong_hash = archive(&server, &bytes);
        wrong_hash.sha256 = "0".repeat(64);
        let mut wrong_size = archive(&server, &bytes);
        wrong_size.size += 1;
        for bad in [wrong_hash, wrong_size] {
            let error = download_platform_tools(&bad, dir.path(), &|_| {})
                .await
                .unwrap_err();
            assert!(matches!(error, Error::Verification(_)));
        }
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "nothing unpacked"
        );
    }

    #[tokio::test]
    async fn an_existing_copy_is_replaced_only_by_a_verified_one() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("platform-tools")).unwrap();
        std::fs::write(dir.path().join("platform-tools/marker.txt"), "old copy").unwrap();

        // Something that is not adb: the old copy stays.
        let junk = zip_with(&[("platform-tools/adb", "#!/bin/sh\necho hello\n", 0o755)]);
        let server = TestServer::start([("/tools.zip", junk.clone())]).await;
        let error = download_platform_tools(&archive(&server, &junk), dir.path(), &|_| {})
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Verification(_)));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("platform-tools/marker.txt")).unwrap(),
            "old copy"
        );

        // A good one replaces it.
        let good = zip_with(&[("platform-tools/adb", FAKE_ADB, 0o755)]);
        let server = TestServer::start([("/tools.zip", good.clone())]).await;
        download_platform_tools(&archive(&server, &good), dir.path(), &|_| {})
            .await
            .unwrap();
        assert!(!dir.path().join("platform-tools/marker.txt").exists());
        assert!(dir.path().join("platform-tools/adb").exists());
        assert!(!dir.path().join(".platform-tools-old").exists());
    }

    #[tokio::test]
    async fn paths_that_leave_the_folder_and_non_zips_are_refused_even_with_a_matching_hash() {
        let evil = zip_with(&[("../evil.txt", "x", 0o644)]);
        let junk = b"not a zip".to_vec();
        let dir = tempfile::tempdir().unwrap();
        for bytes in [evil, junk] {
            let server = TestServer::start([("/tools.zip", bytes.clone())]).await;
            let result =
                download_platform_tools(&archive(&server, &bytes), dir.path(), &|_| {}).await;
            assert!(result.is_err());
        }
        assert!(!dir.path().parent().unwrap().join("evil.txt").exists());
        assert!(!dir.path().join("platform-tools").exists());
    }

    #[test]
    fn a_downloaded_adb_is_found_when_none_is_on_the_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("platform-tools")).unwrap();
        std::fs::write(dir.path().join("platform-tools/adb"), "x").unwrap();
        let found = locate_in(dir.path(), None, None).unwrap();
        assert_eq!(found.source, ToolsSource::Downloaded);
        assert!(locate_in(tempfile::tempdir().unwrap().path(), None, None).is_none());
    }

    #[test]
    fn an_override_beats_the_path_which_beats_a_download() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("platform-tools")).unwrap();
        std::fs::write(dir.path().join("platform-tools/adb"), "x").unwrap();

        let both = locate_in(
            dir.path(),
            Some("/fake/adb".into()),
            Some("/usr/bin/adb".into()),
        )
        .unwrap();
        assert_eq!(
            (both.source, both.adb.exe()),
            (ToolsSource::Override, Path::new("/fake/adb"))
        );
        let on_path = locate_in(dir.path(), None, Some("/usr/bin/adb".into())).unwrap();
        assert_eq!(on_path.source, ToolsSource::System);
    }
}
