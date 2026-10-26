//! Facts about the player's SD card and recovery partition, from what the device prints.

use super::{Adb, SD_ROOT};
use crate::error::{Error, Result};

/// Free kilobytes and the mount point, from `df -k`. BusyBox wraps a long device name onto its
/// own line, so the figures are found by the `Use%` column rather than by line position.
pub(super) fn parse_df(df: &str) -> Option<(u64, String)> {
    let tokens: Vec<&str> = df.split_whitespace().collect();
    let percent = tokens.iter().position(|t| {
        t.strip_suffix('%')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    })?;
    let free = tokens.get(percent.checked_sub(1)?)?.parse().ok()?;
    Some((free, tokens.get(percent + 1)?.to_string()))
}

#[cfg(all(test, unix))]
pub(super) fn parse_free_kb(df: &str) -> Option<u64> {
    parse_df(df).map(|(free, _)| free)
}

/// Whether `/proc/mounts` shows the SD card as a block device mounted where the player keeps it.
pub(super) fn sd_in_mounts(mounts: &str) -> bool {
    mounts.lines().any(|line| {
        let mut fields = line.split_whitespace();
        let (source, point) = (fields.next(), fields.next());
        matches!(point, Some("/usr/data/mnt/sd_0" | "/data/mnt/sd_0"))
            && source.is_some_and(|s| s.starts_with("/dev/") && s != "/dev/root")
    })
}

/// Proves that the SD card, not internal storage or RAM, is what `SD_ROOT` leads to right now:
/// it is a mounted block device and `df` reports that very mount point. Returns the free
/// kilobytes. Called again right before every step that depends on it, because a card can be
/// pulled at any time.
pub(super) async fn verify_card(adb: &Adb, serial: &str) -> Result<u64> {
    let mounts = adb.shell(serial, "cat /proc/mounts").await?;
    if !sd_in_mounts(&mounts) {
        return Err(Error::Rejected(
            "the player's SD card is not mounted, so nothing was copied".into(),
        ));
    }
    let df = adb.shell(serial, &format!("df -k {SD_ROOT}")).await?;
    match parse_df(&df) {
        Some((free, point)) if point.ends_with("/mnt/sd_0") => Ok(free),
        Some((_, point)) => Err(Error::Rejected(format!(
            "{SD_ROOT} is on {point}, not on the SD card, so nothing was copied"
        ))),
        None => Err(Error::Adb(format!(
            "could not read the free space on the player, so nothing was copied (df said: {})",
            df.trim()
        ))),
    }
}

/// Names are put into shell commands: only plain file-name characters are allowed.
pub(super) fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

/// What the player writes to the recovery flag block (`ota:kernel2`, padded).
pub(super) const RECOVERY_FLAG: &str = "ota:kernel2";

/// `/proc/mtd` must show the `ota` flag partition as `mtd5` with a usable size.
pub(super) fn recovery_partition_ok(mtd: &str) -> bool {
    mtd.lines().any(|line| {
        let mut fields = line.split_whitespace();
        let (Some(dev), Some(size), Some(erase), Some(name)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return false;
        };
        let hex = |s: &str| u64::from_str_radix(s, 16).unwrap_or(0);
        dev == "mtd5:" && name == "\"ota\"" && hex(size) >= 256 && hex(erase) >= 256
    })
}
