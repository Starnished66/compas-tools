use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;

use super::*;
use crate::progress::Phase;
use crate::test_support::fake_card;

/// A stand-in `compas-sdtool`: a shell script that prints `output` and exits with `status`.
fn fake_tool(dir: &std::path::Path, output: &str, status: i32) -> SdTool {
    let path = dir.join("fake-sdtool");
    let script = format!("#!/bin/sh\necho \"$@\" >&2\ncat <<'EOF'\n{output}\nEOF\nexit {status}\n");
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    SdTool::new(path)
}

const SCAN: &str = r#"{"event":"progress","phase":"walk","done":1,"total":4}
not json at all
{"event":"progress","phase":"tags","done":2,"total":4}
{"event":"progress","phase":"mystery","done":2,"total":4}
{"event":"done","command":"scan","files":4,"parsed":3,"unchanged":0,"failed":1,"skipped":0,"songs":3,"generation":2,"written":true,"mtime_offset":3600}"#;

#[tokio::test]
async fn scan_streams_progress_and_returns_the_counts() {
    let (dir, card) = fake_card();
    let tool_dir = tempfile::tempdir().unwrap();
    let tool = fake_tool(tool_dir.path(), SCAN, 0);
    let seen = Mutex::new(Vec::new());
    let done = tool
        .scan(&card, ScanOptions::default(), &|p| {
            seen.lock().unwrap().push(p)
        })
        .await
        .unwrap();
    assert_eq!(
        (done.parsed, done.failed, done.songs, done.generation),
        (3, 1, 3, 2)
    );
    assert!(done.written);
    assert_eq!(done.mtime_offset, 3600);
    let seen = seen.into_inner().unwrap();
    assert_eq!(
        seen.iter().map(|p| p.phase).collect::<Vec<_>>(),
        [Phase::Scanning, Phase::ReadingTags]
    );
    drop(dir);
}

#[tokio::test]
async fn passes_the_right_arguments() {
    let (_dir, card) = fake_card();
    let tool_dir = tempfile::tempdir().unwrap();
    let echo = r#"{"event":"done","command":"art","board":"r1","albums":1,"written":1,"unchanged":0,"no_art":0,"mtime_offset":0}"#;
    // The fake prints its arguments to stderr, which only shows up in crash messages,
    // so make it fail after printing a valid object and read the message.
    let tool = fake_tool(tool_dir.path(), echo, 3);
    let error = tool
        .art(&card, Some(Model::R3II2025), Some(-7200), &|_| {})
        .await
        .unwrap_err();
    let text = error.to_string();
    assert!(text.contains("art "), "{text}");
    assert!(
        text.contains("--board r3ii_2025 --mtime-offset -7200"),
        "{text}"
    );
}

#[tokio::test]
async fn info_reads_the_single_object() {
    let (_dir, card) = fake_card();
    let tool_dir = tempfile::tempdir().unwrap();
    let tool = fake_tool(
        tool_dir.path(),
        r#"{"sdtool_version":"1.0.0","database_format":5,"generation":12,"songs":4032}"#,
        0,
    );
    let info = tool.info(&card).await.unwrap();
    assert_eq!(
        (info.database_format, info.generation, info.songs),
        (5, 12, 4032)
    );
}

#[tokio::test]
async fn tool_errors_become_friendly_messages() {
    let (_dir, card) = fake_card();
    let tool_dir = tempfile::tempdir().unwrap();
    let tool = fake_tool(
        tool_dir.path(),
        r#"{"event":"error","code":"no_music_found","message":"nothing"}"#,
        1,
    );
    let error = tool
        .scan(
            &card,
            ScanOptions {
                full: true,
                mtime_offset: None,
            },
            &|_| {},
        )
        .await
        .unwrap_err();
    assert!(
        matches!(&error, Error::Sdtool { code, .. } if code == "no_music_found"),
        "{error:?}"
    );
    assert_eq!(error.to_string(), "No music was found on the card.");
}

#[tokio::test]
async fn a_missing_tool_or_a_crash_is_reported() {
    let (_dir, card) = fake_card();
    let error = SdTool::new("/nonexistent/compas-sdtool")
        .info(&card)
        .await
        .unwrap_err();
    assert!(matches!(&error, Error::Sdtool { code, .. } if code == "missing_tool"));

    let tool_dir = tempfile::tempdir().unwrap();
    let tool = fake_tool(tool_dir.path(), "", 139);
    let error = tool.info(&card).await.unwrap_err();
    assert!(
        matches!(&error, Error::Sdtool { code, .. } if code == "crashed"),
        "{error}"
    );
}

#[test]
fn every_documented_error_code_has_its_own_text() {
    for code in [
        "usage",
        "card_not_found",
        "database_open_failed",
        "walk_failed",
        "no_music_found",
        "worker_failed",
        "write_failed",
        "bad_board",
        "art_failed",
        "out_of_memory",
    ] {
        let text = friendly_message(code, "raw tool text");
        assert_ne!(text, "raw tool text", "{code}");
    }
    assert_eq!(
        friendly_message("something_new", "raw tool text"),
        "raw tool text"
    );
}
