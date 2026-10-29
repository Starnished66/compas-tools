//! Which player is on the other end of the cable, and what runs on it.

use std::collections::HashMap;

use serde::Serialize;

use super::{Adb, AdbDevice};
use crate::card::{Device, Model};
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Firmware {
    /// The vendor's `hiby_player`.
    Stock,
    /// Compás: `compas_player` and its bootloader.
    Compas,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub serial: String,
    /// From the board marker in a Compás program, else the screen size; the product name over
    /// ADB (`Nexus_4`, `mako`) does not say.
    pub board: Option<Model>,
    pub firmware: Firmware,
    /// Read from the running Compás program (`vX.Y.Z`); unknown when it cannot be told.
    pub player_version: Option<String>,
    pub plugin_api: Option<u32>,
    /// Whether the SD card is mounted; an update needs it.
    pub sd_mounted: bool,
}

/// `adb devices` output: a header, then `<serial>\t<state>` lines.
pub(super) fn parse_devices(output: &str) -> Vec<AdbDevice> {
    output
        .lines()
        .filter(|l| !l.starts_with("List of devices") && !l.starts_with('*'))
        .filter_map(|line| {
            let (serial, rest) = line.trim().split_once(char::is_whitespace)?;
            // `adb devices -l`: `<serial> <state> product:mako model:Nexus_4 device:mako ...`
            let rest = rest.trim();
            let head = rest.split_once(" product:").map_or(rest, |(head, _)| head);
            let state = if head.starts_with("no permissions") {
                "no permissions"
            } else {
                head.split_whitespace().next().unwrap_or(head)
            };
            let extras = rest.to_string();
            let field = |name: &str| {
                extras
                    .split_whitespace()
                    .find_map(|w| w.strip_prefix(name))
                    .map(str::to_string)
            };
            Some(AdbDevice {
                serial: serial.to_string(),
                state: state.to_string(),
                product: field("product:"),
                model: field("model:"),
            })
        })
        .collect()
}

/// `virtual_size` reads `480,800`; a doubled height (two frame buffers) counts too.
pub(super) fn board_from_size(text: &str) -> Option<Model> {
    let numbers: Vec<u32> = text
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    let [width, height] = numbers[..] else {
        return None;
    };
    let fits = |h: u32| height >= h && height % h == 0 && height / h <= 3;
    match width {
        480 if fits(800) => Some(Model::R1),
        480 if fits(720) => Some(Model::R3ProII),
        320 if fits(480) => Some(Model::R3II2025),
        240 if fits(320) => Some(Model::TempotecV1),
        _ => None,
    }
}

/// The process list tells the two firmwares apart.
pub(super) fn firmware_from_processes(ps: &str) -> Firmware {
    let ps = ps.to_ascii_lowercase();
    if ps.contains("compas_player") || ps.contains("compas_boot") {
        Firmware::Compas
    } else if ps.contains("hiby_player") {
        Firmware::Stock
    } else {
        Firmware::Unknown
    }
}

/// The running player program from the `ps` output; the firmware installs it as
/// `/usr/bin/compas_player`, a player-only install may put it elsewhere. The device script
/// ([`SCRIPT`]) applies the same rule on the player; this is the reference the tests hold it to.
#[cfg(test)]
pub(super) fn player_binary(ps: &str) -> String {
    const DEFAULT: &str = "/usr/bin/compas_player";
    ps.split_whitespace()
        .find(|w| {
            w.starts_with('/')
                && w.ends_with("compas_player")
                && w.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
        })
        .unwrap_or(DEFAULT)
        .to_string()
}

/// Everything [`identify`] needs, read with a single `adb shell` (each adb call costs a round
/// trip over USB, and the player's shell starts slowly). The answer is cut into sections by
/// lines starting with `@@` (`m` prints one after a line break, so output without a final newline cannot hide it). The program is read only for a Compás firmware, with one `grep`
/// for both the `vX.Y.Z` label and the board marker.
const SCRIPT: &str = r#"m() { printf '\n@@%s\n' "$1"; }
m size
cat /sys/class/graphics/fb0/virtual_size 2>/dev/null
m ps
P=$(ps 2>/dev/null)
echo "$P"
m device
cat /data/mnt/sd_0/.compas/device.json 2>/dev/null
m mounted
if grep -q 'mnt/sd_0 ' /proc/mounts 2>/dev/null; then echo mounted; fi
m program
L=$(echo "$P" | tr 'A-Z' 'a-z')
case "$L" in
*compas_player*|*compas_boot*)
  B=/usr/bin/compas_player
  set -f
  for w in $P; do
    case "$w" in
      *[!A-Za-z0-9/._-]*) ;;
      /*compas_player) B=$w; break ;;
    esac
  done
  if [ -r "$B" ]; then
    grep -a -o -e 'v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*' -e 'COMPAS_PLAYER_BOARD:[a-z0-9_]*' "$B" | sort -u
  fi
  ;;
esac
"#;

/// The sections of [`SCRIPT`]'s output by name; text before the first marker is dropped.
fn sections(output: &str) -> HashMap<&str, String> {
    let mut found: HashMap<&str, String> = HashMap::new();
    let mut current: Option<&str> = None;
    for line in output.lines() {
        if let Some(name) = line.trim_end().strip_prefix("@@") {
            current = Some(name);
            found.entry(name).or_default();
        } else if let Some(name) = current {
            let text = found.entry(name).or_default();
            text.push_str(line);
            text.push('\n');
        }
    }
    found
}

/// The single value of a kind in the program's `grep` output; none or several mean unknown.
fn only<'a>(lines: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let mut lines = lines.filter(|l| !l.is_empty());
    match (lines.next(), lines.next()) {
        (Some(one), None) => Some(one),
        _ => None,
    }
}

/// Reads the answer of [`SCRIPT`]. An empty or garbled answer gives an unknown player.
pub(super) fn parse_identify(serial: &str, output: &str) -> DeviceInfo {
    let sections = sections(output);
    let text = |name: &str| sections.get(name).map_or("", String::as_str);
    let device: Option<Device> = serde_json::from_str(text("device").trim()).ok();
    let firmware = firmware_from_processes(text("ps"));
    let program = || text("program").lines().map(str::trim);
    // The version comes from the running program itself, not from a file on the card.
    let (player_version, marker_board) = if firmware == Firmware::Compas {
        let version = only(program().filter(|l| l.starts_with('v'))).map(str::to_string);
        let board = only(program().filter_map(|l| l.strip_prefix("COMPAS_PLAYER_BOARD:")))
            .and_then(Model::from_board);
        (version, board)
    } else {
        (None, None)
    };
    DeviceInfo {
        serial: serial.to_string(),
        // A Compás program names its own board; otherwise the screen size. The card's
        // device.json is never used: a card moved between players describes another one.
        board: marker_board.or_else(|| board_from_size(text("size"))),
        firmware,
        player_version,
        plugin_api: device.as_ref().map(|d| d.plugin_api),
        sd_mounted: text("mounted").trim() == "mounted",
    }
}

/// Identifies the player; when the shell does not answer the result is an unknown player.
pub async fn identify(adb: &Adb, serial: &str) -> Result<DeviceInfo> {
    // The install path waits as long as it always did.
    let output = adb.shell(serial, SCRIPT).await;
    Ok(parse_identify(serial, &output.unwrap_or_default()))
}

/// Like [`identify`], but for the background look: a shell that does not answer within a
/// few seconds is an error, so the caller can try again later instead of keeping an unknown
/// player.
pub async fn try_identify(adb: &Adb, serial: &str) -> Result<DeviceInfo> {
    let output = adb.shell_for_identify(serial, SCRIPT).await?;
    Ok(parse_identify(serial, &output))
}

#[cfg(test)]
mod tests;
