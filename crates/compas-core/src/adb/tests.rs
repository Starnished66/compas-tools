use super::fake::{FakeAdb, PS_COMPAS, PS_STOCK};
use super::*;

#[tokio::test]
async fn lists_devices_and_runs_shell_commands() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "ingenic_a1b2", "480,800\n", PS_STOCK);
    fake.write_remote("/data/mnt/sd_0/hello.txt", "hi there");
    let adb = fake.adb();

    let devices = adb.devices().await.unwrap();
    assert_eq!(devices[0].serial, "ingenic_a1b2");
    assert_eq!(devices[0].state, "device");
    assert!(adb
        .version()
        .await
        .unwrap()
        .contains("Android Debug Bridge"));
    assert_eq!(
        adb.shell("ingenic_a1b2", "cat /data/mnt/sd_0/hello.txt")
            .await
            .unwrap(),
        "hi there"
    );
}

#[tokio::test]
async fn a_failing_command_is_an_error_with_its_message() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_STOCK);
    fake.set("bootmode_fails", "");
    let error = fake
        .adb()
        .shell("s1", "/usr/bin/bootmode.sh Recovery")
        .await
        .unwrap_err();
    assert!(
        matches!(&error, Error::Adb(m) if m.contains("not found")),
        "{error}"
    );
    assert_eq!(error.code(), "adb");

    let missing = Adb::new("/nonexistent/adb").devices().await.unwrap_err();
    assert!(matches!(missing, Error::Adb(_)));
}

#[tokio::test]
async fn identifies_each_board_and_firmware_from_recorded_outputs() {
    let cases = [
        ("480,800", PS_STOCK, Model::R1, Firmware::Stock),
        ("480,720", PS_COMPAS, Model::R3ProII, Firmware::Compas),
        ("320,480", PS_STOCK, Model::R3II2025, Firmware::Stock),
    ];
    for (size, ps, board, firmware) in cases {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "Nexus_4", size, ps);
        let info = identify(&fake.adb(), "Nexus_4").await.unwrap();
        assert_eq!(
            (info.board, info.firmware),
            (Some(board), firmware),
            "{size}"
        );
        assert!(info.player_version.is_none(), "no device.json yet");
    }
}

#[tokio::test]
async fn the_version_comes_from_device_json_on_the_card() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
    fake.write_remote(
        "/data/mnt/sd_0/.compas/device.json",
        r#"{"schema":1,"model":"r1","player_version":"1.2.0","plugin_api":17}"#,
    );
    let info = identify(&fake.adb(), "s1").await.unwrap();
    assert_eq!(info.player_version.as_deref(), Some("1.2.0"));
    assert_eq!(info.plugin_api, Some(17));
    assert!(info.older_than.is_none());
}

#[tokio::test]
async fn a_v_prefixed_version_is_kept_as_the_player_wrote_it() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
    fake.write_remote(
        "/data/mnt/sd_0/.compas/device.json",
        r#"{"schema":1,"model":"r1","player_version":"v1.3.0","plugin_api":17}"#,
    );
    let info = identify(&fake.adb(), "s1").await.unwrap();
    assert_eq!(info.player_version.as_deref(), Some("v1.3.0"));
    assert!(info.older_than.is_none());
}

/// v1.2.0 runs `compas_player` but writes no `device.json`: the version cannot be read, yet
/// the player is known to be older than the first release that writes the file.
#[tokio::test]
async fn a_v1_2_0_player_is_recognised_by_running_compas_without_device_json() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
    fake.write_remote("/usr/data/.compas/settings.txt", "volume=12\n");
    let info = identify(&fake.adb(), "s1").await.unwrap();
    assert_eq!(info.firmware, Firmware::Compas);
    assert_eq!(info.board, Some(Model::R1));
    assert!(info.player_version.is_none());
    assert_eq!(info.older_than.as_deref(), Some("v1.3.0"));

    // The vendor's firmware has no device.json either, and is not "older Compás".
    let stock = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(stock.path(), "s2", "480,800", PS_STOCK);
    assert!(identify(&fake.adb(), "s2")
        .await
        .unwrap()
        .older_than
        .is_none());
}

use crate::card::Model;
