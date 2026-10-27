//! What happens after music was copied: the library is brought up to date in
//! the same operation, and a failure there never undoes the copy. Only a card
//! whose player writes the tool's database format gets that update.

use serde::Serialize;

use crate::card::Card;
use crate::music::CopyReport;
use crate::progress::OnProgress;
use crate::sdtool::{library_support, LibrarySupport, LibraryUpdate, SdTool};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LibraryOutcome {
    /// Nothing changed on the card, so there was nothing to update.
    Skipped,
    /// The player on the card does not write this tool's database format, so
    /// the player builds the library on its next start.
    Unsupported {
        support: LibrarySupport,
    },
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
    let update = async {
        let info = tool.info(card).await?;
        match library_support(card, &info)? {
            LibrarySupport::Ready => tool.update_library(card, on_progress).await.map(Ok),
            support => Ok(Err(support)),
        }
    };
    match update.await {
        Ok(Ok(update)) => LibraryOutcome::Updated(update),
        Ok(Err(support)) => LibraryOutcome::Unsupported { support },
        Err(error) => LibraryOutcome::Failed {
            message: error.to_string(),
        },
    }
}

#[cfg(all(test, unix))]
mod tests;
