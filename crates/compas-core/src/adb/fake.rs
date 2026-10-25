//! A fake `adb` for tests: a shell script serving a pretend player.
//!
//! The pretend device is a folder (`remote/`) holding `/usr/data`, `/data/mnt/sd_0` and so on;
//! `shell` commands run against it with their absolute paths remapped, and `pull` and
//! `push` copy to and from it. Canned outputs stand in for `ps`, the screen size and the
//! device list, as recorded from a stock and a Compás player. Every call is logged.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::Adb;

const SCRIPT: &str = r##"#!/bin/sh
D="@DIR@"
echo "$*" >> "$D/calls.log"
[ "$1" = "-s" ] && shift 2
cmd="$1"; shift
R="$D/remote"
map() { echo "$1" | sed -E "s#(^|[ '=])/(usr|data|sys|proc|tmp|dev|run)/#\1$R/\2/#g"; }
case "$cmd" in
  devices) cat "$D/devices.txt" ;;
  version) echo "Android Debug Bridge version 1.0.41" ;;
  pull)
    src="$R$1"; dst="$2"
    if [ -d "$src" ]; then
      if [ -d "$dst" ]; then cp -R "$src" "$dst/"; else cp -R "$src" "$dst"; fi
    else cp "$src" "$dst"; fi ;;
  push)
    dst="$R$2"; mkdir -p "$(dirname "$dst")"
    if [ -e "$D/corrupt_push" ]; then echo junk > "$dst"; else cp "$1" "$dst"; fi ;;
  shell)
    c="$1"
    case "$c" in
      /usr/bin/bootmode.sh*)
        [ -e "$D/bootmode_fails" ] && { echo "/usr/bin/bootmode.sh: not found" >&2; exit 1; }
        echo "ok" ;;
      sync|reboot) : ;;
      "cat /sys/class/graphics/fb0/virtual_size"*) cat "$D/fb" 2>/dev/null ;;
      ps) cat "$D/ps" 2>/dev/null ;;
      dmesg*) cat "$D/dmesg" 2>/dev/null ;;
      *) eval "$(map "$c")" | sed "s#$R##g" ;;
    esac ;;
  *) echo "fake adb: unknown command $cmd" >&2; exit 1 ;;
esac
"##;

pub(crate) struct FakeAdb {
    dir: PathBuf,
}

/// Recorded `ps` output of a stock HiBy firmware.
pub(crate) const PS_STOCK: &str = "  PID USER     COMMAND\n    1 root     /sbin/init\n  214 root     /usr/bin/hiby_player\n  230 root     /sbin/adbd\n";
/// And of a Compás firmware.
pub(crate) const PS_COMPAS: &str = "  PID USER     COMMAND\n    1 root     /sbin/init\n  198 root     /usr/bin/compas_bootloader\n  214 root     /usr/bin/compas_player\n  230 root     /sbin/adbd\n";

impl FakeAdb {
    /// A device named `serial` that is `board_size` pixels, running `ps`.
    pub(crate) fn new(dir: &Path, serial: &str, board_size: &str, ps: &str) -> Self {
        fs::create_dir_all(dir.join("remote")).unwrap();
        fs::write(
            dir.join("fake-adb"),
            SCRIPT.replace("@DIR@", &dir.display().to_string()),
        )
        .unwrap();
        fs::set_permissions(dir.join("fake-adb"), fs::Permissions::from_mode(0o755)).unwrap();
        let fake = Self {
            dir: dir.to_path_buf(),
        };
        fake.set(
            "devices.txt",
            &format!("List of devices attached\n{serial}\tdevice\n\n"),
        );
        fake.set("fb", board_size);
        fake.set("ps", ps);
        fake.set("dmesg", "[    0.000000] Linux version 3.10.14\n[   12.5] wlan0: connected to AP mac aa:bb:cc:dd:ee:ff\n");
        fake
    }

    pub(crate) fn adb(&self) -> Adb {
        Adb::new(self.dir.join("fake-adb"))
    }

    /// Sets a canned output (`devices.txt`, `fb`, `ps`, `dmesg`) or a switch file
    /// (`corrupt_push`, `bootmode_fails`).
    pub(crate) fn set(&self, name: &str, content: &str) {
        fs::write(self.dir.join(name), content).unwrap();
    }

    /// Creates a file on the pretend device, e.g. `/usr/data/settings.ini`.
    pub(crate) fn write_remote(&self, path: &str, content: &str) {
        let target = self.remote(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, content).unwrap();
    }

    pub(crate) fn remote(&self, path: &str) -> PathBuf {
        self.dir.join("remote").join(path.trim_start_matches('/'))
    }

    /// Every adb invocation so far, one line each.
    pub(crate) fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("calls.log"))
            .map(|t| t.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }
}
