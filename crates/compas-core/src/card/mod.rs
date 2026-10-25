//! A Compás SD card: discovery, identity (`device.json`) and safe paths.

mod device;
mod discover;
mod leftovers;
mod paths;

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, IoContext, Result};
use crate::fs::{ensure_real_dir, volume_for};

pub use device::{Device, Model};
pub use discover::discover;

/// A mounted card whose root contains the player's `.compas/` folder.
#[derive(Debug, Clone)]
pub struct Card {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct CardInfo {
    pub label: String,
    pub root: PathBuf,
    pub file_system: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    /// `None` when the player has not written `device.json`.
    pub device: Option<Device>,
}

impl Card {
    /// Validates a mount point or a folder picked by the user.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        // A link in place of `.compas` could lead every operation off the card.
        let compas = std::fs::symlink_metadata(root.join(".compas"));
        if !compas.is_ok_and(|m| m.is_dir()) {
            return Err(Error::InvalidCard(format!(
                "{} has no .compas folder",
                root.display()
            )));
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn compas_dir(&self) -> PathBuf {
        self.root.join(".compas")
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.root.join(".plugins")
    }

    pub fn device(&self) -> Result<Option<Device>> {
        device::read(self)
    }

    pub fn info(&self) -> Result<CardInfo> {
        let volume = volume_for(&self.root);
        let label = volume
            .as_ref()
            .map(|v| v.name.clone())
            .filter(|n| !n.is_empty())
            .or_else(|| {
                self.root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| self.root.display().to_string());
        Ok(CardInfo {
            label,
            root: self.root.clone(),
            file_system: volume
                .as_ref()
                .map(|v| v.file_system.clone())
                .unwrap_or_default(),
            total_bytes: volume.as_ref().map_or(0, |v| v.total_bytes),
            free_bytes: volume.as_ref().map_or(0, |v| v.available_bytes),
            device: self.device()?,
        })
    }

    /// Absolute path of a card-relative `dest`; see [`paths::resolve`].
    pub fn resolve(&self, dest: &str, create_parents: bool) -> Result<PathBuf> {
        paths::resolve(&self.root, dest, create_parents)
    }

    /// Creates `.plugins/` if needed, refusing a symlink in its place.
    pub fn ensure_plugins_dir(&self) -> Result<PathBuf> {
        let dir = self.plugins_dir();
        ensure_real_dir(&dir).at(&dir)?;
        Ok(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fake_card;

    #[test]
    fn open_requires_compas_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(Card::open(dir.path()), Err(Error::InvalidCard(_))));
        std::fs::create_dir(dir.path().join(".compas")).unwrap();
        assert!(Card::open(dir.path()).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn open_refuses_a_symlinked_compas_folder() {
        let (dir, outside) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        std::os::unix::fs::symlink(outside.path(), dir.path().join(".compas")).unwrap();
        assert!(matches!(Card::open(dir.path()), Err(Error::InvalidCard(_))));
    }

    #[test]
    fn info_without_device_json_is_unknown_not_an_error() {
        let (dir, card) = fake_card();
        let info = card.info().unwrap();
        assert!(info.device.is_none());
        assert_eq!(info.root, dir.path());
    }
}
