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
