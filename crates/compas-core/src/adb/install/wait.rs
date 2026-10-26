//! Waiting for the player to restart and come back.

use std::time::Instant;

use super::{identify, Adb, DeviceInfo, Model, WaitOptions};

pub(super) enum Returned {
    /// The same serial, or the only device of the same board under a new one.
    Back(DeviceInfo),
    /// Several look like the same board; the user must say which is theirs.
    Candidates(Vec<DeviceInfo>),
    TimedOut,
}

/// Waits for the device to disappear (it is rebooting) and to come back, then looks at it again.
/// Other connected devices are ignored: only the same serial counts, or, if the serial changed
/// after recovery, a player-looking device that identifies as the same board and was not
/// already connected before the reboot.
pub(super) async fn wait_for_return(
    adb: &Adb,
    serial: &str,
    board: Model,
    wait: &WaitOptions,
    already_connected: &[String],
) -> Returned {
    let online = || async {
        adb.devices()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|d| d.state == "device")
            .collect::<Vec<_>>()
    };
    let gone_by = Instant::now() + wait.going_down;
    while online().await.iter().any(|d| d.serial == serial) && Instant::now() < gone_by {
        tokio::time::sleep(wait.poll).await;
    }
    let give_up = Instant::now() + wait.coming_back;
    loop {
        let devices = online().await;
        if devices.iter().any(|d| d.serial == serial) {
            if let Ok(info) = identify(adb, serial).await {
                return Returned::Back(info);
            }
        } else {
            let mut same_board = Vec::new();
            for device in devices
                .iter()
                .filter(|d| d.looks_like_player() && !already_connected.contains(&d.serial))
            {
                if let Ok(info) = identify(adb, &device.serial).await {
                    if info.board == Some(board) {
                        same_board.push(info);
                    }
                }
            }
            match same_board.len() {
                0 => {}
                1 => return Returned::Back(same_board.remove(0)),
                _ => return Returned::Candidates(same_board),
            }
        }
        if Instant::now() >= give_up {
            return Returned::TimedOut;
        }
        tokio::time::sleep(wait.poll).await;
    }
}
