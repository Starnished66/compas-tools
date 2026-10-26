use super::super::fake::{FakeAdb, PS_COMPAS, PS_STOCK};
use super::*;
use crate::releases::test_fixtures::release;

const QUICK: WaitOptions = WaitOptions {
    poll: Duration::from_millis(5),
    going_down: Duration::ZERO,
    coming_back: Duration::from_millis(300),
};

fn request<'a>(release: &'a Release, cache: &'a Path) -> InstallRequest<'a> {
    InstallRequest {
        release,
        cache_dir: cache,
        wait: QUICK,
    }
}

/// A pretend player that is ready to be updated: its SD card is mounted, its recovery flag
/// partition is the expected one and it has a boot id (which a `reboot` changes).
fn player(dir: &Path, serial: &str, size: &str, ps: &str) -> FakeAdb {
    let fake = FakeAdb::new(dir, serial, size, ps);
    fake.write_remote(
        "/proc/mounts",
        "/dev/root / ext4 rw 0 0\n/dev/mmcblk0p1 /usr/data/mnt/sd_0 vfat rw 0 0\n",
    );
    fake.write_remote(
        "/proc/mtd",
        "dev: size erasesize name\nmtd5: 00100000 00020000 \"ota\"\n",
    );
    fake.write_remote("/dev/mtd5", "ota:kernel2");
    fake.write_remote("/proc/sys/kernel/random/boot_id", "boot-before\n");
    fake
}

fn sd_files(fake: &FakeAdb) -> Vec<String> {
    std::fs::read_dir(fake.remote("/data/mnt/sd_0"))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
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
    let fake = player(dir.path(), "ingenic_1", "480,800", PS_COMPAS);
    // The version is read from the running program, which stays the old one until it restarts.
    fake.write_remote("/usr/bin/compas_player", "ELF player v1.2.0");

    let seen = std::sync::Mutex::new(Vec::new());
    let report = install_full_image(
        &fake.adb(),
        "ingenic_1",
        &request(&release, cache.path()),
        &|p| seen.lock().unwrap().push(p.phase),
    )
    .await
    .unwrap();

    assert_eq!(report.remote_path, "/data/mnt/sd_0/r1.upt");
    assert_eq!(
        std::fs::read(fake.remote("/data/mnt/sd_0/r1.upt")).unwrap(),
        b"image r1"
    );
    // Pushed under a temporary name of its own, then renamed into place.
    let pushes = pushes(&fake);
    assert_eq!(pushes.len(), 1);
    assert!(
        pushes[0].contains("/data/mnt/sd_0/.r1.upt.") && pushes[0].ends_with(".part"),
        "{pushes:?}"
    );
    assert!(sd_files(&fake).iter().all(|f| !f.ends_with(".part")));
    assert!(report.rebooted && report.confirmed);
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
    assert!(at("mv -f") < at("bootmode.sh Recovery"));
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
async fn a_tempotec_v1_gets_v1_upt_not_its_board_id() {
    let (_server, release) = release("v1.3.0").await;
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "v1_serial", "240,320", PS_COMPAS);
    fake.write_remote(
        "/usr/bin/compas_player",
        "COMPAS_PLAYER_BOARD:tempotec_v1 v1.2.0",
    );
    let report = install_full_image(
        &fake.adb(),
        "v1_serial",
        &request(&release, cache.path()),
        &|_| {},
    )
    .await
    .unwrap();
    assert_eq!(report.remote_path, "/data/mnt/sd_0/v1.upt");
    assert_eq!(
        std::fs::read(fake.remote("/data/mnt/sd_0/v1.upt")).unwrap(),
        b"image tempotec_v1"
    );
    assert_eq!(report.device.unwrap().board, Some(Model::TempotecV1));
}

#[tokio::test]
async fn a_corrupt_push_is_removed_and_nothing_reboots() {
    let (_server, release) = release("v1.2.0").await;
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
    fake.set("corrupt_push", "");

    let error = install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Verification(_)), "{error}");
    assert!(!fake.remote("/data/mnt/sd_0/r1.upt").exists());
    assert!(
        sd_files(&fake).iter().all(|f| !f.ends_with(".part")),
        "{:?}",
        sd_files(&fake)
    );
    let calls = fake.calls().join("\n");
    assert!(
        !calls.contains("bootmode") && !calls.contains("reboot"),
        "{calls}"
    );
}

#[tokio::test]
async fn a_failed_upload_leaves_an_earlier_staged_update_alone() {
    let (_server, release) = release("v1.2.0").await;
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
    fake.write_remote("/data/mnt/sd_0/r1.upt", "an older staged image");
    fake.set("corrupt_push", "");
    install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert_eq!(
        std::fs::read(fake.remote("/data/mnt/sd_0/r1.upt")).unwrap(),
        b"an older staged image"
    );
}

#[tokio::test]
async fn without_sha256sum_the_file_is_pulled_back_and_hashed_here() {
    let (_server, release) = release("v1.2.0").await;
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
    fake.set("no_sha256sum", "");
    install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap();
    assert!(fake.calls().iter().any(|c| c.contains(" pull ")));

    // A same-size wrong file is caught too; a byte count never counts.
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s2", "480,800", PS_STOCK);
    fake.set("no_sha256sum", "");
    fake.set("corrupt_push", "");
    let error = install_full_image(&fake.adb(), "s2", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Verification(_)), "{error}");
    assert!(!fake.remote("/data/mnt/sd_0/r1.upt").exists());
}

#[tokio::test]
async fn an_unknown_board_or_a_mismatching_one_is_never_flashed() {
    let (_server, release) = release("v1.2.0").await;
    let cache = tempfile::tempdir().unwrap();
    // An odd screen and no marker: unknown, nothing is downloaded, pushed or flashed.
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "123,456", PS_STOCK);
    let error = install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Rejected(_)), "{error}");
    assert!(error.to_string().contains("not recognised"), "{error}");
    let calls = fake.calls().join("\n");
    assert!(
        !calls.contains(" push ") && !calls.contains("bootmode"),
        "{calls}"
    );

    // A Compás player says its own board, and that board gets its own image.
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s2", "480,800", PS_COMPAS);
    fake.write_remote(
        "/usr/bin/compas_player",
        "COMPAS_PLAYER_BOARD:r3proii v1.2.0",
    );
    install_full_image(&fake.adb(), "s2", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap();
    assert!(fake.remote("/data/mnt/sd_0/r3proii.upt").exists());
    assert!(!fake.remote("/data/mnt/sd_0/r1.upt").exists());
}

#[tokio::test]
async fn the_target_must_be_the_mounted_sd_card() {
    let (_server, release) = release("v1.2.0").await;
    let cache = tempfile::tempdir().unwrap();
    for (what, setup) in [
        (
            "no card in /proc/mounts",
            ("/proc/mounts", "/dev/root / ext4 rw 0 0\n"),
        ),
        (
            "a tmpfs",
            ("/proc/mounts", "tmpfs /usr/data/mnt/sd_0 tmpfs rw 0 0\n"),
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
        fake.write_remote(setup.0, setup.1);
        let error =
            install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
                .await
                .unwrap_err();
        assert!(matches!(error, Error::Rejected(_)), "{what}: {error}");
        assert!(pushes(&fake).is_empty(), "{what}");
    }
    // df showing internal storage under the card's path is refused as well.
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s2", "480,800", PS_STOCK);
    fake.set(
        "df_output",
        "Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/mtdblock9 70000 100 69900 1% /usr/data\n",
    );
    let error = install_full_image(&fake.adb(), "s2", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert!(error.to_string().contains("not on the SD card"), "{error}");
    assert!(pushes(&fake).is_empty());
}

#[tokio::test]
async fn other_firmware_images_are_parked_and_the_flag_is_checked_before_rebooting() {
    let (_server, release) = release("v1.2.0").await;
    let cache = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
    fake.write_remote("/data/mnt/sd_0/old.UPT", "older image");
    install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap();
    assert!(!fake.remote("/data/mnt/sd_0/old.UPT").exists());
    assert!(fake.remote("/data/mnt/sd_0/old.UPT.parked").exists());
    assert!(fake.remote("/data/mnt/sd_0/r1.upt").exists());

    // A partition that is not the expected one, or a flag that does not read back, stops it.
    for (what, file, content) in [
        (
            "partition",
            "/proc/mtd",
            "mtd5: 00100000 00020000 \"other\"\n",
        ),
        ("flag", "/dev/mtd5", "something else"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let fake = player(dir.path(), "s2", "480,800", PS_STOCK);
        fake.write_remote(file, content);
        let error =
            install_full_image(&fake.adb(), "s2", &request(&release, cache.path()), &|_| {})
                .await
                .unwrap_err();
        assert!(matches!(error, Error::Rejected(_)), "{what}: {error}");
        assert!(!fake.calls().join("\n").contains("shell reboot"), "{what}");
    }
}

#[tokio::test]
async fn an_install_without_an_observed_restart_is_not_confirmed() {
    let (_server, release) = release("v1.2.0").await;
    let cache = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    // The program already runs the release's version and the reboot does nothing.
    let fake = player(dir.path(), "s1", "480,800", PS_COMPAS);
    fake.write_remote("/usr/bin/compas_player", "COMPAS_PLAYER_BOARD:r1 v1.2.0");
    fake.set("no_reboot", "");
    let report = install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap();
    assert_eq!(report.freshness, Freshness::UpToDate);
    assert!(!report.rebooted && !report.confirmed);
}

#[tokio::test]
async fn when_recovery_mode_cannot_be_entered_nothing_reboots() {
    let (_server, release) = release("v1.2.0").await;
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "320,480", PS_STOCK);
    fake.set("bootmode_fails", "");

    let error = install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
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
    let fake = player(dir.path(), "s1", "480,720", PS_COMPAS);
    fake.write_remote("/usr/bin/compas_player", "ELF player v1.1.0");

    let report = install_player_only(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
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
        "the program still says v1.1.0"
    );
}

#[tokio::test]
async fn a_device_that_does_not_come_back_is_reported_not_hung() {
    let (_server, release) = release("v1.2.0").await;
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
    fake.set("devices.txt", "List of devices attached\n\n");

    let report = install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
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
    let fake = player(dir.path(), "ingenic_1", "480,800", PS_STOCK);
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
        &request(&release, cache.path()),
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
    let fake = player(dir.path(), "ingenic_1", "480,800", PS_STOCK);
    fake.set(
        "devices.txt",
        &devices_file(&[&format!("ingenic_after_recovery device usb:1-1 {MAKO}")]),
    );

    let report = install_full_image(
        &fake.adb(),
        "ingenic_1",
        &request(&release, cache.path()),
        &|_| {},
    )
    .await
    .unwrap();
    assert_eq!(report.device.unwrap().serial, "ingenic_after_recovery");

    // The same cable showing a different board is not the player we installed on.
    let other = tempfile::tempdir().unwrap();
    let fake = player(other.path(), "ingenic_1", "480,720", PS_STOCK);
    fake.set(
        "devices.txt",
        &devices_file(&[&format!("ingenic_after_recovery device usb:1-1 {MAKO}")]),
    );
    let returned = wait_for_return(&fake.adb(), "ingenic_1", Model::R1, &QUICK).await;
    assert!(matches!(returned, Returned::TimedOut));
}

#[tokio::test]
async fn several_matching_players_are_left_for_the_user_to_choose() {
    let (_server, release) = release("v1.2.0").await;
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "ingenic_1", "480,800", PS_STOCK);
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
        &request(&release, cache.path()),
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
    let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
    fake.set("df_output", "df: can't stat\n");
    let error = install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert!(error.to_string().contains("free space"), "{error}");
    assert!(pushes(&fake).is_empty());
}

#[tokio::test]
async fn the_update_file_can_be_removed_from_the_card_root_only() {
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
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
