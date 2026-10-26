//! Putting a player release on a device over ADB: staging on the SD card,
//! recovery, and confirming the new version.

use std::path::Path;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::identify::{identify, DeviceInfo};
use super::{Adb, SD_ROOT};
use crate::card::Model;
use crate::error::{Error, IoContext, Result};
use crate::net::download_verified;
use crate::progress::{OnProgress, Phase, Progress};
use crate::releases::{Freshness, Kind, Release};

/// Slack kept free on the card beyond the file, as elsewhere in the app.
const MARGIN_KB: u64 = 2048;

#[derive(Debug, Clone, Copy)]
pub struct WaitOptions {
    /// How often to look for the device.
    pub poll: Duration,
    /// How long to wait for it to go away after the reboot command.
    pub going_down: Duration,
    /// How long to wait for it to come back. A full image takes minutes to flash.
    pub coming_back: Duration,
}

impl Default for WaitOptions {
    fn default() -> Self {
        Self {
            poll: Duration::from_secs(3),
            going_down: Duration::from_secs(60),
            coming_back: Duration::from_secs(15 * 60),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallReport {
    /// Where the file was put on the device.
    pub remote_path: String,
    /// The player after it came back; `None` if it did not within the time allowed, or if
    /// several players answered (see `candidates`).
    pub device: Option<DeviceInfo>,
    /// Several devices of the same board answered under new serials; the user picks.
    pub candidates: Vec<DeviceInfo>,
    /// How the version on the card compares with the release just installed.
    pub freshness: Freshness,
}

/// Updates and images must be on the card: `/tmp` and similar are RAM and
/// vanish at the reboot that installs them.
fn ensure_on_card(remote: &str) -> Result<()> {
    if remote.starts_with(&format!("{SD_ROOT}/")) {
        Ok(())
    } else {
        Err(Error::Rejected(format!(
            "{remote} is not on the player's storage"
        )))
    }
}

/// Free kilobytes on the card, from `df -k`. BusyBox wraps a long device name onto its own
/// line, so the figures are found by the `Use%` column rather than by line position.
fn parse_free_kb(df: &str) -> Option<u64> {
    let tokens: Vec<&str> = df.split_whitespace().collect();
    let percent = tokens.iter().position(|t| {
        t.strip_suffix('%')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    })?;
    tokens.get(percent.checked_sub(1)?)?.parse().ok()
}

/// Everything one install needs, so the steps below share it.
struct Job<'a> {
    adb: &'a Adb,
    serial: &'a str,
    model: Model,
    release: &'a Release,
    cache_dir: &'a Path,
    wait: &'a WaitOptions,
    on_progress: OnProgress<'a>,
}

impl Job<'_> {
    fn report(&self, phase: Phase, done: u64, total: u64) {
        (self.on_progress)(Progress { phase, done, total });
    }

    /// Downloads the release file, then puts it on the card and checks it there.
    async fn stage(&self, kind: Kind, remote: &str) -> Result<()> {
        ensure_on_card(remote)?;
        let file = self.release.file(kind, self.model)?;
        let dir = self.cache_dir.join(&self.release.tag);
        std::fs::create_dir_all(&dir).at(&dir)?;
        let local = dir.join(&file.name);
        download_verified(&file.url, &local, file.size, &file.sha256, |done, total| {
            self.report(Phase::Downloading, done, total);
        })
        .await?;

        let (adb, serial) = (self.adb, self.serial);
        let df = adb.shell(serial, &format!("df -k {SD_ROOT}")).await?;
        let free_kb = parse_free_kb(&df).ok_or_else(|| {
            Error::Adb(format!(
                "could not read the free space on the player, so nothing was copied (df said: {})",
                df.trim()
            ))
        })?;
        let needed_kb = file.size / 1024 + MARGIN_KB;
        if free_kb < needed_kb {
            return Err(Error::NoSpace {
                needed: needed_kb * 1024,
                available: free_kb * 1024,
            });
        }

        self.report(Phase::Pushing, 0, 1);
        adb.push(serial, &local, remote).await?;
        self.report(Phase::Verifying, 0, 1);
        let hash = adb
            .shell(serial, &format!("sha256sum {remote}"))
            .await
            .unwrap_or_default();
        let on_device = hash.split_whitespace().next().unwrap_or_default();
        let matches = if on_device.len() == 64 {
            on_device == file.sha256
        } else {
            // No sha256sum on the device: the size at least has to match.
            let bytes = adb
                .shell(serial, &format!("wc -c < {remote}"))
                .await
                .unwrap_or_default();
            bytes.trim().parse::<u64>().ok() == Some(file.size)
        };
        if !matches {
            let _ = adb.shell(serial, &format!("rm -f {remote}")).await;
            return Err(Error::Verification(format!(
                "{remote} on the device does not match the download"
            )));
        }
        self.report(Phase::Verifying, 1, 1);
        Ok(())
    }

    /// Reboots, waits for the device and compares its version with the release.
    async fn reboot_and_confirm(&self, remote: String) -> Result<InstallReport> {
        self.report(Phase::Rebooting, 0, 1);
        self.adb.shell(self.serial, "sync").await?;
        // The connection drops as the device goes down, so an error here is expected.
        let _ = self.adb.shell(self.serial, "reboot").await;
        self.report(Phase::Waiting, 0, 1);
        let returned = wait_for_return(self.adb, self.serial, self.model, self.wait).await;
        let (device, candidates) = match returned {
            Returned::Back(device) => (Some(device), Vec::new()),
            Returned::Candidates(found) => (None, found),
            Returned::TimedOut => (None, Vec::new()),
        };
        let freshness = self
            .release
            .freshness(device.as_ref().and_then(|d| d.player_version.as_deref()));
        Ok(InstallReport {
            remote_path: remote,
            device,
            candidates,
            freshness,
        })
    }
}

enum Returned {
    /// The same serial, or the only device of the same board under a new one.
    Back(DeviceInfo),
    /// Several look like the same board; the user must say which is theirs.
    Candidates(Vec<DeviceInfo>),
    TimedOut,
}

/// Waits for the device to disappear (it is rebooting) and to come back, then looks at it again.
/// Other connected devices are ignored: only the same serial counts, or, if the serial changed
/// after recovery, a player-looking device that identifies as the same board.
async fn wait_for_return(adb: &Adb, serial: &str, board: Model, wait: &WaitOptions) -> Returned {
    let online = || async {
        adb.devices()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|d| d.state == "device")
            .collect::<Vec<_>>()
    };
    let gone_by = Instant::now() + wait.going_down;
    while online().await.iter().any(|d| d.serial == serial) && Instant::now() < gone_by {
        tokio::time::sleep(wait.poll).await;
    }
    let give_up = Instant::now() + wait.coming_back;
    loop {
        let devices = online().await;
        if devices.iter().any(|d| d.serial == serial) {
            if let Ok(info) = identify(adb, serial).await {
                return Returned::Back(info);
            }
        } else {
            let mut same_board = Vec::new();
            for device in devices.iter().filter(|d| d.looks_like_player()) {
                if let Ok(info) = identify(adb, &device.serial).await {
                    if info.board == Some(board) {
                        same_board.push(info);
                    }
                }
            }
            match same_board.len() {
                0 => {}
                1 => return Returned::Back(same_board.remove(0)),
                _ => return Returned::Candidates(same_board),
            }
        }
        if Instant::now() >= give_up {
            return Returned::TimedOut;
        }
        tokio::time::sleep(wait.poll).await;
    }
}

/// Deletes an update image the install left on the card. Only a `.upt` at the card's
/// root is accepted, and the player does not remove it by itself.
pub async fn remove_staged_image(adb: &Adb, serial: &str, remote: &str) -> Result<()> {
    ensure_on_card(remote)?;
    let name = remote.trim_start_matches(SD_ROOT).trim_start_matches('/');
    if !name.to_ascii_lowercase().ends_with(".upt") || name.contains(['/', '\'', ' ', '"', '\\']) {
        return Err(Error::Rejected(format!(
            "{remote} is not an update image at the top of the player's storage"
        )));
    }
    adb.shell(serial, &format!("rm -f '{remote}'"))
        .await
        .map(drop)
}

/// What the install functions take besides the device.
pub struct InstallRequest<'a> {
    pub model: Model,
    pub release: &'a Release,
    /// Where downloaded release files are kept.
    pub cache_dir: &'a Path,
    pub wait: WaitOptions,
}

impl<'a> InstallRequest<'a> {
    fn job(&'a self, adb: &'a Adb, serial: &'a str, on_progress: OnProgress<'a>) -> Job<'a> {
        Job {
            adb,
            serial,
            model: self.model,
            release: self.release,
            cache_dir: self.cache_dir,
            wait: &self.wait,
            on_progress,
        }
    }
}

/// Full firmware image: staged at the top of the player's storage, installed from recovery.
pub async fn install_full_image(
    adb: &Adb,
    serial: &str,
    request: &InstallRequest<'_>,
    on_progress: OnProgress<'_>,
) -> Result<InstallReport> {
    let job = request.job(adb, serial, on_progress);
    let remote = format!("{SD_ROOT}/{}.upt", request.model.update_stem());
    job.stage(Kind::FullImage, &remote).await?;
    // Not confirmed on hardware yet; if it fails the user falls back to the key combination.
    adb.shell(serial, "/usr/bin/bootmode.sh Recovery").await?;
    job.reboot_and_confirm(remote).await
}

/// Player only: replaces `.compas/compas_player` on the card and restarts.
pub async fn install_player_only(
    adb: &Adb,
    serial: &str,
    request: &InstallRequest<'_>,
    on_progress: OnProgress<'_>,
) -> Result<InstallReport> {
    let job = request.job(adb, serial, on_progress);
    let remote = format!("{SD_ROOT}/.compas/compas_player");
    job.stage(Kind::PlayerOnly, &remote).await?;
    job.reboot_and_confirm(remote).await
}

#[cfg(all(test, unix))]
mod tests;
