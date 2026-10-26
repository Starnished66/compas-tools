//! The checks that run before a device is flashed and after it restarts.

use super::*;

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
    for (what, file, content) in [(
        "partition",
        "/proc/mtd",
        "mtd5: 00100000 00020000 \"other\"\n",
    )] {
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
async fn the_recovery_flag_has_to_change_because_of_this_install() {
    let (_server, release) = release("v1.2.0").await;
    let cache = tempfile::tempdir().unwrap();
    // The helper does nothing: whatever the flag block held is not proof of anything.
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
    fake.set("bootmode_noop", "");
    let error = install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert!(error.to_string().contains("read back"), "{error}");
    assert!(!fake.calls().join("\n").contains("shell reboot"));

    // A flag that was already set before this run cannot be credited to it.
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s2", "480,800", PS_STOCK);
    fake.write_remote("/dev/mtd5", "ota:kernel2");
    let error = install_full_image(&fake.adb(), "s2", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert!(error.to_string().contains("already set"), "{error}");
    assert!(!fake.calls().join("\n").contains("bootmode"));
}

#[tokio::test]
async fn the_card_s_device_json_never_names_the_board() {
    let (_server, release) = release("v1.2.0").await;
    let cache = tempfile::tempdir().unwrap();
    // A stock player with an unreadable screen size and a card from an R1.
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "?", PS_STOCK);
    fake.write_remote(
        "/data/mnt/sd_0/.compas/device.json",
        r#"{"schema":1,"model":"r1","player_version":"v1.2.0","plugin_api":17}"#,
    );
    let error = install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Rejected(_)), "{error}");
    let calls = fake.calls().join("\n");
    assert!(
        !calls.contains(" push ") && !calls.contains("bootmode"),
        "{calls}"
    );
}

#[tokio::test]
async fn a_stock_tempotec_v1_is_refused_by_the_backend() {
    let (_server, release) = release("v1.3.0").await;
    let cache = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "v1", "240,320", PS_STOCK);
    let error = install_full_image(&fake.adb(), "v1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap_err();
    assert!(error.to_string().contains("stock TempoTec"), "{error}");
    assert!(!fake.calls().join("\n").contains(" push "));
}

#[tokio::test]
async fn a_stranger_of_the_same_board_is_not_the_returning_player() {
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "ingenic_1", "480,800", PS_STOCK);
    // Our player is gone; a second R1 was connected all along.
    fake.set(
        "devices.txt",
        "List of devices attached\nother_r1 device usb:1-2 product:mako model:Nexus_4 device:mako\n",
    );
    let known = vec!["other_r1".to_string()];
    let returned = wait_for_return(&fake.adb(), "ingenic_1", Model::R1, &QUICK, &known).await;
    assert!(matches!(returned, Returned::TimedOut));
    // Without the knowledge that it was already there, it would have been taken for ours.
    let returned = wait_for_return(&fake.adb(), "ingenic_1", Model::R1, &QUICK, &[]).await;
    assert!(matches!(returned, Returned::Back(_)));
}

#[tokio::test]
async fn names_with_spaces_are_parked_too_and_the_rename_is_confirmed() {
    let (_server, release) = release("v1.2.0").await;
    let cache = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let fake = player(dir.path(), "s1", "480,800", PS_STOCK);
    fake.write_remote("/data/mnt/sd_0/r1 (1).upt", "older image");
    install_full_image(&fake.adb(), "s1", &request(&release, cache.path()), &|_| {})
        .await
        .unwrap();
    assert!(fake.remote("/data/mnt/sd_0/r1 (1).upt.parked").exists());
    assert!(fake.remote("/data/mnt/sd_0/r1.upt").exists());
}
