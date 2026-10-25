//! Talking to a player over ADB. Every adb call in the app goes through [`Adb`].

mod backup;
#[cfg(all(test, unix))]
pub(crate) mod fake;
mod identify;
mod install;
mod locate;
mod logs;
mod redact;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use tokio::process::Command;

use crate::error::{Error, Result};

pub use backup::{backup_user_data, BackupReport, SkipReason, Skipped};
pub use identify::{identify, DeviceInfo, Firmware};
pub use install::{
    install_full_image, install_player_only, remove_staged_image, InstallReport, InstallRequest,
    WaitOptions,
};
pub use locate::{
    download_platform_tools, locate, tools_archive, Located, ToolsArchive, ToolsSource,
    PLATFORM_TOOLS_VERSION,
};
pub use logs::{collect_logs, write_zip, LogBundle, LogFile};
pub use redact::{is_secret_name, redact, Redacted};

/// Where the player mounts the SD card when seen over ADB.
pub const SD_ROOT: &str = "/data/mnt/sd_0";
/// The player's internal user data.
pub const USER_DATA: &str = "/usr/data";

const SHORT: Duration = Duration::from_secs(60);
const LONG: Duration = Duration::from_secs(60 * 30);

/// One line of `adb devices`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdbDevice {
    pub serial: String,
    /// `device`, `unauthorized`, `offline`, `no permissions`, ...
    pub state: String,
    /// From `adb devices -l`; the player reports `mako` and `Nexus_4`.
    pub product: Option<String>,
    pub model: Option<String>,
}

impl AdbDevice {
    /// Whether this looks like a Compás-capable player rather than somebody's phone:
    /// an Ingenic serial, or the product the player's Android-style USB gadget reports.
    /// Other devices are not touched unless the user asks.
    pub fn looks_like_player(&self) -> bool {
        let is = |value: &Option<String>, names: &[&str]| {
            value
                .as_deref()
                .is_some_and(|v| names.iter().any(|n| v.eq_ignore_ascii_case(n)))
        };
        self.serial.to_ascii_lowercase().starts_with("ingenic")
            || is(&self.product, &["mako"])
            || is(&self.model, &["Nexus_4"])
    }
}

#[derive(Debug, Clone)]
pub struct Adb {
    exe: PathBuf,
}

impl Adb {
    pub fn new(exe: impl Into<PathBuf>) -> Self {
        Self { exe: exe.into() }
    }

    pub fn exe(&self) -> &Path {
        &self.exe
    }

    async fn run(&self, args: &[&str], limit: Duration) -> Result<String> {
        let bytes = self.run_bytes(args, limit).await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    async fn run_bytes(&self, args: &[&str], limit: Duration) -> Result<Vec<u8>> {
        let mut command = Command::new(&self.exe);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000); // no console window

        let output = tokio::time::timeout(limit, crate::process::output(&mut command))
            .await
            .map_err(|_| Error::Adb(format!("adb {} timed out", args.first().unwrap_or(&""))))?
            .map_err(|e| Error::Adb(format!("{}: {e}", self.exe.display())))?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        Err(Error::Adb(detail.to_string()))
    }

    pub async fn version(&self) -> Result<String> {
        self.run(&["version"], SHORT).await
    }

    pub async fn devices(&self) -> Result<Vec<AdbDevice>> {
        Ok(identify::parse_devices(
            &self.run(&["devices", "-l"], SHORT).await?,
        ))
    }

    /// Runs a command on the device and returns what it printed.
    pub async fn shell(&self, serial: &str, command: &str) -> Result<String> {
        self.run(&["-s", serial, "shell", command], SHORT).await
    }

    /// A command's raw output, safe for binary data (`exec-out` does not translate line ends).
    pub async fn exec_out(&self, serial: &str, command: &str) -> Result<Vec<u8>> {
        self.run_bytes(&["-s", serial, "exec-out", command], SHORT)
            .await
    }

    pub async fn pull(&self, serial: &str, remote: &str, local: &Path) -> Result<()> {
        let local = local.to_string_lossy();
        self.run(&["-s", serial, "pull", remote, &local], LONG)
            .await
            .map(drop)
    }

    pub async fn push(&self, serial: &str, local: &Path, remote: &str) -> Result<()> {
        let local = local.to_string_lossy();
        self.run(&["-s", serial, "push", &local, remote], LONG)
            .await
            .map(drop)
    }
}

#[cfg(all(test, unix))]
mod tests;
