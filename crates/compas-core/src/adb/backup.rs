//! A copy of the player's `/usr/data` before the first install, without anything secret.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::redact::{is_secret_name, redact};
use super::{Adb, USER_DATA};
use crate::error::{Error, IoContext, Result};
use crate::progress::{OnProgress, Phase, Progress};

/// Left out of every backup: it holds account tokens.
const EXCLUDED: [&str; 1] = [".compas/plugins"];
/// Larger files are not text settings.
const REDACT_LIMIT: u64 = 1 << 20;

#[derive(Debug, Clone, Serialize)]
pub struct BackupReport {
    pub path: PathBuf,
    pub files: u64,
    /// Paths (relative to `/usr/data`) that were not copied, and why they matter.
    pub skipped: Vec<String>,
    /// Places in copied text files where a secret was masked.
    pub redacted: usize,
}

#[derive(Debug)]
pub(super) struct PullStep {
    pub rel: String,
    pub files: u64,
}

#[derive(Default)]
struct Node {
    children: BTreeMap<String, Node>,
    is_file: bool,
    blocked: bool,
}

impl Node {
    fn files(&self) -> (u64, u64) {
        if self.is_file {
            return (1, u64::from(self.blocked));
        }
        self.children
            .values()
            .fold((0, 0), |(all, blocked), child| {
                let (a, b) = child.files();
                (all + a, blocked + b)
            })
    }
}

/// Whether a path (relative to `/usr/data`) must not be copied.
pub(super) fn is_left_out(rel: &str) -> bool {
    let excluded = EXCLUDED
        .iter()
        .any(|e| rel == *e || rel.starts_with(&format!("{e}/")));
    excluded || rel.split('/').any(is_secret_name)
}

/// Turns the device's file list into the fewest `adb pull`s that copy
/// everything allowed: a whole folder when nothing inside it is left out,
/// else its entries one by one.
pub(super) fn plan_pulls(files: &[String]) -> (Vec<PullStep>, Vec<String>) {
    let mut root = Node::default();
    for file in files {
        let mut node = &mut root;
        for part in file.split('/') {
            node = node.children.entry(part.to_string()).or_default();
        }
        node.is_file = true;
        node.blocked = is_left_out(file);
    }
    let (mut steps, mut skipped) = (Vec::new(), Vec::new());
    walk(&root, "", &mut steps, &mut skipped);
    (steps, skipped)
}

fn walk(node: &Node, rel: &str, steps: &mut Vec<PullStep>, skipped: &mut Vec<String>) {
    for (name, child) in &node.children {
        let path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let (all, blocked) = child.files();
        if blocked == 0 {
            steps.push(PullStep {
                rel: path,
                files: all,
            });
        } else if blocked == all {
            skipped.push(path);
        } else {
            walk(child, &path, steps, skipped);
        }
    }
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
    let listing = adb
        .shell(serial, &format!("find {USER_DATA} -type f 2>/dev/null"))
        .await?;
    let prefix = format!("{USER_DATA}/");
    let files: Vec<String> = listing
        .lines()
        .filter_map(|l| l.trim().strip_prefix(&prefix))
        .map(str::to_string)
        .collect();
    if files.is_empty() {
        return Err(Error::Adb(format!(
            "{USER_DATA} is empty or cannot be read"
        )));
    }

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let dir = backups_dir.join(format!("{}-{secs}", label(serial)));
    fs::create_dir_all(&dir).at(&dir)?;

    let (steps, skipped) = plan_pulls(&files);
    let total = steps.len() as u64;
    let mut copied = 0;
    for (i, step) in steps.iter().enumerate() {
        on_progress(Progress {
            phase: Phase::BackingUp,
            done: i as u64,
            total,
        });
        let local = dir.join(&step.rel);
        if let Some(parent) = local.parent() {
            fs::create_dir_all(parent).at(parent)?;
        }
        adb.pull(serial, &format!("{prefix}{}", step.rel), &local)
            .await?;
        copied += step.files;
    }
    on_progress(Progress {
        phase: Phase::BackingUp,
        done: total,
        total,
    });

    Ok(BackupReport {
        redacted: redact_text_files(&dir),
        path: dir,
        files: copied,
        skipped,
    })
}

/// Settings files may hold a PIN or Wi-Fi details even when their names do not
/// say so, so every small text file is passed through the redactor.
fn redact_text_files(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            total += redact_text_files(&path);
        } else if kind.is_file() && entry.metadata().is_ok_and(|m| m.len() <= REDACT_LIMIT) {
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let masked = redact(&text);
            if masked.count > 0 && fs::write(&path, masked.text).is_ok() {
                total += masked.count;
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| (*p).to_string()).collect()
    }

    #[test]
    fn plugins_and_secret_names_are_left_out() {
        for path in [
            ".compas/plugins/lastfm.json",
            ".compas/plugins",
            "wpa_supplicant.conf",
            "a/b/remote_pin",
            "wifi/known.cfg",
        ] {
            assert!(is_left_out(path), "{path}");
        }
        for path in [
            ".compas/settings.ini",
            ".compas/pluginsx/a",
            "player_crash.log",
            "a/mapping.json",
        ] {
            assert!(!is_left_out(path), "{path}");
        }
    }

    #[test]
    fn clean_folders_go_in_one_pull_and_mixed_ones_entry_by_entry() {
        let files = list(&[
            "player_crash.log",
            ".compas/settings.ini",
            ".compas/plugins/token.json",
            ".compas/plugins/other.json",
            "wpa_supplicant.conf",
            "music/db/a.bin",
            "music/db/b.bin",
            "mixed/keep.txt",
            "mixed/inner/remote_pin",
        ]);
        let (steps, skipped) = plan_pulls(&files);
        let pulled: Vec<&str> = steps.iter().map(|s| s.rel.as_str()).collect();
        assert_eq!(
            pulled,
            [
                ".compas/settings.ini",
                "mixed/keep.txt",
                "music",
                "player_crash.log"
            ]
        );
        assert_eq!(
            skipped,
            [".compas/plugins", "mixed/inner", "wpa_supplicant.conf"]
        );
        assert_eq!(steps.iter().find(|s| s.rel == "music").unwrap().files, 2);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_backup_leaves_out_plugins_and_secrets_and_masks_settings() {
        use super::super::fake::{FakeAdb, PS_STOCK};

        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "ingenic_1", "480,800", PS_STOCK);
        fake.write_remote("/usr/data/player_crash.log", "crash\n");
        fake.write_remote(
            "/usr/data/.compas/settings.ini",
            "volume=12\nremote_control_pin=424242\nname=My Player\n",
        );
        fake.write_remote(
            "/usr/data/.compas/plugins/lastfm.json",
            "{\"session_key\": \"SECRET-ACCOUNT-TOKEN\"}",
        );
        fake.write_remote(
            "/usr/data/wpa_supplicant.conf",
            "network={ ssid=\"Home\" psk=\"hunter2\" }",
        );
        fake.write_remote("/usr/data/db/index.bin", "binary-ish");

        let out = tempfile::tempdir().unwrap();
        let report = backup_user_data(&fake.adb(), "ingenic_1", out.path(), &|_| {})
            .await
            .unwrap();

        let root = &report.path;
        assert!(root.starts_with(out.path()));
        assert!(root.join("player_crash.log").exists());
        assert!(root.join("db/index.bin").exists());
        assert!(
            !root.join(".compas/plugins").exists(),
            "account tokens stay on the device"
        );
        assert!(
            !root.join("wpa_supplicant.conf").exists(),
            "Wi-Fi details stay on the device"
        );
        assert_eq!(report.skipped, [".compas/plugins", "wpa_supplicant.conf"]);

        let settings = std::fs::read_to_string(root.join(".compas/settings.ini")).unwrap();
        assert!(settings.contains("volume=12") && settings.contains("name=My Player"));
        assert!(
            !settings.contains("424242"),
            "the remote-control PIN is not copied"
        );
        assert_eq!(report.redacted, 1);
        assert_eq!(report.files, 3);

        // Nothing the user must not share is anywhere in the backup, in any file.
        let calls = fake.calls().join("\n");
        assert!(
            !calls.contains("lastfm")
                && !calls.contains("wpa_supplicant")
                && !calls.contains("424242")
        );
    }
}
