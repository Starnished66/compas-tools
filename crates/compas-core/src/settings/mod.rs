//! Changing a player's settings from the computer over ADB. The same HTTP API the phone app uses,
//! reached through `adb forward` (see [`crate::adb::Adb::forward`]); `settings.txt` is never touched.
//! Every write needs the Remote Control PIN, which is kept in memory only.

mod client;
mod model;

#[cfg(test)]
mod tests;

use serde::Serialize;

pub use client::{Pin, Player};
pub use model::{
    Accepted, ActionDef, Applied, ApplyRequest, Capabilities, Device, EnumOption, Group, Pending,
    Refusal, Reply, Schema, SettingDef, Snapshot, Status,
};

use crate::adb::Adb;
use crate::error::{Error, Result};

/// What opening the player's settings found.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Connection {
    /// Nothing answered behind the forward: Remote Control is off on the player.
    Unreachable,
    /// The player answered but does not offer remote settings.
    Unsupported,
    /// The PIN is missing or wrong.
    PinRequired { wrong: bool },
    /// The player's client workers are busy (the phone app may be polling): try again shortly.
    Busy,
    /// Too many wrong PINs; the player asks to wait.
    RateLimited,
    Ready {
        schema: Box<Schema>,
        snapshot: Box<Snapshot>,
        status: Status,
    },
}

/// Opens the settings: checks the player offers them and the PIN is right, then reads the schema
/// and every value. Build-agnostic: it asks the player what it supports and never its version.
pub async fn connect(player: &Player, has_pin: bool) -> Result<Connection> {
    let caps = match player.capabilities().await {
        Ok(caps) => caps,
        // Through `adb forward` a closed port shows as a dropped connection.
        Err(Error::Network(_)) => return Ok(Connection::Unreachable),
        Err(Error::HttpStatus { status: 503, .. }) => return Ok(Connection::Busy),
        Err(e) => return Err(e),
    };
    if !caps.has_settings() {
        return Ok(Connection::Unsupported);
    }
    if !has_pin {
        return Ok(Connection::PinRequired { wrong: false });
    }
    let status = match player.status().await? {
        Reply::Ok { data } => data,
        Reply::Refused { refusal } if refusal.status == 401 => {
            return Ok(Connection::PinRequired { wrong: true });
        }
        Reply::Refused { refusal } if refusal.status == 429 => return Ok(Connection::RateLimited),
        Reply::Refused { refusal } if refusal.status == 503 => return Ok(Connection::Busy),
        Reply::Refused { refusal } => return Err(refused(&refusal)),
    };
    let schema = match player.schema().await? {
        Reply::Ok { data } => data,
        Reply::Refused { refusal } => return Err(refused(&refusal)),
    };
    let snapshot = match player.settings(None).await? {
        Reply::Ok { data } => data,
        Reply::Refused { refusal } => return Err(refused(&refusal)),
    };
    Ok(Connection::Ready {
        schema: Box::new(schema),
        snapshot: Box::new(snapshot),
        status,
    })
}

fn refused(refusal: &Refusal) -> Error {
    Error::Rejected(format!(
        "The player refused the request ({} {}).",
        refusal.status, refusal.code
    ))
}

/// A forward to the player and what opening its settings through it found.
pub struct Opened {
    pub port: u16,
    pub player: std::sync::Arc<Player>,
    pub connection: Connection,
}

/// Forwards the player's port and opens its settings. When opening fails after the forward was
/// made, the forward is removed again and the error returned: nothing is left behind.
pub async fn open(adb: &Adb, serial: &str, pin: Pin) -> Result<Opened> {
    let port = adb.forward(serial).await?;
    let opened = async {
        let player = std::sync::Arc::new(Player::new(port, pin.clone())?);
        let connection = connect(&player, !pin.is_empty()).await?;
        Ok(Opened {
            port,
            player,
            connection,
        })
    }
    .await;
    if opened.is_err() {
        let _ = adb.remove_forward(serial, port).await;
    }
    opened
}
