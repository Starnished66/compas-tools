use std::collections::HashSet;

use super::{Card, CardInfo};
use crate::fs::volumes;

/// Mounted volumes whose root holds a `.compas/` folder.
pub fn discover() -> Vec<CardInfo> {
    let mut seen = HashSet::new();
    volumes()
        .into_iter()
        .filter(|v| seen.insert(v.mount_point.clone()))
        .filter_map(|v| Card::open(v.mount_point).ok()?.info().ok())
        .collect()
}
