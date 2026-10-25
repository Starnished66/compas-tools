//! One file per feature area; each command is a thin wrapper over compas-core.

use tauri::ipc::Channel;

use compas_core::Progress;

pub mod card;
pub mod catalog;
pub mod releases;

/// Adapts a Tauri channel into core's progress callback.
fn reporter(channel: Channel<Progress>) -> impl Fn(Progress) + Send + Sync {
    move |progress| {
        let _ = channel.send(progress);
    }
}
