//! Which player is on the other end of the cable, and what runs on it.

use serde::Serialize;

use super::{Adb, AdbDevice, SD_ROOT};
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
    /// From the screen size; the product name over ADB (`Nexus_4`, `mako`) does not say.
    pub board: Option<Model>,
    pub firmware: Firmware,
    /// From `.compas/device.json` on the card, when the player has written one.
    pub player_version: Option<String>,
    pub plugin_api: Option<u32>,
}

/// `adb devices` output: a header, then `<serial>\t<state>` lines.
pub(super) fn parse_devices(output: &str) -> Vec<AdbDevice> {
    output
        .lines()
        .filter(|l| !l.starts_with("List of devices") && !l.starts_with('*'))
        .filter_map(|line| {
            let (serial, state) = line.trim().split_once(char::is_whitespace)?;
            Some(AdbDevice {
                serial: serial.to_string(),
                state: state.trim().to_string(),
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

pub async fn identify(adb: &Adb, serial: &str) -> Result<DeviceInfo> {
    let size = adb
        .shell(
            serial,
            "cat /sys/class/graphics/fb0/virtual_size 2>/dev/null",
        )
        .await
        .unwrap_or_default();
    let processes = adb.shell(serial, "ps").await.unwrap_or_default();
    let device_json = adb
        .shell(
            serial,
            &format!("cat {SD_ROOT}/.compas/device.json 2>/dev/null"),
        )
        .await
        .unwrap_or_default();
    let device: Option<Device> = serde_json::from_str(device_json.trim()).ok();

    Ok(DeviceInfo {
        serial: serial.to_string(),
        board: board_from_size(&size).or(device.as_ref().map(|d| d.model)),
        firmware: firmware_from_processes(&processes),
        player_version: device.as_ref().map(|d| d.player_version.clone()),
        plugin_api: device.as_ref().map(|d| d.plugin_api),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_device_list_with_odd_serials_and_states() {
        let out = "List of devices attached\ningenic_1234abcd\tdevice\n0123456789ABCDEF\tunauthorized\nemulator-5554\toffline\n\n";
        let devices = parse_devices(out);
        assert_eq!(devices.len(), 3);
        assert_eq!(
            devices[0],
            AdbDevice {
                serial: "ingenic_1234abcd".into(),
                state: "device".into()
            }
        );
        assert_eq!(devices[1].state, "unauthorized");
        assert!(parse_devices("List of devices attached\n\n").is_empty());
        let daemon = "* daemon not running; starting now at tcp:5037\n* daemon started successfully\nList of devices attached\nabc\tdevice\n";
        assert_eq!(parse_devices(daemon).len(), 1);
    }

    #[test]
    fn the_screen_size_names_the_board() {
        assert_eq!(board_from_size("480,800\n"), Some(Model::R1));
        assert_eq!(board_from_size("480x720"), Some(Model::R3ProII));
        assert_eq!(board_from_size("320,480"), Some(Model::R3II2025));
        assert_eq!(
            board_from_size("480,1600"),
            Some(Model::R1),
            "double-buffered"
        );
        assert_eq!(board_from_size("1080,1920"), None);
        assert_eq!(board_from_size(""), None);
        assert_eq!(board_from_size("480"), None);
    }

    #[test]
    fn processes_tell_stock_from_compas() {
        let stock = "  PID USER COMMAND\n  101 root /usr/bin/hiby_player\n  202 root adbd\n";
        let compas = "  PID USER COMMAND\n  101 root /usr/bin/compas_player\n  202 root adbd\n";
        let bootloader = "  PID USER COMMAND\n  50 root /usr/bin/compas_bootloader\n";
        assert_eq!(firmware_from_processes(stock), Firmware::Stock);
        assert_eq!(firmware_from_processes(compas), Firmware::Compas);
        assert_eq!(firmware_from_processes(bootloader), Firmware::Compas);
        assert_eq!(
            firmware_from_processes("  1 root init\n"),
            Firmware::Unknown
        );
    }
}
