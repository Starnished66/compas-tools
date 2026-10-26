//! The library step runs only for a player that writes the database layout this
//! tool writes; for any other card the player builds the library itself.

use serde::Serialize;

use super::{LibraryInfo, SdTool};
use crate::card::Card;
use crate::error::{Error, Result};

/// How the card's player relates to the database format this tool writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LibrarySupport {
    Ready,
    /// An older format, or no `device.json` at all (every released player so far).
    PlayerOlder,
    PlayerNewer,
}

/// Compares the card's `device.json` `database_format` with the tool's.
pub fn library_support(card: &Card, info: &LibraryInfo) -> Result<LibrarySupport> {
    let card_format = card.device()?.and_then(|device| device.database_format);
    Ok(match card_format {
        Some(format) if format == info.database_format => LibrarySupport::Ready,
        Some(format) if format > info.database_format => LibrarySupport::PlayerNewer,
        _ => LibrarySupport::PlayerOlder,
    })
}

impl SdTool {
    /// Fails unless the formats match; for commands that build the library on request.
    pub async fn ensure_library_ready(&self, card: &Card) -> Result<()> {
        let info = self.info(card).await?;
        Err(Error::Rejected(match library_support(card, &info)? {
            LibrarySupport::Ready => return Ok(()),
            LibrarySupport::PlayerOlder => {
                "The player builds this card's library itself on its next start. Update the player \
                 first if Compás Tools should build it on this computer."
            }
            LibrarySupport::PlayerNewer => {
                "The player on this card is newer than this version of Compás Tools. Update \
                 Compás Tools to build the library on this computer."
            }
        }
        .into()))
    }
}
