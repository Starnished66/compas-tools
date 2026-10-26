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

/// The version is the label baked into the running program, as recorded from real players.
const V1_2_0_PROGRAM: &str = "ELF\0player build v1.2.0 settings.txt\0";
const V1_3_0_PROGRAM: &str = "ELF\0player build v1.3.0 .compas/device.json\0";

async fn version_of(program: Option<&str>, ps: &str) -> Option<String> {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s1", "480,800", ps);
    if let Some(program) = program {
        fake.write_remote("/usr/bin/compas_player", program);
    }
    identify(&fake.adb(), "s1").await.unwrap().player_version
}

#[tokio::test]
async fn the_version_is_read_from_the_running_program_not_from_the_card() {
    assert_eq!(
        version_of(Some(V1_2_0_PROGRAM), PS_COMPAS).await.as_deref(),
        Some("v1.2.0")
    );
    assert_eq!(
        version_of(Some(V1_3_0_PROGRAM), PS_COMPAS).await.as_deref(),
        Some("v1.3.0")
    );

    // A device.json on the card is not consulted for the version, only for the plugin API.
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
    fake.write_remote(
        "/data/mnt/sd_0/.compas/device.json",
        r#"{"schema":1,"model":"r1","player_version":"v9.9.9","plugin_api":17}"#,
    );
    let info = identify(&fake.adb(), "s1").await.unwrap();
    assert_eq!(info.player_version, None);
    assert_eq!(info.plugin_api, Some(17));
}

#[tokio::test]
async fn an_unclear_label_or_a_failing_read_leaves_the_version_unknown() {
    // A development build carries no vX.Y.Z; several versions are ambiguous.
    for program in ["ELF Development Build", "ELF v1.2.0 and v1.3.0", ""] {
        assert_eq!(
            version_of(Some(program), PS_COMPAS).await,
            None,
            "{program}"
        );
    }
    // The program is not where ps says: grep cannot be asked.
    assert_eq!(version_of(None, PS_COMPAS).await, None);
    // The vendor's firmware has no Compás program to read.
    assert_eq!(version_of(Some(V1_3_0_PROGRAM), PS_STOCK).await, None);
}

#[tokio::test]
async fn a_player_only_install_is_found_through_ps() {
    let ps = PS_COMPAS.replace("/usr/bin/compas_player", "/usr/data/compas_player");
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s2", "480,800", &ps);
    fake.write_remote("/usr/data/compas_player", V1_2_0_PROGRAM);
    let info = identify(&fake.adb(), "s2").await.unwrap();
    assert_eq!(info.player_version.as_deref(), Some("v1.2.0"));
}

#[test]
fn the_player_program_is_taken_from_ps_with_a_safe_default() {
    assert_eq!(identify::player_binary(PS_COMPAS), "/usr/bin/compas_player");
    assert_eq!(
        identify::player_binary("1 root compas_player"),
        "/usr/bin/compas_player"
    );
    assert_eq!(
        identify::player_binary("1 root /tmp/a;b/compas_player"),
        "/usr/bin/compas_player"
    );
}

#[tokio::test]
async fn the_sd_card_must_show_in_proc_mounts() {
    let mounts = "/dev/root / ext4 rw 0 0\n/dev/mmcblk0p1 /usr/data/mnt/sd_0 vfat rw 0 0\n";
    let other = "/dev/root / ext4 rw 0 0\n/dev/mmcblk0p2 /usr/data/mnt/sd_01 vfat rw 0 0\n";
    for (content, expected) in [(Some(mounts), true), (Some(other), false), (None, false)] {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
        if let Some(content) = content {
            fake.write_remote("/proc/mounts", content);
        }
        let info = identify(&fake.adb(), "s1").await.unwrap();
        assert_eq!(info.sd_mounted, expected, "{content:?}");
    }
}

#[tokio::test]
async fn a_tempotec_v1_is_recognised_by_its_marker_or_its_screen() {
    let program = "ELF COMPAS_PLAYER_BOARD:tempotec_v1 build v1.3.0";
    // Compás: the marker in the running program names the board, whatever the screen says.
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
    fake.write_remote("/usr/bin/compas_player", program);
    let info = identify(&fake.adb(), "s1").await.unwrap();
    assert_eq!(info.board, Some(Model::TempotecV1));
    assert_eq!(info.player_version.as_deref(), Some("v1.3.0"));

    // The vendor's firmware has no marker: the 240x320 panel says it is a V1.
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s2", "240,320", PS_STOCK);
    let info = identify(&fake.adb(), "s2").await.unwrap();
    assert_eq!(
        (info.board, info.firmware),
        (Some(Model::TempotecV1), Firmware::Stock)
    );

    // Two markers, an unknown board or a missing program fall back to the screen.
    for program in [
        "COMPAS_PLAYER_BOARD:r1 COMPAS_PLAYER_BOARD:r3proii",
        "COMPAS_PLAYER_BOARD:future_board",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s3", "480,720", PS_COMPAS);
        fake.write_remote("/usr/bin/compas_player", program);
        assert_eq!(
            identify(&fake.adb(), "s3").await.unwrap().board,
            Some(Model::R3ProII)
        );
    }
}

#[tokio::test]
async fn the_marker_beats_a_misleading_screen_size_on_other_boards() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
    fake.write_remote("/usr/bin/compas_player", "COMPAS_PLAYER_BOARD:r3ii_2025");
    assert_eq!(
        identify(&fake.adb(), "s1").await.unwrap().board,
        Some(Model::R3II2025)
    );
}

use crate::card::Model;
