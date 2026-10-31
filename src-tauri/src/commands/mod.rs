//! One file per feature area; each command is a thin wrapper over compas-core.

use tauri::ipc::Channel;

use compas_core::Progress;

pub mod about;
pub mod adb;
pub mod app_update;
#[cfg(feature = "updater")]
mod app_update_feed;
pub mod card;
pub mod catalog;
pub mod library;
pub mod music;
pub mod plugins;
pub mod releases;
pub mod settings;

/// Adapts a Tauri channel into core's progress callback.
fn reporter(channel: Channel<Progress>) -> impl Fn(Progress) + Send + Sync {
    move |progress| {
        let _ = channel.send(progress);
    }
}
