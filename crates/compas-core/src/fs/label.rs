//! Filesystem label of a block device. sysinfo only gives `/dev/sdb1`.

/// The volume label for `device` (e.g. `/dev/sdb1`) from `/dev/disk/by-label`.
#[cfg(target_os = "linux")]
pub(super) fn volume_label(device: &str) -> Option<String> {
    let device = std::path::Path::new(device).canonicalize().ok()?;
    std::fs::read_dir("/dev/disk/by-label")
        .ok()?
        .flatten()
        .find(|entry| {
            entry
                .path()
                .canonicalize()
                .is_ok_and(|target| target == device)
        })
        .map(|entry| unescape(&entry.file_name().to_string_lossy()))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn volume_label(_device: &str) -> Option<String> {
    None
}

/// udev escapes unusual characters in link names as `\xNN`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn unescape(name: &str) -> String {
    let mut bytes = Vec::new();
    let mut rest = name.as_bytes();
    while let Some((&b, tail)) = rest.split_first() {
        let code = (b == b'\\')
            .then(|| tail.strip_prefix(b"x").and_then(|t| t.get(..2)))
            .flatten()
            .and_then(|h| u8::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok());
        match code {
            Some(c) => {
                bytes.push(c);
                rest = &tail[3..];
            }
            None => {
                bytes.push(b);
                rest = tail;
            }
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests;
