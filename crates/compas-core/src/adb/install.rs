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
        Err(Error::Rejected(format!("{remote} is not on the SD card")))
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
                "could not read the free space on the player's card, so nothing was copied (df said: {})",
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
            "{remote} is not an update image at the card's root"
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

/// Full firmware image: staged at the card's root, installed from recovery.
pub async fn install_full_image(
    adb: &Adb,
    serial: &str,
    request: &InstallRequest<'_>,
    on_progress: OnProgress<'_>,
) -> Result<InstallReport> {
    let job = request.job(adb, serial, on_progress);
    let remote = format!("{SD_ROOT}/{}.upt", request.model.board());
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
mod tests {
    use super::super::fake::{FakeAdb, PS_COMPAS, PS_STOCK};
    use super::*;
    use crate::releases::test_fixtures::release;

    const QUICK: WaitOptions = WaitOptions {
        poll: Duration::from_millis(5),
        going_down: Duration::ZERO,
        coming_back: Duration::from_millis(300),
    };

    fn request<'a>(release: &'a Release, cache: &'a Path, model: Model) -> InstallRequest<'a> {
        InstallRequest {
            model,
            release,
            cache_dir: cache,
            wait: QUICK,
        }
    }

    fn pushes(fake: &FakeAdb) -> Vec<String> {
        fake.calls()
            .into_iter()
            .filter(|c| c.contains(" push "))
            .collect()
    }

    #[test]
    fn only_the_sd_card_is_acceptable_storage() {
        assert!(ensure_on_card("/data/mnt/sd_0/r1.upt").is_ok());
        for bad in [
            "/tmp/r1.upt",
            "/dev/shm/r1.upt",
            "/run/r1.upt",
            "/data/mnt/sd_01/x",
            "/usr/data/r1.upt",
        ] {
            assert!(ensure_on_card(bad).is_err(), "{bad}");
        }
        assert_eq!(parse_free_kb("Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/mmcblk0p1 7000000 100 6999900 1% /data/mnt/sd_0"), Some(6999900));
        assert_eq!(parse_free_kb("garbage"), None);
        // BusyBox puts a long device name on its own line.
        let wrapped = "Filesystem           1K-blocks      Used Available Use% Mounted on\n/dev/block/mmcblk0p1\n                       7000000       100   6999900   1% /data/mnt/sd_0\n";
        assert_eq!(parse_free_kb(wrapped), Some(6999900));
    }

    #[tokio::test]
    async fn a_full_image_goes_to_the_card_root_then_recovery_and_back() {
        let (_server, release) = release("v1.2.0").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "ingenic_1", "480,800", PS_STOCK);
        fake.write_remote(
            "/data/mnt/sd_0/.compas/device.json",
            r#"{"schema":1,"model":"r1","player_version":"1.2.0","plugin_api":17}"#,
        );

        let seen = std::sync::Mutex::new(Vec::new());
        let report = install_full_image(
            &fake.adb(),
            "ingenic_1",
            &request(&release, cache.path(), Model::R1),
            &|p| seen.lock().unwrap().push(p.phase),
        )
        .await
        .unwrap();

        assert_eq!(report.remote_path, "/data/mnt/sd_0/r1.upt");
        assert_eq!(
            std::fs::read(fake.remote("/data/mnt/sd_0/r1.upt")).unwrap(),
            b"image r1"
        );
        let pushes = pushes(&fake);
        assert_eq!(pushes.len(), 1);
        assert!(pushes[0].ends_with("/data/mnt/sd_0/r1.upt"), "{pushes:?}");
        let on_device = fake.calls().into_iter().filter(|c| c.contains(" shell "));
        assert!(
            on_device.into_iter().all(|c| !c.contains("/tmp")),
            "never RAM-backed storage"
        );

        let calls = fake.calls().join("\n");
        let at = |needle: &str| {
            calls
                .find(needle)
                .unwrap_or_else(|| panic!("{needle} missing in\n{calls}"))
        };
        assert!(at("push") < at("bootmode.sh Recovery"));
        assert!(at("bootmode.sh Recovery") < at("shell sync"));
        assert!(at("shell sync") < at("shell reboot"));

        assert_eq!(report.freshness, Freshness::UpToDate);
        assert_eq!(report.device.unwrap().board, Some(Model::R1));
        let phases = seen.into_inner().unwrap();
        for phase in [
            Phase::Downloading,
            Phase::Pushing,
            Phase::Verifying,
            Phase::Rebooting,
            Phase::Waiting,
        ] {
            assert!(phases.contains(&phase), "{phase:?}");
        }
    }

    #[tokio::test]
    async fn a_corrupt_push_is_removed_and_nothing_reboots() {
        let (_server, release) = release("v1.2.0").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_STOCK);
        fake.set("corrupt_push", "");

        let error = install_full_image(
            &fake.adb(),
            "s1",
            &request(&release, cache.path(), Model::R1),
            &|_| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(error, Error::Verification(_)), "{error}");
        assert!(!fake.remote("/data/mnt/sd_0/r1.upt").exists());
        let calls = fake.calls().join("\n");
        assert!(
            !calls.contains("bootmode") && !calls.contains("reboot"),
            "{calls}"
        );
    }

    #[tokio::test]
    async fn when_recovery_mode_cannot_be_entered_nothing_reboots() {
        let (_server, release) = release("v1.2.0").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "320,480", PS_STOCK);
        fake.set("bootmode_fails", "");

        let error = install_full_image(
            &fake.adb(),
            "s1",
            &request(&release, cache.path(), Model::R3II2025),
            &|_| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(error, Error::Adb(_)));
        assert!(
            fake.remote("/data/mnt/sd_0/r3ii_2025.upt").exists(),
            "the image stays for the key combination"
        );
        assert!(!fake.calls().join("\n").contains("reboot"));
    }

    #[tokio::test]
    async fn a_player_only_update_replaces_the_player_and_restarts() {
        let (_server, release) = release("v1.2.0").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,720", PS_COMPAS);
        fake.write_remote(
            "/data/mnt/sd_0/.compas/device.json",
            r#"{"schema":1,"model":"r3proii","player_version":"1.1.0","plugin_api":16}"#,
        );

        let report = install_player_only(
            &fake.adb(),
            "s1",
            &request(&release, cache.path(), Model::R3ProII),
            &|_| {},
        )
        .await
        .unwrap();
        assert_eq!(report.remote_path, "/data/mnt/sd_0/.compas/compas_player");
        assert_eq!(
            std::fs::read(fake.remote("/data/mnt/sd_0/.compas/compas_player")).unwrap(),
            b"player r3proii"
        );
        let calls = fake.calls().join("\n");
        assert!(
            !calls.contains("bootmode"),
            "no recovery for a player-only update"
        );
        assert!(calls.contains("shell reboot"));
        assert_eq!(
            report.freshness,
            Freshness::UpdateAvailable,
            "device.json still says 1.1.0"
        );
    }

    #[tokio::test]
    async fn a_device_that_does_not_come_back_is_reported_not_hung() {
        let (_server, release) = release("v1.2.0").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_STOCK);
        fake.set("devices.txt", "List of devices attached\n\n");

        let report = install_full_image(
            &fake.adb(),
            "s1",
            &request(&release, cache.path(), Model::R1),
            &|_| {},
        )
        .await
        .unwrap();
        assert!(report.device.is_none());
        assert_eq!(report.freshness, Freshness::Unknown);
    }

    fn devices_file(lines: &[&str]) -> String {
        let mut text = String::from("List of devices attached\n");
        for line in lines {
            text.push_str(line);
            text.push('\n');
        }
        text
    }

    const MAKO: &str = "product:mako model:Nexus_4 device:mako";

    #[tokio::test]
    async fn other_connected_devices_are_never_touched_or_mistaken_for_the_player() {
        let (_server, release) = release("v1.2.0").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "ingenic_1", "480,800", PS_STOCK);
        fake.set(
            "devices.txt",
            &devices_file(&[
                &format!("ingenic_1 device usb:1-1 {MAKO}"),
                "R58M12345AB device usb:1-2 product:beyond1lte model:SM_G973F device:beyond1lte",
            ]),
        );
        let report = install_full_image(
            &fake.adb(),
            "ingenic_1",
            &request(&release, cache.path(), Model::R1),
            &|_| {},
        )
        .await
        .unwrap();
        assert_eq!(report.device.unwrap().serial, "ingenic_1");
        assert!(
            fake.calls()
                .iter()
                .all(|c| !c.contains("R58M12345AB") || c.contains("devices")),
            "the phone was probed"
        );
    }

    #[tokio::test]
    async fn a_changed_serial_is_accepted_only_for_the_same_board() {
        let (_server, release) = release("v1.2.0").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "ingenic_1", "480,800", PS_STOCK);
        fake.set(
            "devices.txt",
            &devices_file(&[&format!("ingenic_after_recovery device usb:1-1 {MAKO}")]),
        );

        let report = install_full_image(
            &fake.adb(),
            "ingenic_1",
            &request(&release, cache.path(), Model::R1),
            &|_| {},
        )
        .await
        .unwrap();
        assert_eq!(report.device.unwrap().serial, "ingenic_after_recovery");

        // The same cable showing a different board is not the player we installed on.
        let other = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(other.path(), "ingenic_1", "480,720", PS_STOCK);
        fake.set(
            "devices.txt",
            &devices_file(&[&format!("ingenic_after_recovery device usb:1-1 {MAKO}")]),
        );
        let report = install_full_image(
            &fake.adb(),
            "ingenic_1",
            &request(&release, cache.path(), Model::R1),
            &|_| {},
        )
        .await
        .unwrap();
        assert!(report.device.is_none() && report.candidates.is_empty());
    }

    #[tokio::test]
    async fn several_matching_players_are_left_for_the_user_to_choose() {
        let (_server, release) = release("v1.2.0").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "ingenic_1", "480,800", PS_STOCK);
        fake.set(
            "devices.txt",
            &devices_file(&[
                &format!("ingenic_a device {MAKO}"),
                &format!("ingenic_b device {MAKO}"),
            ]),
        );
        let report = install_full_image(
            &fake.adb(),
            "ingenic_1",
            &request(&release, cache.path(), Model::R1),
            &|_| {},
        )
        .await
        .unwrap();
        assert!(report.device.is_none());
        let serials: Vec<_> = report
            .candidates
            .iter()
            .map(|c| c.serial.as_str())
            .collect();
        assert_eq!(serials, ["ingenic_a", "ingenic_b"]);
    }

    #[tokio::test]
    async fn an_unreadable_free_space_stops_before_anything_is_copied() {
        let (_server, release) = release("v1.2.0").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_STOCK);
        fake.set("df_output", "df: can't stat\n");
        let error = install_full_image(
            &fake.adb(),
            "s1",
            &request(&release, cache.path(), Model::R1),
            &|_| {},
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("free space"), "{error}");
        assert!(pushes(&fake).is_empty());
    }

    #[tokio::test]
    async fn the_update_file_can_be_removed_from_the_card_root_only() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_STOCK);
        fake.write_remote("/data/mnt/sd_0/r1.upt", "image");
        fake.write_remote("/data/mnt/sd_0/Music/a.upt", "not at the root");
        fake.write_remote("/data/mnt/sd_0/.compas/compas_player", "player");

        remove_staged_image(&fake.adb(), "s1", "/data/mnt/sd_0/r1.upt")
            .await
            .unwrap();
        assert!(!fake.remote("/data/mnt/sd_0/r1.upt").exists());
        for bad in [
            "/data/mnt/sd_0/Music/a.upt",
            "/data/mnt/sd_0/.compas/compas_player",
            "/tmp/r1.upt",
            "/data/mnt/sd_0/r1.upt; reboot",
        ] {
            assert!(
                remove_staged_image(&fake.adb(), "s1", bad).await.is_err(),
                "{bad}"
            );
        }
        assert!(fake.remote("/data/mnt/sd_0/Music/a.upt").exists());
    }
}
