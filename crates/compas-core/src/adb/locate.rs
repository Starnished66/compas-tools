//! Finding adb: the user's own, or Google's platform-tools downloaded once.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use serde::Serialize;

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

/// Google's platform-tools archive for an operating system (`std::env::consts::OS`).
pub fn tools_url(os: &str) -> String {
    let name = match os {
        "windows" => "windows",
        "macos" => "darwin",
        _ => "linux",
    };
    format!("https://dl.google.com/android/repository/platform-tools-latest-{name}.zip")
}

/// Downloads platform-tools over HTTPS into `tools_dir`, unpacks them, and
/// runs `adb version` to make sure what came out is really adb.
pub async fn download_platform_tools(
    url: &str,
    tools_dir: &Path,
    on_progress: OnProgress<'_>,
) -> Result<Adb> {
    on_progress(Progress {
        phase: Phase::Downloading,
        done: 0,
        total: 1,
    });
    let archive = net::get_bytes(url, DOWNLOAD_LIMIT).await?;
    on_progress(Progress {
        phase: Phase::Downloading,
        done: 1,
        total: 1,
    });

    let target = tools_dir.join("platform-tools");
    // A half-unpacked earlier attempt must not be mistaken for a working install.
    let _ = fs::remove_dir_all(&target);
    let unpacked = unpack(&archive, tools_dir);
    let adb = Adb::new(target.join(exe_name()));
    let verified = match unpacked {
        Ok(()) => adb.version().await.and_then(|v| {
            v.contains("Android Debug Bridge")
                .then_some(())
                .ok_or_else(|| Error::Verification("the download is not adb".into()))
        }),
        Err(error) => Err(error),
    };
    if let Err(error) = verified {
        let _ = fs::remove_dir_all(&target);
        return Err(error);
    }
    Ok(adb)
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

    #[test]
    fn the_download_url_matches_the_system() {
        assert!(tools_url("linux").ends_with("platform-tools-latest-linux.zip"));
        assert!(tools_url("macos").ends_with("platform-tools-latest-darwin.zip"));
        assert!(tools_url("windows").ends_with("platform-tools-latest-windows.zip"));
        assert!(tools_url("linux").starts_with("https://dl.google.com/"));
    }

    #[tokio::test]
    async fn downloads_unpacks_and_checks_the_version() {
        let archive = zip_with(&[
            ("platform-tools/adb", FAKE_ADB, 0o755),
            ("platform-tools/NOTICE.txt", "notice", 0o644),
        ]);
        let server = TestServer::start([("/tools.zip", archive)]).await;
        let dir = tempfile::tempdir().unwrap();

        let adb = download_platform_tools(&server.url("/tools.zip"), dir.path(), &|_| {})
            .await
            .unwrap();
        assert!(adb
            .version()
            .await
            .unwrap()
            .contains("Android Debug Bridge"));
        assert_eq!(adb.exe(), dir.path().join("platform-tools/adb"));
        assert!(dir.path().join("platform-tools/NOTICE.txt").exists());
    }

    #[tokio::test]
    async fn something_that_is_not_adb_is_removed_again() {
        let archive = zip_with(&[("platform-tools/adb", "#!/bin/sh\necho hello\n", 0o755)]);
        let server = TestServer::start([("/tools.zip", archive)]).await;
        let dir = tempfile::tempdir().unwrap();

        let error = download_platform_tools(&server.url("/tools.zip"), dir.path(), &|_| {})
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Verification(_)));
        assert!(!dir.path().join("platform-tools").exists());
    }

    #[tokio::test]
    async fn paths_that_leave_the_folder_and_non_zips_are_refused() {
        let evil = zip_with(&[("../evil.txt", "x", 0o644)]);
        let server =
            TestServer::start([("/evil.zip", evil), ("/junk.zip", b"not a zip".to_vec())]).await;
        let dir = tempfile::tempdir().unwrap();
        for path in ["/evil.zip", "/junk.zip"] {
            let result = download_platform_tools(&server.url(path), dir.path(), &|_| {}).await;
            assert!(result.is_err(), "{path}");
        }
        assert!(!dir.path().parent().unwrap().join("evil.txt").exists());
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
