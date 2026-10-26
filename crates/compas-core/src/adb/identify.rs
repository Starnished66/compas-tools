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
/// `/usr/bin/compas_player`, a player-only install may put it elsewhere.
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

/// The version label baked into the program: exactly one `vX.Y.Z` string in it. A development
/// build, several versions, no program or any adb error mean it is not known (never guessed).
async fn version_from_binary(adb: &Adb, serial: &str, binary: &str) -> Option<String> {
    let out = adb
        .shell(
            serial,
            &format!(
                "if [ -r {binary} ]; then grep -a -o 'v[0-9][0-9]*\\.[0-9][0-9]*\\.[0-9][0-9]*' {binary} | sort -u; fi"
            ),
        )
        .await
        .ok()?;
    let mut found = out.lines().map(str::trim).filter(|l| !l.is_empty());
    match (found.next(), found.next()) {
        (Some(version), None) => Some(version.to_string()),
        _ => None,
    }
}

/// Whether the SD card is mounted: updates are staged on it. `/proc/mounts` names it
/// `/usr/data/mnt/sd_0` (and `/data/mnt/sd_0` is the same place).
async fn sd_is_mounted(adb: &Adb, serial: &str) -> bool {
    adb.shell(
        serial,
        "if grep -q 'mnt/sd_0 ' /proc/mounts; then echo mounted; fi",
    )
    .await
    .is_ok_and(|out| out.trim() == "mounted")
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

    let firmware = firmware_from_processes(&processes);
    // The version comes from the running program itself, not from a file on the card.
    let player_version = if firmware == Firmware::Compas {
        version_from_binary(adb, serial, &player_binary(&processes)).await
    } else {
        None
    };
    let sd_mounted = sd_is_mounted(adb, serial).await;

    Ok(DeviceInfo {
        serial: serial.to_string(),
        board: board_from_size(&size).or(device.as_ref().map(|d| d.model)),
        firmware,
        player_version,
        plugin_api: device.as_ref().map(|d| d.plugin_api),
        sd_mounted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_device_list_with_odd_serials_and_states() {
        let out = "List of devices attached\ningenic_1234abcd        device usb:1-1 product:mako model:Nexus_4 device:mako transport_id:1\n0123456789ABCDEF       unauthorized usb:1-2 transport_id:2\nemulator-5554\toffline\nabc no permissions (user is not in the plugdev group); see [http://developer.android.com/tools/device.html] usb:1-3 transport_id:3\n\n";
        let devices = parse_devices(out);
        assert_eq!(devices.len(), 4);
        assert_eq!(devices[0].serial, "ingenic_1234abcd");
        assert_eq!(devices[0].state, "device");
        assert_eq!(devices[0].product.as_deref(), Some("mako"));
        assert_eq!(devices[0].model.as_deref(), Some("Nexus_4"));
        assert_eq!(devices[1].state, "unauthorized");
        assert_eq!(devices[1].product, None);
        assert_eq!(devices[2].state, "offline");
        assert_eq!(devices[3].state, "no permissions");
        assert!(parse_devices("List of devices attached\n\n").is_empty());
        let daemon = "* daemon not running; starting now at tcp:5037\n* daemon started successfully\nList of devices attached\nabc\tdevice\n";
        assert_eq!(parse_devices(daemon).len(), 1);
    }

    #[test]
    fn only_devices_that_look_like_the_player_are_probed() {
        let device = |serial: &str, product: Option<&str>, model: Option<&str>| AdbDevice {
            serial: serial.into(),
            state: "device".into(),
            product: product.map(str::to_string),
            model: model.map(str::to_string),
        };
        assert!(device("ingenic_9f3a", None, None).looks_like_player());
        assert!(device("0123456789", Some("mako"), None).looks_like_player());
        assert!(device("0123456789", None, Some("Nexus_4")).looks_like_player());
        assert!(!device("R58M12345AB", Some("beyond1lte"), Some("SM_G973F")).looks_like_player());
        assert!(!device("emulator-5554", None, None).looks_like_player());
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
