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
    write_file(src.path(), "Lib/big.flac", &[0u8; 20]);
    write_file(src.path(), "Lib/long.mp3", b"x");
    let deep = "d".repeat(200);
    write_file(
        src.path(),
        &format!("Lib/{deep}/{deep}/{deep}/song.mp3"),
        b"x",
    );
    write_file(src.path(), "Lib/a?.mp3", b"1");
    write_file(src.path(), "Lib/a_.mp3", b"2");
    write_file(src.path(), "Lib/ok.mp3", b"3");

    let options = PlanOptions {
        max_file_bytes: Some(10),
        ..PlanOptions::default()
    };
    let plan = plan(&card, &[src.path().join("Lib")], &options).unwrap();
    let kind_of = |needle: &str| {
        plan.issues
            .iter()
            .find(|i| i.source.contains(needle))
            .map(|i| i.kind)
    };
    let folder = "Lib";
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

#[test]
fn hidden_and_player_owned_destinations_are_refused() {
    let (card_dir, card) = fake_card();
    let src = tempfile::tempdir().unwrap();
    write_file(src.path(), "a.mp3", b"x");
    write_file(card_dir.path(), ".compas/device.json", b"{}");

    for dest_dir in [
        ".compas",
        ".plugins",
        ".PLUGINS",
        "Themes",
        "themes/Night",
        "Music/.hidden",
    ] {
        let options = PlanOptions {
            dest_dir: dest_dir.into(),
            ..PlanOptions::default()
        };
        assert!(
            plan(&card, &[src.path().join("a.mp3")], &options).is_err(),
            "{dest_dir}"
        );
    }
}

#[test]
fn a_mirror_of_a_player_folder_is_refused_and_nothing_is_listed_for_deletion() {
    let (card_dir, card) = fake_card();
    write_file(card_dir.path(), ".compas/device.json", b"{}");
    write_file(card_dir.path(), ".compas/albumart/v4-x.bmp", b"b");
    write_file(card_dir.path(), ".plugins/A.lua", b"l");
    write_file(card_dir.path(), "Themes/Night.theme", b"t");
    let options = PlanOptions {
        delete_extra: true,
        ..PlanOptions::default()
    };

    for name in [".compas", ".plugins", ".PLUGINS", "Themes", "THEMES"] {
        let src = tempfile::tempdir().unwrap();
        write_file(src.path(), &format!("{name}/new.txt"), b"x");
        assert!(
            plan(&card, &[src.path().join(name)], &options).is_err(),
            "{name}"
        );
    }
}

#[test]
fn firmware_images_and_hidden_card_files_are_never_copied_or_listed_for_deletion() {
    let (card_dir, card) = fake_card();
    write_file(card_dir.path(), "Album/old.upt", b"image");
    write_file(card_dir.path(), "Album/.hidden/keep.mp3", b"k");
    write_file(card_dir.path(), "Album/gone.mp3", b"g");
    let src = tempfile::tempdir().unwrap();
    write_file(src.path(), "Album/01.mp3", b"a");
    write_file(src.path(), "Album/new.UPT", b"firmware");
    write_file(src.path(), "r1.upt", b"firmware");
    let options = PlanOptions {
        delete_extra: true,
        ..PlanOptions::default()
    };

    let found = plan(&card, &[src.path().join("Album")], &options).unwrap();
    assert_eq!(dests(&found), ["Album/01.mp3"]);
    assert_eq!(found.deletions, ["Album/gone.mp3"]);

    let single = plan(&card, &[src.path().join("r1.upt")], &PlanOptions::default()).unwrap();
    assert!(single.items.is_empty());
    assert_eq!(single.issues[0].kind, IssueKind::ReservedName);
}

#[cfg(unix)]
#[test]
fn an_unreadable_folder_blocks_every_deletion() {
    use std::os::unix::fs::PermissionsExt;

    let (card_dir, card) = fake_card();
    write_file(card_dir.path(), "Album/extra.mp3", b"e");
    write_file(card_dir.path(), "Album/Disc 2/01.mp3", b"d");
    let src = tempfile::tempdir().unwrap();
    write_file(src.path(), "Album/01.mp3", b"a");
    write_file(src.path(), "Album/Disc 2/01.mp3", b"d");
    let locked = src.path().join("Album/Disc 2");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let options = PlanOptions {
        delete_extra: true,
        ..PlanOptions::default()
    };

    let found = plan(&card, &[src.path().join("Album")], &options);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    let found = found.unwrap();
    if found.issues.is_empty() {
        return; // running as root: permissions do not apply
    }
    assert_eq!(found.issues[0].kind, IssueKind::Unreadable);
    assert!(found.deletions_blocked);
    assert!(found.deletions.is_empty(), "the card's Disc 2 must survive");
    assert!(found.summary().deletions_blocked);
}

#[cfg(unix)]
#[test]
fn linked_source_entries_keep_their_card_copies() {
    let (card_dir, card) = fake_card();
    write_file(card_dir.path(), "Album/link.flac", b"copy of a link target");
    write_file(
        card_dir.path(),
        "Album/linked/x.flac",
        b"inside a linked folder",
    );
    write_file(card_dir.path(), "Album/gone.flac", b"g");
    let src = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write_file(src.path(), "Album/real.flac", b"r");
    write_file(outside.path(), "t.flac", b"t");
    std::os::unix::fs::symlink(
        outside.path().join("t.flac"),
        src.path().join("Album/link.flac"),
    )
    .unwrap();
    std::os::unix::fs::symlink(outside.path(), src.path().join("Album/linked")).unwrap();
    let options = PlanOptions {
        delete_extra: true,
        ..PlanOptions::default()
    };

    let found = plan(&card, &[src.path().join("Album")], &options).unwrap();
    assert_eq!(found.deletions, ["Album/gone.flac"]);
}
