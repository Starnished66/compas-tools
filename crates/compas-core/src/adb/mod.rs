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

pub use backup::{backup_user_data, BackupReport};
pub use identify::{identify, DeviceInfo, Firmware};
pub use install::{
    install_full_image, install_player_only, InstallReport, InstallRequest, WaitOptions,
};
pub use locate::{download_platform_tools, locate, tools_url, Located, ToolsSource};
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
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            return Ok(text);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stderr.trim().is_empty() {
            text.trim()
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
            &self.run(&["devices"], SHORT).await?,
        ))
    }

    /// Runs a command on the device and returns what it printed.
    pub async fn shell(&self, serial: &str, command: &str) -> Result<String> {
        self.run(&["-s", serial, "shell", command], SHORT).await
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
