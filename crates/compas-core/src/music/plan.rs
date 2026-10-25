use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::clutter::is_clutter_name;
use super::names::{fat_safe_name, name_too_long, player_path_fits};
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
    pub skipped_clutter: u64,
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
}

struct Planner<'a> {
    card: &'a Card,
    options: &'a PlanOptions,
    plan: Plan,
    /// Lowercased destinations of every source file, usable or not.
    claimed: HashSet<String>,
}

/// Works out what copying `sources` (files and folders) onto the card involves.
pub fn plan(card: &Card, sources: &[PathBuf], options: &PlanOptions) -> Result<Plan> {
    let mut planner = Planner {
        card,
        options,
        plan: Plan::default(),
        claimed: HashSet::new(),
    };
    let base: Vec<String> = options
        .dest_dir
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(|s| planner.safe_segment(s))
        .collect();

    for source in sources {
        let name = source
            .file_name()
            .ok_or_else(|| Error::Rejected(format!("{} has no name", source.display())))?
            .to_string_lossy();
        let mut dest = base.clone();
        dest.push(planner.safe_segment(&name));
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
            if is_clutter_name(&name) {
                self.plan.skipped_clutter += 1;
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                self.issue(&entry.path(), IssueKind::Unreadable);
                continue;
            };
            let mut child = dest.to_vec();
            child.push(self.safe_segment(&name));
            if kind.is_dir() {
                self.walk(&entry.path(), &child);
            } else if kind.is_file() {
                match entry.metadata() {
                    Ok(meta) => self.add_file(&entry.path(), &child, meta.len()),
                    Err(_) => self.issue(&entry.path(), IssueKind::Unreadable),
                }
            }
            // Links inside a folder are left out: they could loop or leave it.
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
        let name = source
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let mut root = base.to_vec();
        root.push(fat_safe_name(&name));
        let relative = root.join("/");
        let dir = self.card.resolve(&relative, false)?;
        let mut found = Vec::new();
        list_files(&dir, &relative, &mut found);
        self.plan.deletions = found
            .into_iter()
            .filter(|f| !self.claimed.contains(&f.to_lowercase()))
            .collect();
        Ok(())
    }
}

/// Regular files below `dir`, as `prefix/…` paths; links are not followed.
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
        let relative = format!("{prefix}/{name}");
        if kind.is_dir() {
            list_files(&entry.path(), &relative, out);
        } else if kind.is_file() && !is_clutter_name(&name) {
            out.push(relative);
        }
    }
    out.sort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fake_card, write_file};

    fn dests(plan: &Plan) -> Vec<&str> {
        plan.items.iter().map(|i| i.dest.as_str()).collect()
    }

    #[test]
    fn folders_and_files_land_under_the_chosen_folder() {
        let (_card_dir, card) = fake_card();
        let src = tempfile::tempdir().unwrap();
        write_file(src.path(), "Album/01.flac", b"a");
        write_file(src.path(), "Album/cover.jpg", b"b");
        write_file(src.path(), "single.mp3", b"c");

        let sources = [src.path().join("Album"), src.path().join("single.mp3")];
        let options = PlanOptions {
            dest_dir: "Music/Rock".into(),
            ..PlanOptions::default()
        };
        let plan = plan(&card, &sources, &options).unwrap();
        assert_eq!(
            dests(&plan),
            [
                "Music/Rock/Album/01.flac",
                "Music/Rock/Album/cover.jpg",
                "Music/Rock/single.mp3"
            ]
        );
        assert_eq!(plan.total_bytes(), 3);
        assert!(plan.issues.is_empty());
    }

    #[test]
    fn clutter_links_and_bad_names_are_handled() {
        let (_card_dir, card) = fake_card();
        let src = tempfile::tempdir().unwrap();
        write_file(src.path(), "Disc/._01.flac", b"apple");
        write_file(src.path(), "Disc/.DS_Store", b"");
        write_file(src.path(), "Disc/AC:DC?.flac", b"x");
        write_file(src.path(), "Disc/Why.", b"x");
        #[cfg(unix)]
        std::os::unix::fs::symlink(src.path(), src.path().join("Disc/loop")).unwrap();

        let plan = plan(&card, &[src.path().join("Disc")], &PlanOptions::default()).unwrap();
        assert_eq!(dests(&plan), ["Disc/AC_DC_.flac", "Disc/Why"]);
        assert_eq!(plan.skipped_clutter, 2);
        assert_eq!(plan.renames.len(), 2);
        assert_eq!(plan.renames[0].from, "AC:DC?.flac");
    }

    #[test]
    fn size_name_length_path_length_and_conflicts_keep_files_out() {
        let (_card_dir, card) = fake_card();
        let src = tempfile::tempdir().unwrap();
        write_file(src.path(), "big.flac", &[0u8; 20]);
        write_file(src.path(), "long.mp3", b"x");
        let deep = "d".repeat(200);
        write_file(src.path(), &format!("{deep}/{deep}/{deep}/song.mp3"), b"x");
        write_file(src.path(), "a?.mp3", b"1");
        write_file(src.path(), "a_.mp3", b"2");
        write_file(src.path(), "ok.mp3", b"3");

        let options = PlanOptions {
            max_file_bytes: Some(10),
            ..PlanOptions::default()
        };
        let plan = plan(&card, &[src.path().to_path_buf()], &options).unwrap();
        let kind_of = |needle: &str| {
            plan.issues
                .iter()
                .find(|i| i.source.contains(needle))
                .map(|i| i.kind)
        };
        let folder = src
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(kind_of("big.flac"), Some(IssueKind::TooLargeForFat));
        assert_eq!(kind_of("song.mp3"), Some(IssueKind::PathTooLong));
        assert_eq!(kind_of("a_.mp3"), Some(IssueKind::NameConflict));
        assert!(dests(&plan).contains(&format!("{folder}/ok.mp3").as_str()));
        assert_eq!(
            plan.items.len(),
            3,
            "ok.mp3 and the first of the conflicting pair"
        );
    }

    #[test]
    fn existing_files_are_flagged() {
        let (card_dir, card) = fake_card();
        write_file(card_dir.path(), "Album/01.flac", b"old");
        let src = tempfile::tempdir().unwrap();
        write_file(src.path(), "Album/01.flac", b"new");
        write_file(src.path(), "Album/02.flac", b"new");
        let plan = plan(&card, &[src.path().join("Album")], &PlanOptions::default()).unwrap();
        let exists: Vec<_> = plan.items.iter().map(|i| i.exists).collect();
        assert_eq!(exists, [true, false]);
    }

    #[test]
    fn mirror_lists_card_files_the_folder_no_longer_has() {
        let (card_dir, card) = fake_card();
        write_file(card_dir.path(), "Album/01.flac", b"x");
        write_file(card_dir.path(), "Album/gone.flac", b"x");
        write_file(card_dir.path(), "Album/Disc 2/old.flac", b"x");
        write_file(card_dir.path(), "Other/keep.flac", b"x");
        let src = tempfile::tempdir().unwrap();
        write_file(src.path(), "Album/01.flac", b"x");
        let options = PlanOptions {
            delete_extra: true,
            ..PlanOptions::default()
        };

        let plan = plan(&card, &[src.path().join("Album")], &options).unwrap();
        assert_eq!(plan.deletions, ["Album/Disc 2/old.flac", "Album/gone.flac"]);

        let two = [src.path().join("Album"), src.path().join("Album")];
        assert!(super::plan(&card, &two, &options).is_err());
        assert!(super::plan(&card, &[src.path().join("Album/01.flac")], &options).is_err());
    }

    #[test]
    fn the_destination_folder_cannot_climb_out() {
        let (_card_dir, card) = fake_card();
        let src = tempfile::tempdir().unwrap();
        write_file(src.path(), "a.mp3", b"x");
        let options = PlanOptions {
            dest_dir: "../../etc/..\\x".into(),
            ..PlanOptions::default()
        };
        let plan = plan(&card, &[src.path().join("a.mp3")], &options).unwrap();
        assert_eq!(dests(&plan), ["_/_/etc/_/x/a.mp3"]);
    }
}
