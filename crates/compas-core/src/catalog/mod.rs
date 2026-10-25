//! The plugin store: catalog, per-plugin state, install, update, uninstall
//! and enable/disable. The on-card behaviour follows `docs/formats/plugin-store.md`.

mod classify;
mod commit;
mod dest;
mod disabled;
mod entry;
mod index;
mod install;
mod preview;
mod record;
mod state;
mod uninstall;
mod update_all;
mod validate;

use crate::error::{Error, Result};
use crate::net;

pub use classify::{classify, Classification};
pub use disabled::set_enabled;
pub use entry::{list, Entry};
pub use index::{Plugin, PluginFile, Preview};
pub use install::{install, update, Outcome};
pub use preview::ensure_preview;
pub use state::PluginState;
pub use uninstall::uninstall;
pub use update_all::{update_all, UpdateAllReport};

pub const INDEX_URL: &str =
    "https://github.com/Starnished66/compas-plugins/releases/latest/download/index.json";
pub const RELEASES_BASE: &str = "https://github.com/Starnished66/compas-plugins/releases/download";
const INDEX_LIMIT: usize = 256 << 10;

/// A validated catalog index, plus where its assets are served from.
#[derive(Debug, Clone)]
pub struct Catalog {
    pub tag: String,
    pub plugins: Vec<Plugin>,
    asset_base: String,
}

impl Catalog {
    /// `releases_base` is the URL prefix before `<tag>/<asset>`.
    pub fn parse(bytes: &[u8], releases_base: &str) -> Result<Self> {
        let index = index::parse(bytes)?;
        Ok(Self {
            asset_base: format!("{releases_base}/{}", index.tag),
            tag: index.tag,
            plugins: index.plugins,
        })
    }

    pub async fn fetch() -> Result<Self> {
        Self::fetch_from(INDEX_URL, RELEASES_BASE).await
    }

    pub async fn fetch_from(index_url: &str, releases_base: &str) -> Result<Self> {
        Self::parse(
            &net::get_bytes(index_url, INDEX_LIMIT).await?,
            releases_base,
        )
    }

    pub fn get(&self, id: &str) -> Result<&Plugin> {
        self.plugins
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| Error::NotFound(format!("plugin {id}")))
    }

    pub(crate) fn asset_url(&self, asset: &str) -> String {
        format!("{}/{asset}", self.asset_base)
    }
}

#[cfg(test)]
mod test_fixtures;
