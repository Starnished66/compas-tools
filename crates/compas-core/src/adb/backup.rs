//! A copy of the player's own settings before the first install, with nothing secret in it.
//!
//! Only an allowlist of `/usr/data` entries is read, file by file, and every file is masked
//! in memory before it is written, so nothing sensitive ever reaches the computer's disk.
//! The SD card is mounted below `/usr/data/mnt` and is never part of this.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::redact::{is_secret_name, redact};
use super::{Adb, USER_DATA};
use crate::error::{Error, IoContext, Result};
use crate::progress::{OnProgress, Phase, Progress};

/// What a backup may contain, from the player's own sources
/// (`storage_paths.h`, `assets.c`, `main.c`): settings, EQ, library ratings, theme
/// overrides, and the player's logs. Paths are relative to `/usr/data`. The hostname
/// override is left out: a name like "Maria's player" identifies its owner.
const ALLOWED: [&str; 14] = [
    ".compas/settings.txt",
    ".compas/peq.txt",
    ".compas/remote_state.tsv",
    ".compas/charge_baseline.txt",
    ".compas/usb_dac_session.txt",
    "theme_overrides",
    "player_crash.log",
    "player_crash.log.1",
    "player_exit.log",
    "player_exit.log.1",
    "bootloader_boot.log",
    "boot_debug.log",
    "sd_fsck.log",
    "selector-script.log",
];

/// Never copied, whatever the allowlist says. `.compas/plugins` holds plugin account
/// tokens (`plugin.secrets`), `subsonic_servers.tsv` has server passwords, `pin.conf`
/// the Bluetooth PIN, and `mnt` is the SD card.
const NEVER: [&str; 4] = [
    "mnt",
    ".compas/plugins",
    ".compas/subsonic_servers.tsv",
    "pin.conf",
];

/// What the device prints for a file it cannot read.
const UNREADABLE: &str = "__COMPAS_UNREADABLE__";

/// Larger files are not settings.
const MAX_BYTES: usize = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// Credentials or private data.
    Private,
    TooLarge,
    /// Cannot be masked, so it is not copied.
    Binary,
    Unreadable,
    /// The name has a character that cannot be quoted for the device, so it is not fetched.
    UnsupportedName,
}

#[derive(Debug, Clone, Serialize)]
pub struct Skipped {
    pub path: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupReport {
    pub path: PathBuf,
    pub files: u64,
    pub skipped: Vec<Skipped>,
    /// Places in the copied files where a secret was masked.
    pub redacted: usize,
}

/// Whether a path (relative to `/usr/data`) must not be copied.
pub(super) fn is_left_out(rel: &str) -> bool {
    let never = NEVER
        .iter()
        .any(|n| rel == *n || rel.starts_with(&format!("{n}/")));
    never || rel.split('/').any(is_secret_name)
}

/// A device-supplied path that stays below the backup folder and can be quoted safely.
fn is_safe_relative(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.contains(['\\', '\'', '\n', '\0'])
        && rel.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

fn label(serial: &str) -> String {
    serial
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub async fn backup_user_data(
    adb: &Adb,
    serial: &str,
    backups_dir: &Path,
    on_progress: OnProgress<'_>,
) -> Result<BackupReport> {
    // `find` without `-L`: links are never followed, and `-type f` skips them.
    let roots: Vec<String> = ALLOWED
        .iter()
        .map(|e| format!("'{USER_DATA}/{e}'"))
        .collect();
    let listing = adb
        .shell(
            serial,
            &format!("find {} -type f 2>/dev/null", roots.join(" ")),
        )
        .await?;
    let prefix = format!("{USER_DATA}/");
    let mut files: Vec<String> = listing
        .lines()
        .filter_map(|l| l.trim().strip_prefix(&prefix))
        .map(str::to_string)
        .collect();
    files.sort();
    files.dedup();

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let dir = backups_dir.join(format!("{}-{secs}", label(serial)));
    fs::create_dir_all(&dir).at(&dir)?;

    let (mut copied, mut redacted, mut skipped) = (0, 0, Vec::new());
    let total = files.len() as u64;
    for (i, rel) in files.iter().enumerate() {
        on_progress(Progress {
            phase: Phase::BackingUp,
            done: i as u64,
            total,
        });
        let skip = |reason| Skipped {
            path: rel.clone(),
            reason,
        };
        if is_left_out(rel) {
            skipped.push(skip(SkipReason::Private));
            continue;
        }
        if !is_safe_relative(rel) {
            skipped.push(skip(SkipReason::UnsupportedName));
            continue;
        }
        // adb's exit status is not reliable, so the shell says it in-band.
        let path = format!("'{USER_DATA}/{rel}'");
        let command = format!(
            "if [ -r {path} ] && head -c {} {path}; then :; else echo {UNREADABLE}; fi",
            MAX_BYTES + 1
        );
        let bytes = match adb.exec_out(serial, &command).await {
            // A read that fails half-way prints the marker after the partial content.
            Ok(bytes)
                if !bytes
                    .windows(UNREADABLE.len())
                    .any(|w| w == UNREADABLE.as_bytes()) =>
            {
                bytes
            }
            _ => {
                skipped.push(skip(SkipReason::Unreadable));
                continue;
            }
        };
        if bytes.len() > MAX_BYTES {
            skipped.push(skip(SkipReason::TooLarge));
            continue;
        }
        let Some(text) = std::str::from_utf8(&bytes)
            .ok()
            .filter(|t| !t.contains('\0'))
        else {
            skipped.push(skip(SkipReason::Binary));
            continue;
        };
        let masked = redact(text);
        let target = dir.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).at(parent)?;
        }
        fs::write(&target, masked.text).at(&target)?;
        copied += 1;
        redacted += masked.count;
    }
    on_progress(Progress {
        phase: Phase::BackingUp,
        done: total,
        total,
    });

    if files.is_empty() {
        return Err(Error::Adb(format!(
            "none of the expected files were found in {USER_DATA}"
        )));
    }
    Ok(BackupReport {
        path: dir,
        files: copied,
        skipped,
        redacted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_the_sd_card_and_plugins_are_always_left_out() {
        for path in [
            ".compas/plugins/lastfm.json",
            ".compas/plugins",
            ".compas/subsonic_servers.tsv",
            "mnt/sd_0/Music/a.flac",
            "mnt",
            "pin.conf",
            "wpa_supplicant.conf",
            "a/b/remote_pin",
            "wifi/known.cfg",
        ] {
            assert!(is_left_out(path), "{path}");
        }
        for path in [
            ".compas/settings.txt",
            ".compas/pluginsx/a",
            "player_crash.log",
            "theme_overrides/a.png",
        ] {
            assert!(!is_left_out(path), "{path}");
        }
    }

    #[test]
    fn device_supplied_paths_cannot_escape_or_break_quoting() {
        for bad in ["../x", "a/../b", "/abs", "a//b", "it's", "a\\b", "a\nb", ""] {
            assert!(!is_safe_relative(bad), "{bad:?}");
        }
        assert!(is_safe_relative("theme_overrides/icons/play.png"));
    }

    #[test]
    fn the_allowlist_never_names_anything_that_is_always_excluded() {
        for entry in ALLOWED {
            assert!(!is_left_out(entry), "{entry}");
        }
    }

    #[cfg(unix)]
    mod with_fake_adb {
        use super::super::super::fake::{FakeAdb, PS_COMPAS, PS_STOCK};
        use super::super::*;

        fn fixture(dir: &Path) -> FakeAdb {
            let fake = FakeAdb::new(dir, "ingenic_1", "480,800", PS_STOCK);
            fake.write_remote(
                "/usr/data/player_crash.log",
                "crash\nremote_control_pin=424242\n",
            );
            fake.write_remote(
                "/usr/data/.compas/settings.txt",
                "volume=12\nremote_control_pin=424242\nsubsonic_password=hunter2\nsubsonic_saved_0_password=hunter3\nwifi_last=Casa Garita 5G\nname=My Player\n",
            );
            fake.write_remote(
                "/usr/data/.compas/subsonic_servers.tsv",
                "https://x\tana\tPASSWORD-IN-TSV\t1\n",
            );
            fake.write_remote(
                "/usr/data/.compas/plugins/lastfm.json",
                "{\"session_key\": \"ACCOUNT-TOKEN\"}",
            );
            fake.write_remote("/usr/data/.compas/queue.bin", "text");
            fake.write_remote("/usr/data/mnt/sd_0/Music/song.flac", "SD-CARD-AUDIO");
            fake.write_remote("/usr/data/mnt/sd_0/.compas/device.json", "{}");
            fake.write_remote("/usr/data/pin.conf", "* 0000");
            fake.write_remote("/usr/data/wpa_supplicant.conf", "psk=\"WIFI-SECRET\"");
            fake.write_remote("/usr/data/theme_overrides/colors.txt", "accent=blue\n");
            fake.write_remote("/usr/data/theme_overrides/icon.bin", "x");
            std::fs::write(
                fake.remote("/usr/data/theme_overrides/icon.bin"),
                [0u8, 159, 146, 150],
            )
            .unwrap();
            std::fs::write(fake.remote("/usr/data/big.log"), vec![b'a'; 3 << 20]).unwrap();
            std::fs::write(
                fake.remote("/usr/data/player_exit.log"),
                vec![b'a'; 2 << 20],
            )
            .unwrap();
            fake
        }

        #[tokio::test]
        async fn only_the_allowlist_is_read_and_everything_is_masked_before_it_is_written() {
            let dir = tempfile::tempdir().unwrap();
            let fake = fixture(dir.path());
            let out = tempfile::tempdir().unwrap();
            let report = backup_user_data(&fake.adb(), "ingenic_1", out.path(), &|_| {})
                .await
                .unwrap();
            let root = &report.path;

            // What is in the backup, and nothing else.
            let settings = std::fs::read_to_string(root.join(".compas/settings.txt")).unwrap();
            assert!(settings.contains("volume=12") && settings.contains("name=My Player"));
            for secret in ["424242", "hunter2", "hunter3", "Casa Garita"] {
                assert!(!settings.contains(secret), "{secret} reached the disk");
            }
            assert!(std::fs::read_to_string(root.join("player_crash.log"))
                .unwrap()
                .contains("<redacted>"));
            assert!(root.join("theme_overrides/colors.txt").exists());

            // What is not: the SD card, credentials, plugins, binaries and big files.
            for path in [
                ".compas/subsonic_servers.tsv",
                ".compas/plugins",
                "mnt",
                "pin.conf",
                "wpa_supplicant.conf",
                "big.log",
                "theme_overrides/icon.bin",
                "player_exit.log",
            ] {
                assert!(!root.join(path).exists(), "{path} was copied");
            }
            let reason = |path: &str| {
                report
                    .skipped
                    .iter()
                    .find(|s| s.path == path)
                    .map(|s| s.reason)
            };
            assert_eq!(reason("theme_overrides/icon.bin"), Some(SkipReason::Binary));
            assert_eq!(reason("player_exit.log"), Some(SkipReason::TooLarge));
            assert_eq!(report.files, 3);
            assert_eq!(report.redacted, 5);

            // The device was never asked for anything outside the allowlist.
            let calls = fake.calls().join("\n");
            for forbidden in [
                "mnt/sd_0",
                "subsonic_servers",
                "plugins",
                "pin.conf",
                "wpa_supplicant",
                " pull ",
            ] {
                assert!(!calls.contains(forbidden), "{forbidden} in\n{calls}");
            }
        }

        /// v1.2.0 keeps only some of the allowlisted files in `/usr/data` and has no device.json.
        #[tokio::test]
        async fn a_v1_2_0_layout_with_few_files_backs_up() {
            let dir = tempfile::tempdir().unwrap();
            let fake = FakeAdb::new(dir.path(), "ingenic_1", "480,800", PS_COMPAS);
            fake.write_remote("/usr/data/.compas/settings.txt", "volume=7\nname=Old\n");
            fake.write_remote("/usr/data/player_exit.log", "exit 0\n");
            fake.write_remote("/usr/data/.compas/plugins/x.json", "{\"token\":\"T\"}");
            let out = tempfile::tempdir().unwrap();
            let report = backup_user_data(&fake.adb(), "ingenic_1", out.path(), &|_| {})
                .await
                .unwrap();
            assert_eq!(report.files, 2);
            assert!(report.path.join(".compas/settings.txt").exists());
            assert!(!report.path.join(".compas/plugins").exists());
        }

        #[tokio::test]
        async fn a_linked_entry_is_not_followed_off_the_allowlist() {
            let dir = tempfile::tempdir().unwrap();
            let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_STOCK);
            fake.write_remote("/usr/data/mnt/sd_0/secret.txt", "SD-ONLY");
            fake.write_remote("/usr/data/player_crash.log", "ok\n");
            std::fs::create_dir_all(fake.remote("/usr/data")).unwrap();
            std::os::unix::fs::symlink(
                fake.remote("/usr/data/mnt/sd_0"),
                fake.remote("/usr/data/theme_overrides"),
            )
            .unwrap();
            std::os::unix::fs::symlink(
                fake.remote("/usr/data/mnt/sd_0/secret.txt"),
                fake.remote("/usr/data/player_exit.log"),
            )
            .unwrap();

            let out = tempfile::tempdir().unwrap();
            let report = backup_user_data(&fake.adb(), "s1", out.path(), &|_| {})
                .await
                .unwrap();
            assert_eq!(report.files, 1);
            assert!(!report.path.join("theme_overrides").exists());
            assert!(!report.path.join("player_exit.log").exists());
        }

        #[tokio::test]
        async fn unreadable_files_empty_files_and_odd_names_get_their_own_reasons() {
            use std::os::unix::fs::PermissionsExt;

            let dir = tempfile::tempdir().unwrap();
            let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_STOCK);
            fake.write_remote("/usr/data/player_crash.log", "locked\n");
            fake.write_remote("/usr/data/player_exit.log", "");
            fake.write_remote("/usr/data/theme_overrides/it's.txt", "quote in the name\n");
            fake.write_remote("/usr/data/boot_debug.log", "fine\n");
            let locked = fake.remote("/usr/data/player_crash.log");
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

            let out = tempfile::tempdir().unwrap();
            let report = backup_user_data(&fake.adb(), "s1", out.path(), &|_| {})
                .await
                .unwrap();
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();

            let reason = |path: &str| {
                report
                    .skipped
                    .iter()
                    .find(|s| s.path == path)
                    .map(|s| s.reason)
            };
            if std::fs::read(&locked).is_ok() && reason("player_crash.log").is_none() {
                return; // running as root: permissions do not apply
            }
            assert_eq!(reason("player_crash.log"), Some(SkipReason::Unreadable));
            assert_eq!(
                reason("theme_overrides/it's.txt"),
                Some(SkipReason::UnsupportedName)
            );
            assert!(report.path.join("boot_debug.log").exists());
            assert_eq!(
                std::fs::read(report.path.join("player_exit.log")).unwrap(),
                b"",
                "a real empty file is copied"
            );
            assert!(reason("player_exit.log").is_none());
        }
    }
}
