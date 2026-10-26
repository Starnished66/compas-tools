//! The library step runs only for a player that writes the database layout this
//! tool writes; for any other card the player builds the library itself.

use super::{LibraryInfo, SdTool};
use crate::card::Card;
use crate::error::{Error, Result};

/// Whether the card's `device.json` reports the same database format as the tool.
/// A card without `device.json` (every released player so far) is not ready.
pub fn library_ready(card: &Card, info: &LibraryInfo) -> Result<bool> {
    let card_format = card.device()?.and_then(|device| device.database_format);
    Ok(card_format == Some(info.database_format))
}

impl SdTool {
    /// Fails unless `library_ready`; for commands that build the library on request.
    pub async fn ensure_library_ready(&self, card: &Card) -> Result<()> {
        let info = self.info(card).await?;
        if library_ready(card, &info)? {
            return Ok(());
        }
        Err(Error::Rejected(
            "The player builds this card's library itself on its next start. Update the player \
             first if Compás Tools should build it on this computer."
                .into(),
        ))
    }
}
