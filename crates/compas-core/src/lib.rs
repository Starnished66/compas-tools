//! Compás Tools core: everything the app does to a player's SD card.
//!
//! No Tauri dependency; the desktop app only adapts these functions into commands.

pub mod card;
pub mod catalog;
pub mod error;
pub mod fs;
pub mod music;
pub mod net;
pub mod progress;
pub mod releases;
pub mod sdtool;
pub mod transfer;
pub mod version;

#[cfg(test)]
mod test_support;

pub use error::{Error, Result};
pub use progress::{OnProgress, Phase, Progress};
