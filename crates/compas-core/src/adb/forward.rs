//! `adb forward`: a local TCP port that reaches the player's remote-control server.

use super::Adb;
use crate::error::{Error, Result};

/// Where the player's remote-control server listens.
pub const PLAYER_PORT: u16 = 8899;

impl Adb {
    /// Forwards a free local port to the player's remote-control port and returns it.
    /// adb prints the port it picked when asked for `tcp:0`.
    pub async fn forward(&self, serial: &str) -> Result<u16> {
        let remote = format!("tcp:{PLAYER_PORT}");
        let out = self
            .run(&["-s", serial, "forward", "tcp:0", &remote], super::SHORT)
            .await?;
        out.trim()
            .lines()
            .next()
            .and_then(|line| line.trim().parse().ok())
            .ok_or_else(|| Error::Adb(format!("adb forward gave no port: {}", out.trim())))
    }

    /// Takes a forward made by [`Adb::forward`] away again.
    pub async fn remove_forward(&self, serial: &str, port: u16) -> Result<()> {
        let local = format!("tcp:{port}");
        self.run(&["-s", serial, "forward", "--remove", &local], super::SHORT)
            .await
            .map(drop)
    }
}
