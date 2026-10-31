//! Typed access to the player's plugin and layout endpoints, sharing the settings session client.

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{client::Player, Reply};
use crate::error::{Error, Result};

fn required_option<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub state: String,
    pub incompatible: bool,
    pub needs_confirmation: bool,
    pub player_layout: bool,
    pub theme_pack: bool,
    pub description: String,
    pub category: String,
    pub author: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSnapshot {
    pub state: String,
    pub percent: u8,
    pub current_plugin: String,
    #[serde(deserialize_with = "required_option")]
    pub error: Option<String>,
    pub changed: bool,
    pub refresh_required: bool,
    pub confirmation_required: bool,
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
    pub plugins: Vec<StorePlugin>,
    pub accepted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub loaded: bool,
    #[serde(deserialize_with = "required_option")]
    pub error: Option<String>,
    pub player_layout: bool,
    pub theme_package: bool,
    #[serde(default)]
    pub can_open_settings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSnapshot {
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
    pub truncated: bool,
    pub refresh_pending: bool,
    pub plugins: Vec<InstalledPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerLayout {
    pub id: String,
    pub name: String,
    pub source: String,
    pub has_preview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSnapshot {
    pub current: String,
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
    pub layouts: Vec<PlayerLayout>,
}

/// Accepted mutation. Store operations may include their current snapshot; plugin and layout
/// operations return only their acceptance flags. Optional fields cover both documented shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAction {
    pub accepted: bool,
    pub refresh_requested: Option<bool>,
    pub current: Option<String>,
    pub state: Option<String>,
    pub percent: Option<u8>,
    pub current_plugin: Option<String>,
    pub error: Option<String>,
    pub changed: Option<bool>,
    pub refresh_required: Option<bool>,
    pub confirmation_required: Option<bool>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub total: Option<u32>,
    pub plugins: Option<Vec<StorePlugin>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewResult {
    Image(PreviewImage),
    Pending(PreviewPending),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewImage {
    pub mime: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewPending {
    pub pending: bool,
}

impl PreviewResult {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Pending(PreviewPending { pending: true }) => Ok(()),
            Self::Pending(_) => Err(Error::Rejected(
                "The player's preview answer was not understood.".into(),
            )),
            Self::Image(image)
                if matches!(image.mime.as_str(), "image/png" | "image/jpeg")
                    && valid_preview_base64(&image.data) =>
            {
                Ok(())
            }
            Self::Image(_) => Err(Error::Rejected(
                "The player's preview answer was not understood.".into(),
            )),
        }
    }
}

/// A decoded preview is limited to 512 KiB; this is the corresponding padded Base64 bound.
const PREVIEW_BASE64_LIMIT: usize = 699_052;
const PREVIEW_DECODED_LIMIT: usize = 512 * 1024;

fn valid_preview_base64(data: &str) -> bool {
    let bytes = data.as_bytes();
    if bytes.is_empty() || bytes.len() > PREVIEW_BASE64_LIMIT || !bytes.len().is_multiple_of(4) {
        return false;
    }
    let padding = if bytes.ends_with(b"==") {
        2
    } else if bytes.ends_with(b"=") {
        1
    } else {
        0
    };
    let content_len = bytes.len() - padding;
    if bytes[..content_len]
        .iter()
        .any(|byte| base64_value(*byte).is_none())
        || bytes[content_len..].iter().any(|byte| *byte != b'=')
    {
        return false;
    }
    if padding == 2 && base64_value(bytes[bytes.len() - 3]).is_none_or(|value| value & 0x0f != 0) {
        return false;
    }
    if padding == 1 && base64_value(bytes[bytes.len() - 2]).is_none_or(|value| value & 0x03 != 0) {
        return false;
    }
    bytes.len() / 4 * 3 - padding <= PREVIEW_DECODED_LIMIT
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

impl Player {
    pub async fn plugin_capabilities(&self) -> Result<super::Capabilities> {
        self.capabilities().await
    }

    pub(super) fn page(offset: u64, limit: u64) -> Result<String> {
        if offset > i32::MAX as u64 || !(1..=200).contains(&limit) {
            return Err(Error::Rejected(
                "Page offset must be nonnegative and at most 2147483647; limit must be between 1 and 200.".into(),
            ));
        }
        Ok(format!("?offset={offset}&limit={limit}"))
    }

    pub(super) fn valid_id(id: &str, kind: &str) -> bool {
        match kind {
            // The native plugin manager exposes exact filenames up to NAME_MAX. Unicode and
            // spaces are valid; path separators and ASCII control bytes are not.
            "file" => {
                (5..=255).contains(&id.len())
                    && id.to_ascii_lowercase().ends_with(".lua")
                    && !id
                        .bytes()
                        .any(|byte| byte < 32 || byte == 127 || byte == b'/' || byte == b'\\')
            }
            // Store IDs and registered layout IDs use the API's bounded identifier alphabet.
            "store" => {
                (1..=63).contains(&id.len())
                    && id.bytes().enumerate().all(|(index, byte)| {
                        byte.is_ascii_lowercase()
                            || byte.is_ascii_digit()
                            || (index > 0 && matches!(byte, b'_' | b'.' | b'-'))
                    })
            }
            _ => {
                (1..=63).contains(&id.len())
                    && id.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-')
                    })
            }
        }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<Reply<T>> {
        let (status, body) = self.send(Method::GET, path, None, true).await?;
        Self::reply(status, body)
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: Value,
    ) -> Result<Reply<T>> {
        let bytes = serde_json::to_vec(&body)
            .map_err(|e| Error::Rejected(format!("The request could not be encoded: {e}")))?;
        let (status, body) = self.send(Method::POST, path, Some(bytes), true).await?;
        Self::reply(status, body)
    }

    pub async fn plugin_store(&self, offset: u64, limit: u64) -> Result<Reply<StoreSnapshot>> {
        self.get(&format!("/plugin-store{}", Self::page(offset, limit)?))
            .await
    }

    pub async fn store_plugin_preview(&self, id: &str) -> Result<Reply<PreviewResult>> {
        if !Self::valid_id(id, "store") {
            return Err(Error::Rejected("Invalid plugin catalog ID.".into()));
        }
        let reply: Reply<PreviewResult> =
            self.get(&format!("/plugin-store/preview?id={id}")).await?;
        if let Reply::Ok { data } = &reply {
            data.validate()?;
        }
        Ok(reply)
    }

    pub async fn plugin_store_refresh(&self) -> Result<Reply<PluginAction>> {
        self.post("/plugin-store/refresh", serde_json::json!({}))
            .await
    }

    pub async fn plugin_install(&self, id: &str, force: bool) -> Result<Reply<PluginAction>> {
        if !Self::valid_id(id, "store") {
            return Err(Error::Rejected("Invalid plugin catalog ID.".into()));
        }
        self.post(
            "/plugin-store/install",
            serde_json::json!({ "id": id, "force": force }),
        )
        .await
    }

    pub async fn plugin_update(&self, id: &str, force: bool) -> Result<Reply<PluginAction>> {
        if !Self::valid_id(id, "store") {
            return Err(Error::Rejected("Invalid plugin catalog ID.".into()));
        }
        self.post(
            "/plugin-store/update",
            serde_json::json!({ "id": id, "force": force }),
        )
        .await
    }

    pub async fn plugin_update_all(&self) -> Result<Reply<PluginAction>> {
        self.post("/plugin-store/update-all", serde_json::json!({}))
            .await
    }

    pub async fn plugin_uninstall(&self, id: &str) -> Result<Reply<PluginAction>> {
        if !Self::valid_id(id, "store") {
            return Err(Error::Rejected("Invalid plugin catalog ID.".into()));
        }
        self.post("/plugin-store/uninstall", serde_json::json!({ "id": id }))
            .await
    }

    pub async fn installed_plugins(
        &self,
        offset: u64,
        limit: u64,
    ) -> Result<Reply<InstalledSnapshot>> {
        self.get(&format!("/plugins{}", Self::page(offset, limit)?))
            .await
    }

    pub async fn plugin_set_enabled(&self, id: &str, enabled: bool) -> Result<Reply<PluginAction>> {
        if !Self::valid_id(id, "file") {
            return Err(Error::Rejected("Invalid plugin filename.".into()));
        }
        self.post(
            "/plugins/set-enabled",
            serde_json::json!({ "id": id, "enabled": enabled }),
        )
        .await
    }

    pub async fn plugins_refresh(&self) -> Result<Reply<PluginAction>> {
        self.post("/plugins/refresh", serde_json::json!({})).await
    }

    pub async fn open_plugin_settings(&self, id: &str) -> Result<Reply<PluginAction>> {
        if !Self::valid_id(id, "file") {
            return Err(Error::Rejected("Invalid plugin filename.".into()));
        }
        self.post("/plugins/open-settings", serde_json::json!({ "id": id }))
            .await
    }

    pub async fn player_layouts(&self, offset: u64, limit: u64) -> Result<Reply<LayoutSnapshot>> {
        self.get(&format!("/player-layouts{}", Self::page(offset, limit)?))
            .await
    }

    pub async fn player_layout_preview(&self, id: &str) -> Result<Reply<PreviewResult>> {
        if !Self::valid_id(id, "layout") {
            return Err(Error::Rejected("Invalid player layout ID.".into()));
        }
        let reply: Reply<PreviewResult> = self
            .get(&format!("/player-layouts/preview?id={id}"))
            .await?;
        if let Reply::Ok { data } = &reply {
            data.validate()?;
        }
        Ok(reply)
    }

    pub async fn select_player_layout(&self, id: &str) -> Result<Reply<PluginAction>> {
        if !Self::valid_id(id, "layout") {
            return Err(Error::Rejected("Invalid player layout ID.".into()));
        }
        self.post("/player-layouts/select", serde_json::json!({ "id": id }))
            .await
    }
}
