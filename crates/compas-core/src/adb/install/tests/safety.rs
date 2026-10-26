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
