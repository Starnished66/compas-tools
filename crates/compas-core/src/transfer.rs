//! What happens after music was copied: the library is brought up to date in
//! the same operation, and a failure there never undoes the copy.

use serde::Serialize;

use crate::card::Card;
use crate::music::CopyReport;
use crate::progress::OnProgress;
use crate::sdtool::{LibraryUpdate, SdTool};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LibraryOutcome {
    /// Nothing changed on the card, so there was nothing to update.
    Skipped,
    Updated(LibraryUpdate),
    /// The copied files stay; only the library update has to be repeated.
    Failed {
        message: String,
    },
}

/// Runs the library update when the copy added, replaced or removed a file,
/// even if other files failed to copy.
pub async fn update_after_copy(
    tool: &SdTool,
    card: &Card,
    copy: &CopyReport,
    on_progress: OnProgress<'_>,
) -> LibraryOutcome {
    if copy.copied + copy.deleted == 0 {
        return LibraryOutcome::Skipped;
    }
    match tool.update_library(card, on_progress).await {
        Ok(update) => LibraryOutcome::Updated(update),
        Err(error) => LibraryOutcome::Failed {
            message: error.to_string(),
        },
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::music::{self, PlanOptions};
    use crate::progress::Phase;
    use crate::test_support::{calls, fake_card, script_tool, write_file};

    const SCAN_STEP: &str = r#"echo '{"event":"progress","phase":"tags","done":1,"total":2}'
echo '{"event":"done","command":"scan","files":2,"parsed":1,"unchanged":0,"failed":1,"skipped":0,"songs":1,"generation":3,"written":true,"mtime_offset":3600}'"#;
    const ART_STEP: &str = r#"echo '{"event":"progress","phase":"art","done":1,"total":1}'
echo '{"event":"done","command":"art","board":"r1","albums":1,"written":1,"unchanged":0,"no_art":0,"failed":0,"mtime_offset":3600}'"#;

    /// A fake tool whose `scan` and `art` commands run the given shell snippets.
    fn script(scan: &str, art: &str) -> String {
        format!("case \"$1\" in\nscan) {scan} ;;\nart) {art} ;;\nesac")
    }

    fn working() -> String {
        script(SCAN_STEP, ART_STEP)
    }

    fn copied(count: u64, deleted: u64) -> CopyReport {
        CopyReport {
            copied: count,
            deleted,
            ..CopyReport::default()
        }
    }

    #[tokio::test]
    async fn a_copy_that_changed_files_runs_scan_then_art_as_one_operation() {
        let (_dir, card) = fake_card();
        let tools = tempfile::tempdir().unwrap();
        let tool = script_tool(tools.path(), &working());
        let seen = Mutex::new(Vec::new());

        let outcome = update_after_copy(&tool, &card, &copied(2, 0), &|p| {
            seen.lock().unwrap().push(p.phase)
        })
        .await;

        let LibraryOutcome::Updated(update) = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(calls(tools.path()), ["scan", "art"]);
        assert_eq!((update.scan.failed, update.scan.mtime_offset), (1, 3600));
        assert_eq!(update.art.unwrap().written, 1);
        assert_eq!(
            *seen.lock().unwrap(),
            [Phase::ReadingTags, Phase::BuildingArt]
        );
    }

    #[tokio::test]
    async fn removals_alone_also_update_the_library() {
        let (_dir, card) = fake_card();
        let tools = tempfile::tempdir().unwrap();
        let tool = script_tool(tools.path(), &working());
        let outcome = update_after_copy(&tool, &card, &copied(0, 3), &|_| {}).await;
        assert!(matches!(outcome, LibraryOutcome::Updated(_)));
    }

    #[tokio::test]
    async fn nothing_changed_means_the_tool_is_not_even_started() {
        let (_dir, card) = fake_card();
        let tools = tempfile::tempdir().unwrap();
        let tool = script_tool(tools.path(), &working());
        let identical_only = CopyReport {
            skipped_identical: 5,
            ..CopyReport::default()
        };

        let outcome = update_after_copy(&tool, &card, &identical_only, &|_| {}).await;
        assert!(matches!(outcome, LibraryOutcome::Skipped));
        assert!(calls(tools.path()).is_empty());
    }

    #[tokio::test]
    async fn a_partly_failed_copy_still_updates_the_library() {
        let (_dir, card) = fake_card();
        let tools = tempfile::tempdir().unwrap();
        let tool = script_tool(tools.path(), &working());
        let mut report = copied(1, 0);
        report.failed.push(music::Failure {
            path: "x.flac".into(),
            message: "disk".into(),
        });

        let outcome = update_after_copy(&tool, &card, &report, &|_| {}).await;
        assert!(matches!(outcome, LibraryOutcome::Updated(_)));
    }

    #[tokio::test]
    async fn a_failing_tool_keeps_the_copied_files_and_a_retry_runs_only_the_update() {
        let (card_dir, card) = fake_card();
        let src = tempfile::tempdir().unwrap();
        write_file(src.path(), "Album/01.flac", b"music");
        let plan =
            music::plan(&card, &[src.path().join("Album")], &PlanOptions::default()).unwrap();
        let report = music::copy_plan(&card, &plan, &|_| {}).unwrap();
        assert_eq!(report.copied, 1);

        let tools = tempfile::tempdir().unwrap();
        let broken = script_tool(
            tools.path(),
            r#"echo '{"event":"error","code":"worker_failed","message":"x"}'; exit 1"#,
        );
        let outcome = update_after_copy(&broken, &card, &report, &|_| {}).await;
        let LibraryOutcome::Failed { message } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(
            message,
            "The tag reader could not run, so the library was left unchanged."
        );
        assert!(
            card_dir.path().join("Album/01.flac").exists(),
            "the copy is not undone"
        );

        // Try again: only the library update runs, and the copy is not repeated.
        let fixed = script_tool(tools.path(), &working());
        let update = fixed.update_library(&card, &|_| {}).await.unwrap();
        assert_eq!(update.scan.songs, 1);
        assert_eq!(calls(tools.path()), ["scan", "scan", "art"]);
    }

    #[tokio::test]
    async fn a_partial_art_failure_still_counts_as_an_updated_library() {
        let (_dir, card) = fake_card();
        let tools = tempfile::tempdir().unwrap();
        // The order sdtool 1.1 prints: `done` with a `failed` count, then `art_failed`.
        let art = r#"echo '{"event":"done","command":"art","board":"r1","albums":10,"written":8,"unchanged":1,"no_art":0,"failed":1,"mtime_offset":0}'
echo '{"event":"error","code":"art_failed","message":"one cover"}'; exit 1"#;
        let tool = script_tool(tools.path(), &script(SCAN_STEP, art));

        let outcome = update_after_copy(&tool, &card, &copied(1, 0), &|_| {}).await;
        let LibraryOutcome::Updated(update) = outcome else {
            panic!("{outcome:?}")
        };
        let art = update.art.unwrap();
        assert_eq!((art.written, art.failed), (8, 1));
        assert!(art.warning.is_some());
        assert!(update.art_error.is_none());
    }

    #[tokio::test]
    async fn covers_failing_completely_do_not_make_the_library_update_fail() {
        let (_dir, card) = fake_card();
        let tools = tempfile::tempdir().unwrap();
        let art = r#"echo '{"event":"error","code":"out_of_memory","message":"x"}'; exit 1"#;
        let tool = script_tool(tools.path(), &script(SCAN_STEP, art));

        let outcome = update_after_copy(&tool, &card, &copied(1, 0), &|_| {}).await;
        let LibraryOutcome::Updated(update) = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(update.scan.songs, 1);
        assert!(update.art.is_none());
        assert_eq!(
            update.art_error.as_deref(),
            Some("The computer ran out of memory.")
        );
    }

    #[tokio::test]
    async fn a_failed_scan_is_the_only_failure_that_offers_a_retry() {
        let (_dir, card) = fake_card();
        let tools = tempfile::tempdir().unwrap();
        let scan = r#"echo '{"event":"error","code":"write_failed","message":"x"}'; exit 1"#;
        let tool = script_tool(tools.path(), &script(scan, ART_STEP));
        let outcome = update_after_copy(&tool, &card, &copied(1, 0), &|_| {}).await;
        assert!(matches!(outcome, LibraryOutcome::Failed { .. }));
        assert_eq!(
            calls(tools.path()),
            ["scan"],
            "no art step after a failed scan"
        );
    }

    #[tokio::test]
    async fn a_missing_tool_is_a_failure_not_a_panic() {
        let (_dir, card) = fake_card();
        let tool = SdTool::new("/nonexistent/compas-sdtool");
        let outcome = update_after_copy(&tool, &card, &copied(1, 0), &|_| {}).await;
        assert!(matches!(outcome, LibraryOutcome::Failed { .. }));
    }
}
