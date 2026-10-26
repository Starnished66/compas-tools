use super::*;

#[test]
fn credentials_the_sd_card_and_plugins_are_always_left_out() {
    for path in [
        ".compas/plugins/lastfm.json",
        ".compas/plugins",
        ".compas/subsonic_servers.tsv",
        "mnt/sd_0/Music/a.flac",
        "mnt",
        "pin.conf",
        "wpa_supplicant.conf",
        "a/b/remote_pin",
        "wifi/known.cfg",
    ] {
        assert!(is_left_out(path), "{path}");
    }
    for path in [
        ".compas/settings.txt",
        ".compas/pluginsx/a",
        "player_crash.log",
        "theme_overrides/a.png",
    ] {
        assert!(!is_left_out(path), "{path}");
    }
}

#[test]
fn device_supplied_paths_cannot_escape_or_break_quoting() {
    for bad in ["../x", "a/../b", "/abs", "a//b", "it's", "a\\b", "a\nb", ""] {
        assert!(!is_safe_relative(bad), "{bad:?}");
    }
    assert!(is_safe_relative("theme_overrides/icons/play.png"));
}

#[test]
fn the_allowlist_never_names_anything_that_is_always_excluded() {
    for entry in ALLOWED {
        assert!(!is_left_out(entry), "{entry}");
    }
}

#[cfg(unix)]
mod with_fake_adb {
    use super::super::super::fake::{FakeAdb, PS_COMPAS, PS_STOCK};
    use super::super::*;

    fn fixture(dir: &Path) -> FakeAdb {
        let fake = FakeAdb::new(dir, "ingenic_1", "480,800", PS_STOCK);
        fake.write_remote(
            "/usr/data/player_crash.log",
            "crash\nremote_control_pin=424242\n",
        );
        fake.write_remote(
            "/usr/data/.compas/settings.txt",
            "volume=12\nremote_control_pin=424242\nsubsonic_password=hunter2\nsubsonic_saved_0_password=hunter3\nwifi_last=Casa Garita 5G\nname=My Player\n",
        );
        fake.write_remote(
            "/usr/data/.compas/subsonic_servers.tsv",
            "https://x\tana\tPASSWORD-IN-TSV\t1\n",
        );
        fake.write_remote(
            "/usr/data/.compas/plugins/lastfm.json",
            "{\"session_key\": \"ACCOUNT-TOKEN\"}",
        );
        fake.write_remote("/usr/data/.compas/queue.bin", "text");
        fake.write_remote("/usr/data/mnt/sd_0/Music/song.flac", "SD-CARD-AUDIO");
        fake.write_remote("/usr/data/mnt/sd_0/.compas/device.json", "{}");
        fake.write_remote("/usr/data/pin.conf", "* 0000");
        fake.write_remote("/usr/data/wpa_supplicant.conf", "psk=\"WIFI-SECRET\"");
        fake.write_remote("/usr/data/theme_overrides/colors.txt", "accent=blue\n");
        fake.write_remote("/usr/data/theme_overrides/icon.bin", "x");
        std::fs::write(
            fake.remote("/usr/data/theme_overrides/icon.bin"),
            [0u8, 159, 146, 150],
        )
        .unwrap();
        std::fs::write(fake.remote("/usr/data/big.log"), vec![b'a'; 3 << 20]).unwrap();
        std::fs::write(
            fake.remote("/usr/data/player_exit.log"),
            vec![b'a'; 2 << 20],
        )
        .unwrap();
        fake
    }

    #[tokio::test]
    async fn only_the_allowlist_is_read_and_everything_is_masked_before_it_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let fake = fixture(dir.path());
        let out = tempfile::tempdir().unwrap();
        let report = backup_user_data(&fake.adb(), "ingenic_1", out.path(), &|_| {})
            .await
            .unwrap();
        let root = &report.path;

        // What is in the backup, and nothing else.
        let settings = std::fs::read_to_string(root.join(".compas/settings.txt")).unwrap();
        assert!(settings.contains("volume=12") && settings.contains("name=My Player"));
        for secret in ["424242", "hunter2", "hunter3", "Casa Garita"] {
            assert!(!settings.contains(secret), "{secret} reached the disk");
        }
        assert!(std::fs::read_to_string(root.join("player_crash.log"))
            .unwrap()
            .contains("<redacted>"));
        assert!(root.join("theme_overrides/colors.txt").exists());

        // What is not: the SD card, credentials, plugins, binaries and big files.
        for path in [
            ".compas/subsonic_servers.tsv",
            ".compas/plugins",
            "mnt",
            "pin.conf",
            "wpa_supplicant.conf",
            "big.log",
            "theme_overrides/icon.bin",
            "player_exit.log",
        ] {
            assert!(!root.join(path).exists(), "{path} was copied");
        }
        let reason = |path: &str| {
            report
                .skipped
                .iter()
                .find(|s| s.path == path)
                .map(|s| s.reason)
        };
        assert_eq!(reason("theme_overrides/icon.bin"), Some(SkipReason::Binary));
        assert_eq!(reason("player_exit.log"), Some(SkipReason::TooLarge));
        assert_eq!(report.files, 3);
        assert_eq!(report.redacted, 5);

        // The device was never asked for anything outside the allowlist.
        let calls = fake.calls().join("\n");
        for forbidden in [
            "mnt/sd_0",
            "subsonic_servers",
            "plugins",
            "pin.conf",
            "wpa_supplicant",
            " pull ",
        ] {
            assert!(!calls.contains(forbidden), "{forbidden} in\n{calls}");
        }
    }

    /// v1.2.0 keeps only some of the allowlisted files in `/usr/data` and has no device.json.
    #[tokio::test]
    async fn a_v1_2_0_layout_with_few_files_backs_up() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "ingenic_1", "480,800", PS_COMPAS);
        fake.write_remote("/usr/data/.compas/settings.txt", "volume=7\nname=Old\n");
        fake.write_remote("/usr/data/player_exit.log", "exit 0\n");
        fake.write_remote("/usr/data/.compas/plugins/x.json", "{\"token\":\"T\"}");
        let out = tempfile::tempdir().unwrap();
        let report = backup_user_data(&fake.adb(), "ingenic_1", out.path(), &|_| {})
            .await
            .unwrap();
        assert_eq!(report.files, 2);
        assert_eq!(report.quality, BackupQuality::Complete);
        assert!(report.path.join(".compas/settings.txt").exists());
        assert!(!report.path.join(".compas/plugins").exists());
    }

    #[tokio::test]
    async fn a_copy_without_settings_is_unusable_and_one_with_skips_is_partial() {
        // Only a binary theme file: nothing is copied, yet the listing was not empty.
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
        fake.write_remote("/usr/data/theme_overrides/icon.bin", "x");
        std::fs::write(
            fake.remote("/usr/data/theme_overrides/icon.bin"),
            [0u8, 159, 146, 150],
        )
        .unwrap();
        let out = tempfile::tempdir().unwrap();
        let report = backup_user_data(&fake.adb(), "s1", out.path(), &|_| {})
            .await
            .unwrap();
        assert_eq!(report.files, 0);
        assert_eq!(report.quality, BackupQuality::Unusable);

        // Logs alone are not the settings either.
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s2", "480,800", PS_COMPAS);
        fake.write_remote("/usr/data/player_exit.log", "exit 0\n");
        let report = backup_user_data(&fake.adb(), "s2", out.path(), &|_| {})
            .await
            .unwrap();
        assert_eq!(report.quality, BackupQuality::Unusable);

        // Settings plus a skipped binary: usable, but partial.
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s3", "480,800", PS_COMPAS);
        fake.write_remote("/usr/data/.compas/settings.txt", "volume=7\n");
        fake.write_remote("/usr/data/theme_overrides/icon.bin", "x");
        std::fs::write(
            fake.remote("/usr/data/theme_overrides/icon.bin"),
            [0u8, 159, 146, 150],
        )
        .unwrap();
        let report = backup_user_data(&fake.adb(), "s3", out.path(), &|_| {})
            .await
            .unwrap();
        assert_eq!(report.quality, BackupQuality::Partial);
    }

    #[tokio::test]
    async fn a_linked_entry_is_not_followed_off_the_allowlist() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_STOCK);
        fake.write_remote("/usr/data/mnt/sd_0/secret.txt", "SD-ONLY");
        fake.write_remote("/usr/data/player_crash.log", "ok\n");
        std::fs::create_dir_all(fake.remote("/usr/data")).unwrap();
        std::os::unix::fs::symlink(
            fake.remote("/usr/data/mnt/sd_0"),
            fake.remote("/usr/data/theme_overrides"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            fake.remote("/usr/data/mnt/sd_0/secret.txt"),
            fake.remote("/usr/data/player_exit.log"),
        )
        .unwrap();

        let out = tempfile::tempdir().unwrap();
        let report = backup_user_data(&fake.adb(), "s1", out.path(), &|_| {})
            .await
            .unwrap();
        assert_eq!(report.files, 1);
        assert!(!report.path.join("theme_overrides").exists());
        assert!(!report.path.join("player_exit.log").exists());
    }

    #[tokio::test]
    async fn unreadable_files_empty_files_and_odd_names_get_their_own_reasons() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_STOCK);
        fake.write_remote("/usr/data/player_crash.log", "locked\n");
        fake.write_remote("/usr/data/player_exit.log", "");
        fake.write_remote("/usr/data/theme_overrides/it's.txt", "quote in the name\n");
        fake.write_remote("/usr/data/boot_debug.log", "fine\n");
        let locked = fake.remote("/usr/data/player_crash.log");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let out = tempfile::tempdir().unwrap();
        let report = backup_user_data(&fake.adb(), "s1", out.path(), &|_| {})
            .await
            .unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();

        let reason = |path: &str| {
            report
                .skipped
                .iter()
                .find(|s| s.path == path)
                .map(|s| s.reason)
        };
        if std::fs::read(&locked).is_ok() && reason("player_crash.log").is_none() {
            return; // running as root: permissions do not apply
        }
        assert_eq!(reason("player_crash.log"), Some(SkipReason::Unreadable));
        assert_eq!(
            reason("theme_overrides/it's.txt"),
            Some(SkipReason::UnsupportedName)
        );
        assert!(report.path.join("boot_debug.log").exists());
        assert_eq!(
            std::fs::read(report.path.join("player_exit.log")).unwrap(),
            b"",
            "a real empty file is copied"
        );
        assert!(reason("player_exit.log").is_none());
    }

    /// The vendor's firmware has none of the Compás files; that is an unusable copy the user can
    /// choose to go without, not an error that locks the first install out.
    #[tokio::test]
    async fn a_stock_player_with_nothing_to_copy_gives_an_unusable_copy_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "stock_1", "480,800", PS_STOCK);
        let out = tempfile::tempdir().unwrap();
        let report = backup_user_data(&fake.adb(), "stock_1", out.path(), &|_| {})
            .await
            .unwrap();
        assert_eq!(report.files, 0);
        assert_eq!(report.quality, BackupQuality::Unusable);
    }
}
