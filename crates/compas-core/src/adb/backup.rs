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
use crate::error::{IoContext, Result};
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

/// How much of the player's settings and logs the copy holds. It is a diagnostic copy of text
/// files, never a full restore: themes and other binaries are not in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupQuality {
    /// The settings file and every readable file that is allowed.
    Complete,
    /// The settings file is there, but some files could not be copied.
    Partial,
    /// No settings were captured: this copy cannot stand in for anything.
    Unusable,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupReport {
    pub quality: BackupQuality,
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
    let mut settings_copied = false;
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
        settings_copied |= rel == ".compas/settings.txt";
        redacted += masked.count;
    }
    on_progress(Progress {
        phase: Phase::BackingUp,
        done: total,
        total,
    });

    let quality = if copied == 0 || !settings_copied {
        BackupQuality::Unusable
    } else if skipped.iter().any(|s| s.reason != SkipReason::Private) {
        BackupQuality::Partial
    } else {
        BackupQuality::Complete
    };
    Ok(BackupReport {
        quality,
        path: dir,
        files: copied,
        skipped,
        redacted,
    })
}

#[cfg(test)]
mod tests;
