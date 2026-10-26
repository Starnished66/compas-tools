//! Logs and diagnostics from a player, redacted, for a bug report.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde::Serialize;

use super::identify::DeviceInfo;
use super::redact::redact;
use super::{Adb, SD_ROOT, USER_DATA};
use crate::error::{Error, IoContext, Result};
use crate::progress::{OnProgress, Phase, Progress};

/// Longest tail kept of one log, so a runaway log cannot fill the report.
const TAIL_BYTES: u64 = 1 << 20;
const MISSING: &str = "__compas_tools_missing__";

/// Log files on the device: `(name in the report, path on the device)`.
fn log_paths() -> Vec<(String, String)> {
    let internal = [
        "player_crash.log",
        "player_crash.log.1",
        "bootloader_boot.log",
        "player_exit.log",
        "boot_debug.log",
        "sd_fsck.log",
        "selector-script.log",
    ]
    .map(|n| (format!("usr_data/{n}"), format!("{USER_DATA}/{n}")));
    let card = [
        ("sd/ota_update.log", ".compas/ota/update.log"),
        ("sd/reload_diag.log", ".logs/reload_diag.log"),
        ("sd/database_artwork.log", ".compas/database_artwork.log"),
        ("sd/usb_dac_bridge.log", ".compas/usb_dac_bridge.log"),
    ]
    .map(|(n, p)| (n.to_string(), format!("{SD_ROOT}/{p}")));
    internal.into_iter().chain(card).collect()
}

/// Commands whose output goes into the report: `(name, command)`.
const DIAGNOSTICS: [(&str, &str); 6] = [
    ("diagnostics/ps.txt", "ps"),
    ("diagnostics/dmesg.txt", "dmesg | tail -n 1000"),
    ("diagnostics/kernel.txt", "uname -a; cat /proc/version"),
    (
        "diagnostics/device.json",
        "cat /data/mnt/sd_0/.compas/device.json",
    ),
    ("diagnostics/free_space.txt", "df -h"),
    ("diagnostics/mounts.txt", "mount"),
];

#[derive(Debug, Clone, Serialize)]
pub struct LogFile {
    /// Name inside the report.
    pub name: String,
    /// Where it came from on the device, or the command that made it.
    pub source: String,
    /// What would be saved: already redacted.
    pub content: String,
    pub bytes: usize,
    /// Places where something secret was masked.
    pub redactions: usize,
    /// The file does not exist on this device.
    pub missing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogBundle {
    pub files: Vec<LogFile>,
    /// A few lines about the device, for the top of an issue.
    pub summary: String,
}

fn file_of(name: &str, source: &str, raw: Option<String>) -> LogFile {
    match raw {
        Some(text) => {
            let masked = redact(&text);
            LogFile {
                name: name.into(),
                source: source.into(),
                bytes: masked.text.len(),
                redactions: masked.count,
                content: masked.text,
                missing: false,
            }
        }
        None => LogFile {
            name: name.into(),
            source: source.into(),
            content: String::new(),
            bytes: 0,
            redactions: 0,
            missing: true,
        },
    }
}

pub async fn collect_logs(
    adb: &Adb,
    serial: &str,
    info: &DeviceInfo,
    on_progress: OnProgress<'_>,
) -> Result<LogBundle> {
    let paths = log_paths();
    let total = (paths.len() + DIAGNOSTICS.len()) as u64;
    let mut files = Vec::new();

    for (i, (name, path)) in paths.iter().enumerate() {
        on_progress(Progress {
            phase: Phase::Collecting,
            done: i as u64,
            total,
        });
        let command = format!("[ -f {path} ] && tail -c {TAIL_BYTES} {path} || echo {MISSING}");
        let output = adb.shell(serial, &command).await?;
        let raw = (!output.contains(MISSING)).then_some(output);
        files.push(file_of(name, path, raw));
    }
    for (i, (name, command)) in DIAGNOSTICS.iter().enumerate() {
        on_progress(Progress {
            phase: Phase::Collecting,
            done: (paths.len() + i) as u64,
            total,
        });
        // Some commands may be absent on a given firmware; that only means less to read.
        let output = adb
            .shell(serial, command)
            .await
            .ok()
            .filter(|o| !o.trim().is_empty());
        files.push(file_of(name, command, output));
    }
    on_progress(Progress {
        phase: Phase::Collecting,
        done: total,
        total,
    });

    let summary = format!(
        "Board: {}\nFirmware: {:?}\nPlayer version: {}",
        info.board.map_or("unknown", |b| b.board()),
        info.firmware,
        info.player_version.as_deref().unwrap_or("unknown"),
    );
    Ok(LogBundle { files, summary })
}

/// Writes the zip: exactly the files shown, plus a `manifest.txt` listing them.
pub fn write_zip(bundle: &LogBundle, path: &Path) -> Result<()> {
    let zip_error = |e: zip::result::ZipError| Error::Verification(e.to_string());
    let file = File::create(path).at(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    let mut manifest = format!("{}\n\nIncluded (secrets are masked):\n", bundle.summary);
    for f in bundle.files.iter().filter(|f| !f.missing) {
        manifest.push_str(&format!(
            "{}: {} bytes, {} masked\n",
            f.name, f.bytes, f.redactions
        ));
    }
    zip.start_file("manifest.txt", options).map_err(zip_error)?;
    zip.write_all(manifest.as_bytes()).at(path)?;
    for f in bundle.files.iter().filter(|f| !f.missing) {
        zip.start_file(&f.name, options).map_err(zip_error)?;
        zip.write_all(f.content.as_bytes()).at(path)?;
    }
    zip.finish().map_err(zip_error)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::io::Read;

    use super::super::fake::{FakeAdb, PS_COMPAS};
    use super::super::identify::identify;
    use super::*;

    async fn collect(fake: &FakeAdb) -> LogBundle {
        let adb = fake.adb();
        let info = identify(&adb, "s1").await.unwrap();
        collect_logs(&adb, "s1", &info, &|_| {}).await.unwrap()
    }

    fn file<'a>(bundle: &'a LogBundle, name: &str) -> &'a LogFile {
        bundle.files.iter().find(|f| f.name == name).unwrap()
    }

    #[tokio::test]
    async fn collects_the_listed_logs_masks_secrets_and_marks_missing_ones() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
        fake.write_remote(
            "/usr/data/player_crash.log",
            "crash at 12:00\nremote_control_pin=424242\nconnected ssid=\"Home Net\" psk=hunter2\nuser ana@example.com ip 192.168.1.50\n",
        );
        fake.write_remote("/data/mnt/sd_0/.compas/ota/update.log", "update ok\n");
        fake.write_remote("/usr/bin/compas_player", "ELF player v1.2.0");

        let bundle = collect(&fake).await;
        assert_eq!(bundle.files.len(), 11 + 6);

        let crash = file(&bundle, "usr_data/player_crash.log");
        assert!(crash.content.contains("crash at 12:00"));
        for secret in [
            "424242",
            "Home Net",
            "hunter2",
            "ana@example.com",
            "192.168.1.50",
        ] {
            assert!(!crash.content.contains(secret), "{secret} leaked");
        }
        assert_eq!(crash.redactions, 5);

        assert_eq!(file(&bundle, "sd/ota_update.log").content, "update ok\n");
        assert!(file(&bundle, "usr_data/player_crash.log.1").missing);
        assert!(file(&bundle, "sd/usb_dac_bridge.log").missing);
        assert!(!file(&bundle, "diagnostics/ps.txt").missing);
        let dmesg = file(&bundle, "diagnostics/dmesg.txt");
        assert!(
            dmesg.content.contains("xx:xx:xx:xx:xx:xx"),
            "{}",
            dmesg.content
        );
        assert!(bundle.summary.contains("Board: r1") && bundle.summary.contains("1.2.0"));
    }

    #[tokio::test]
    async fn the_zip_holds_exactly_the_shown_files_and_a_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeAdb::new(dir.path(), "s1", "480,800", PS_COMPAS);
        fake.write_remote("/usr/data/boot_debug.log", "boot token=abc123def\n");
        let bundle = collect(&fake).await;

        let out = tempfile::tempdir().unwrap();
        let path = out.path().join("report.zip");
        write_zip(&bundle, &path).unwrap();

        let mut zip = zip::ZipArchive::new(File::open(&path).unwrap()).unwrap();
        let mut names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        names.sort();
        let mut expected: Vec<String> = bundle
            .files
            .iter()
            .filter(|f| !f.missing)
            .map(|f| f.name.clone())
            .collect();
        expected.push("manifest.txt".into());
        expected.sort();
        assert_eq!(names, expected);

        let mut boot = String::new();
        zip.by_name("usr_data/boot_debug.log")
            .unwrap()
            .read_to_string(&mut boot)
            .unwrap();
        assert_eq!(boot, "boot token=<redacted>\n");
        let mut manifest = String::new();
        zip.by_name("manifest.txt")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        assert!(manifest.contains("usr_data/boot_debug.log") && manifest.contains("1 masked"));
    }
}
