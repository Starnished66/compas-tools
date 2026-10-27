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
mod tests;
