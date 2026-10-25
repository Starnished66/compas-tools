use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Downloading,
    Installing,
    Removing,
    Updating,
    /// `compas-sdtool` walking the card's folders.
    Scanning,
    ReadingTags,
    Saving,
    BuildingArt,
    Copying,
    Cleaning,
    BackingUp,
    Pushing,
    Verifying,
    Rebooting,
    /// Waiting for the device to come back after a reboot.
    Waiting,
    Collecting,
}

/// `done` and `total` are bytes while downloading and items otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub phase: Phase,
    pub done: u64,
    pub total: u64,
}

pub type OnProgress<'a> = &'a (dyn Fn(Progress) + Send + Sync);
