use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::names::{fat_safe_name, name_too_long, player_path_fits};
use super::reserved::{is_firmware_image, is_hidden, reserved_destination};
use crate::card::Card;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct PlanOptions {
    /// Card folder the sources are copied into, `/`-separated; empty for the root.
    pub dest_dir: String,
    /// Mirror: also remove files from the card copy of the (single) source
    /// folder that the folder no longer has.
    pub delete_extra: bool,
    /// Largest file the card's file system holds (FAT32: just under 4 GiB).
    pub max_file_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PlanItem {
    pub source: PathBuf,
    /// Card-relative destination, `/`-separated.
    pub dest: String,
    pub size: u64,
    /// A file is already there; copying decides whether it is identical.
    pub exists: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    TooLargeForFat,
    NameTooLong,
    PathTooLong,
    NameConflict,
    Unreadable,
    /// A hidden name, a firmware image, or something the player owns.
    ReservedName,
}

/// Something that keeps one file out of the transfer.
#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub source: String,
    pub kind: IssueKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct Rename {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub items: Vec<PlanItem>,
    /// Card-relative files a mirror would remove.
    pub deletions: Vec<String>,
    pub issues: Vec<Issue>,
    /// Folder or file names that had to change to be valid on FAT.
    pub renames: Vec<Rename>,
    /// Hidden files, macOS leftovers and firmware images inside folders.
    pub skipped_clutter: u64,
    /// Some source could not be read, so a mirror removes nothing: the files
    /// it could not see would look like extras.
    pub deletions_blocked: bool,
}

impl Plan {
    pub fn total_bytes(&self) -> u64 {
        self.items.iter().map(|i| i.size).sum()
    }

    pub fn summary(&self) -> PlanSummary {
        PlanSummary {
            files: self.items.len() as u64,
            total_bytes: self.total_bytes(),
            existing: self.items.iter().filter(|i| i.exists).count() as u64,
            deletions: self.deletions.clone(),
            issues: self.issues.clone(),
            renames: self.renames.clone(),
            skipped_clutter: self.skipped_clutter,
            deletions_blocked: self.deletions_blocked,
        }
    }
}

/// What the UI shows before anything is copied.
#[derive(Debug, Clone, Serialize)]
pub struct PlanSummary {
    pub files: u64,
    pub total_bytes: u64,
    /// Files whose name already exists on the card.
    pub existing: u64,
    pub deletions: Vec<String>,
    pub issues: Vec<Issue>,
    pub renames: Vec<Rename>,
    pub skipped_clutter: u64,
    pub deletions_blocked: bool,
}

struct Planner<'a> {
    card: &'a Card,
    options: &'a PlanOptions,
    plan: Plan,
    /// Lowercased destinations of every source file, usable or not.
    claimed: HashSet<String>,
    /// Lowercased `folder/` prefixes of linked folders, which are not copied
    /// but must not have their card copies deleted either.
    claimed_dirs: Vec<String>,
}

/// Works out what copying `sources` (files and folders) onto the card involves.
pub fn plan(card: &Card, sources: &[PathBuf], options: &PlanOptions) -> Result<Plan> {
    let mut planner = Planner {
        card,
        options,
        plan: Plan::default(),
        claimed: HashSet::new(),
        claimed_dirs: Vec::new(),
    };
    let base: Vec<String> = options
        .dest_dir
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(|s| planner.safe_segment(s))
        .collect();
    if reserved_destination(&base) {
        return Err(Error::Rejected(format!(
            "The card folder \"{}\" is hidden or belongs to the player.",
            options.dest_dir
        )));
    }

    for source in sources {
        let name = source
            .file_name()
            .ok_or_else(|| Error::Rejected(format!("{} has no name", source.display())))?
            .to_string_lossy();
        let mut dest = base.clone();
        dest.push(planner.safe_segment(&name));
        if reserved_destination(&dest) {
            planner.issue(source, IssueKind::ReservedName);
            continue;
        }
        match fs::metadata(source) {
            Ok(meta) if meta.is_dir() => planner.walk(source, &dest),
            Ok(meta) if meta.is_file() => planner.add_file(source, &dest, meta.len()),
            _ => planner.issue(source, IssueKind::Unreadable),
        }
    }

    if options.delete_extra {
        planner.find_deletions(sources, &base)?;
    }
    Ok(planner.plan)
}

impl Planner<'_> {
    fn issue(&mut self, source: &Path, kind: IssueKind) {
        self.plan.issues.push(Issue {
            source: source.display().to_string(),
            kind,
        });
    }

    fn safe_segment(&mut self, name: &str) -> String {
        let safe = fat_safe_name(name);
        if safe != name {
            self.plan.renames.push(Rename {
                from: name.into(),
                to: safe.clone(),
            });
        }
        safe
    }

    fn walk(&mut self, dir: &Path, dest: &[String]) {
        let Ok(entries) = fs::read_dir(dir) else {
            return self.issue(dir, IssueKind::Unreadable);
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_hidden(&name) || is_firmware_image(&name) {
                self.plan.skipped_clutter += 1;
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                self.issue(&entry.path(), IssueKind::Unreadable);
                continue;
            };
            let mut child = dest.to_vec();
            child.push(self.safe_segment(&name));
            if kind.is_symlink() {
                // Not copied (it could loop or leave the folder), but whatever
                // the card has under that name is not an extra.
                let claim = child.join("/").to_lowercase();
                self.claimed_dirs.push(format!("{claim}/"));
                self.claimed.insert(claim);
            } else if kind.is_dir() {
                self.walk(&entry.path(), &child);
            } else if kind.is_file() {
                match entry.metadata() {
                    Ok(meta) => self.add_file(&entry.path(), &child, meta.len()),
                    Err(_) => self.issue(&entry.path(), IssueKind::Unreadable),
                }
            }
        }
    }

    fn add_file(&mut self, source: &Path, dest: &[String], size: u64) {
        let relative = dest.join("/");
        let first_claim = self.claimed.insert(relative.to_lowercase());
        let problem = if dest.iter().any(|s| name_too_long(s)) {
            Some(IssueKind::NameTooLong)
        } else if !player_path_fits(&relative) {
            Some(IssueKind::PathTooLong)
        } else if self.options.max_file_bytes.is_some_and(|max| size > max) {
            Some(IssueKind::TooLargeForFat)
        } else if !first_claim {
            Some(IssueKind::NameConflict)
        } else {
            None
        };
        if let Some(kind) = problem {
            return self.issue(source, kind);
        }
        let exists = self
            .card
            .resolve(&relative, false)
            .ok()
            .and_then(|p| fs::symlink_metadata(p).ok())
            .is_some_and(|m| m.is_file());
        self.plan.items.push(PlanItem {
            source: source.to_path_buf(),
            dest: relative,
            size,
            exists,
        });
    }

    /// Files under the mirrored folder on the card that no source file claims.
    fn find_deletions(&mut self, sources: &[PathBuf], base: &[String]) -> Result<()> {
        let [source] = sources else {
            return Err(Error::Rejected("mirroring needs exactly one folder".into()));
        };
        if !source.is_dir() {
            return Err(Error::Rejected(
                "mirroring needs a folder, not a file".into(),
            ));
        }
        if self
            .plan
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::Unreadable)
        {
            self.plan.deletions_blocked = true;
            return Ok(());
        }
        let name = source
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let mut root = base.to_vec();
        root.push(fat_safe_name(&name));
        if reserved_destination(&root) {
            return Err(Error::Rejected(format!(
                "\"{name}\" is hidden or belongs to the player and cannot be mirrored."
            )));
        }
        let relative = root.join("/");
        let dir = self.card.resolve(&relative, false)?;
        let mut found = Vec::new();
        list_files(&dir, &relative, &mut found);
        self.plan.deletions = found
            .into_iter()
            .filter(|f| {
                let lower = f.to_lowercase();
                !self.claimed.contains(&lower)
                    && !self.claimed_dirs.iter().any(|d| lower.starts_with(d))
            })
            .collect();
        Ok(())
    }
}

/// Regular files below `dir`, as `prefix/…` paths. Links are not followed and
/// hidden entries and firmware images are never listed, so never deleted.
fn list_files(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    if !fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir()) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if is_hidden(&name) {
            continue;
        }
        let relative = format!("{prefix}/{name}");
        if kind.is_dir() {
            list_files(&entry.path(), &relative, out);
        } else if kind.is_file() && !is_firmware_image(&name) {
            out.push(relative);
        }
    }
    out.sort();
}

#[cfg(test)]
mod tests;
