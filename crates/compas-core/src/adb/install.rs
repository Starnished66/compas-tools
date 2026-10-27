//! Putting a player release on a device over ADB: staging on the SD card,
//! recovery, and confirming the new version.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use super::identify::{identify, DeviceInfo, Firmware};
use super::{Adb, SD_ROOT};
use crate::card::Model;
use crate::error::{Error, IoContext, Result};
use crate::net::download_verified;
use crate::progress::{OnProgress, Phase, Progress};
use crate::releases::{Freshness, Kind, Release};

mod card;
mod wait;
use card::{is_plain_name, recovery_partition_ok, shell_quote, verify_card, RECOVERY_FLAG};
use wait::{wait_for_return, Returned};

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
    /// The device was seen restarting: its boot id changed.
    pub rebooted: bool,
    /// Restarted, the same board, and running the release's version. Only then is the
    /// install done; otherwise it is "not confirmed" and the update file is kept.
    pub confirmed: bool,
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

/// Everything one install needs, so the steps below share it. `model` is the board the
/// connected device itself reported, never a choice made elsewhere.
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

    /// Downloads the release file, then puts it on the card under a name of this job's own,
    /// checks its SHA-256 there and only then renames it into place.
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
        // The download can take long enough for the card to have been taken out.
        let free_kb = verify_card(adb, serial).await?;
        let needed_kb = file.size / 1024 + MARGIN_KB;
        if free_kb < needed_kb {
            return Err(Error::NoSpace {
                needed: needed_kb * 1024,
                available: free_kb * 1024,
            });
        }
        let (parent, name) = remote.rsplit_once('/').unwrap_or((SD_ROOT, remote));
        if parent != SD_ROOT {
            // A linked folder could lead off the card.
            let made = adb
                .shell(
                    serial,
                    &format!(
                        "if [ -L {parent} ]; then echo link; else mkdir -p {parent} && echo ok; fi"
                    ),
                )
                .await?;
            if made.trim() != "ok" {
                return Err(Error::Rejected(format!(
                    "{parent} is not a plain folder on the SD card"
                )));
            }
        }
        if !is_plain_name(name) {
            return Err(Error::Rejected(format!("{name} is not a safe file name")));
        }
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let temp = format!("{parent}/.{name}.{}-{millis}.part", std::process::id());

        self.report(Phase::Pushing, 0, 1);
        let pushed = adb.push(serial, &local, &temp).await;
        let checked = match pushed {
            Ok(()) => {
                self.report(Phase::Verifying, 0, 1);
                self.matches_download(&temp, &file.sha256, &dir).await
            }
            Err(e) => Err(e),
        };
        if let Err(error) = checked {
            // Only this job's partial file; an earlier staged update is left alone.
            let _ = adb.shell(serial, &format!("rm -f {temp}")).await;
            return Err(error);
        }
        let moved = async {
            verify_card(adb, serial).await?;
            // adb's exit status is not reliable here, so the shell says it in-band.
            let said = adb
                .shell(serial, &format!("mv -f {temp} {remote} && sync && echo ok"))
                .await?;
            if said.trim() != "ok" {
                return Err(Error::Verification(format!(
                    "{remote} could not be put in place on the device"
                )));
            }
            // The temporary name is gone and the file under its final name is the download.
            let gone = adb
                .shell(
                    serial,
                    &format!("if [ -e {temp} ]; then echo present; else echo gone; fi"),
                )
                .await?;
            if gone.trim() != "gone" {
                return Err(Error::Verification(format!(
                    "{temp} is still on the device after the rename"
                )));
            }
            self.matches_download(remote, &file.sha256, &dir).await
        }
        .await;
        if let Err(error) = moved {
            let _ = adb.shell(serial, &format!("rm -f {temp}")).await;
            return Err(error);
        }
        self.report(Phase::Verifying, 1, 1);
        Ok(())
    }

    /// The staged file has the downloaded content: the device's `sha256sum`, or, without one,
    /// the file pulled back and hashed here. A byte count alone never counts.
    async fn matches_download(&self, remote: &str, sha256: &str, scratch: &Path) -> Result<()> {
        let (adb, serial) = (self.adb, self.serial);
        let out = adb
            .shell(serial, &format!("sha256sum {remote}"))
            .await
            .unwrap_or_default();
        let on_device = out.split_whitespace().next().unwrap_or_default();
        let digest = if on_device.len() == 64 {
            on_device.to_ascii_lowercase()
        } else {
            let back = scratch.join(format!("pullback-{}.tmp", std::process::id()));
            let pulled = adb.pull(serial, remote, &back).await;
            let hashed = pulled.and_then(|()| crate::fs::sha256_file(&back));
            let _ = std::fs::remove_file(&back);
            hashed?
        };
        if digest == sha256.to_ascii_lowercase() {
            Ok(())
        } else {
            Err(Error::Verification(format!(
                "{remote} on the device does not match the download"
            )))
        }
    }

    /// The player's own contract before a recovery reboot (`firmware_update.c`): the recovery
    /// flag partition is the expected one, other firmware images are moved aside, and the flag
    /// reads back. Every failure stops the install before anything reboots.
    async fn prepare_recovery(&self, selected: &str) -> Result<()> {
        let (adb, serial) = (self.adb, self.serial);
        verify_card(adb, serial).await?;
        let mtd = adb.shell(serial, "cat /proc/mtd").await?;
        if !recovery_partition_ok(&mtd) {
            return Err(Error::Rejected(
                "the player's recovery flag partition is not the expected one, so no recovery was started"
                    .into(),
            ));
        }
        self.park_other_images(selected).await?;
        // A flag that is already set proves nothing about this run: it has to change.
        let before = adb.shell(serial, "head -c 11 /dev/mtd5").await?;
        if before == RECOVERY_FLAG {
            return Err(Error::Rejected(
                "the recovery flag was already set before this install, so it cannot be told whether this install set it. Restart the player normally and try again"
                    .into(),
            ));
        }
        // Not confirmed on hardware yet; if it fails the user falls back to the key combination.
        adb.shell(serial, "/usr/bin/bootmode.sh Recovery").await?;
        let flag = adb.shell(serial, "head -c 11 /dev/mtd5").await?;
        if flag != RECOVERY_FLAG {
            return Err(Error::Rejected(
                "the recovery flag could not be read back, so the player was not restarted. Do not restart it by hand until it is checked"
                    .into(),
            ));
        }
        Ok(())
    }

    /// Moves every other `.upt` at the top of the card to `<name>.parked` (or `.parked.1` ...),
    /// as the player does, so recovery cannot pick the wrong image.
    async fn park_other_images(&self, selected: &str) -> Result<()> {
        let (adb, serial) = (self.adb, self.serial);
        let listing = adb.shell(serial, &format!("ls -a {SD_ROOT}")).await?;
        let keep = selected.rsplit('/').next().unwrap_or(selected);
        for name in listing.lines().map(str::trim) {
            let lower = name.to_ascii_lowercase();
            if !lower.ends_with(".upt") || lower == keep.to_ascii_lowercase() {
                continue;
            }
            let from = format!("{SD_ROOT}/{name}");
            let mut parked = false;
            for n in 0..10 {
                let to = if n == 0 {
                    format!("{from}.parked")
                } else {
                    format!("{from}.parked.{n}")
                };
                let (q_from, q_to) = (shell_quote(&from), shell_quote(&to));
                let moved = adb
                    .shell(
                        serial,
                        &format!(
                            "if [ -e {q_to} ]; then echo taken; else mv {q_from} {q_to} && echo moved; fi"
                        ),
                    )
                    .await?;
                if moved.trim() == "moved" {
                    parked = true;
                    break;
                }
            }
            if !parked {
                return Err(Error::Rejected(format!(
                    "could not move {name} aside on the SD card"
                )));
            }
        }
        Ok(())
    }

    async fn boot_id(&self, serial: &str) -> Option<String> {
        let id = self
            .adb
            .shell(serial, "cat /proc/sys/kernel/random/boot_id")
            .await
            .ok()?;
        let id = id.trim();
        (!id.is_empty()).then(|| id.to_string())
    }

    /// Reboots, waits for the device and decides whether the install can be called done: the
    /// device has to be seen restarting (its boot id changes), be the same board, and report
    /// the release's version. Anything less is "not confirmed".
    async fn reboot_and_confirm(&self, remote: String) -> Result<InstallReport> {
        self.report(Phase::Rebooting, 0, 1);
        let before = self.boot_id(self.serial).await;
        // Players already connected are not the one coming back under a new serial.
        let others: Vec<String> = self
            .adb
            .devices()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.serial)
            .filter(|s| s != self.serial)
            .collect();
        self.adb.shell(self.serial, "sync").await?;
        // The connection drops as the device goes down, so an error here is expected.
        let _ = self.adb.shell(self.serial, "reboot").await;
        self.report(Phase::Waiting, 0, 1);
        let returned = wait_for_return(self.adb, self.serial, self.model, self.wait, &others).await;
        let (device, candidates) = match returned {
            Returned::Back(device) => (Some(device), Vec::new()),
            Returned::Candidates(found) => (None, found),
            Returned::TimedOut => (None, Vec::new()),
        };
        let freshness = self
            .release
            .freshness(device.as_ref().and_then(|d| d.player_version.as_deref()));
        let rebooted = match (&device, &before) {
            (Some(device), Some(before)) => self
                .boot_id(&device.serial)
                .await
                .is_some_and(|after| &after != before),
            _ => false,
        };
        let confirmed = rebooted
            && freshness == Freshness::UpToDate
            && device.as_ref().and_then(|d| d.board) == Some(self.model);
        Ok(InstallReport {
            remote_path: remote,
            device,
            candidates,
            freshness,
            rebooted,
            confirmed,
        })
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

/// What the install functions take besides the device. There is deliberately no model: the
/// board is whatever the connected device reports about itself.
pub struct InstallRequest<'a> {
    pub release: &'a Release,
    /// Where downloaded release files are kept.
    pub cache_dir: &'a Path,
    pub wait: WaitOptions,
}

impl<'a> InstallRequest<'a> {
    /// Identifies the device and refuses an unknown board or a missing card, so nothing is
    /// downloaded, staged or flashed for a device that cannot be recognised.
    async fn job(
        &'a self,
        adb: &'a Adb,
        serial: &'a str,
        on_progress: OnProgress<'a>,
    ) -> Result<Job<'a>> {
        let info = identify(adb, serial).await?;
        let model = info.board.ok_or_else(|| {
            Error::Rejected("this player model is not recognised, so nothing was installed".into())
        })?;
        if model == Model::TempotecV1 && info.firmware != Firmware::Compas {
            // The way onto a stock V1 is not confirmed, so nothing is staged for it.
            return Err(Error::Rejected(
                "installing Compás on a stock TempoTec Variations V1 is not supported yet, so nothing was installed"
                    .into(),
            ));
        }
        if !info.sd_mounted {
            return Err(Error::Rejected(
                "the player's SD card is not mounted, so nothing was installed".into(),
            ));
        }
        Ok(Job {
            adb,
            serial,
            model,
            release: self.release,
            cache_dir: self.cache_dir,
            wait: &self.wait,
            on_progress,
        })
    }
}

/// Full firmware image: staged at the top of the player's storage, installed from recovery.
pub async fn install_full_image(
    adb: &Adb,
    serial: &str,
    request: &InstallRequest<'_>,
    on_progress: OnProgress<'_>,
) -> Result<InstallReport> {
    let job = request.job(adb, serial, on_progress).await?;
    let remote = format!("{SD_ROOT}/{}.upt", job.model.update_stem());
    job.stage(Kind::FullImage, &remote).await?;
    job.prepare_recovery(&remote).await?;
    job.reboot_and_confirm(remote).await
}

/// Player only: replaces `.compas/compas_player` on the card and restarts.
pub async fn install_player_only(
    adb: &Adb,
    serial: &str,
    request: &InstallRequest<'_>,
    on_progress: OnProgress<'_>,
) -> Result<InstallReport> {
    let job = request.job(adb, serial, on_progress).await?;
    let remote = format!("{SD_ROOT}/.compas/compas_player");
    job.stage(Kind::PlayerOnly, &remote).await?;
    job.reboot_and_confirm(remote).await
}

#[cfg(all(test, unix))]
mod tests;
