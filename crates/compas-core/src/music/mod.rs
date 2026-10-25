//! Putting music on the card: planning, verified copying, FAT rules and
//! macOS clutter.

mod clutter;
mod copy;
mod names;
mod plan;

use crate::card::Card;
use crate::error::Result;

pub use clutter::{is_clutter_name, tidy, TidyReport};
pub use copy::{copy_plan, CopyReport, Failure};
pub use names::fat_safe_name;
pub use plan::{plan, Issue, IssueKind, Plan, PlanItem, PlanOptions, PlanSummary, Rename};

/// FAT32 holds files up to 4 GiB minus one byte.
const FAT32_MAX_FILE: u64 = u32::MAX as u64;

/// The largest file the card's file system takes, when it has a limit.
pub fn max_file_bytes(card: &Card) -> Result<Option<u64>> {
    let fs = card.info()?.file_system.to_ascii_lowercase();
    let fat32 = fs.contains("fat") && !fs.contains("exfat");
    Ok(fat32.then_some(FAT32_MAX_FILE))
}
