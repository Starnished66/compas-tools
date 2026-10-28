//! The player's remote-settings JSON, as Compás Tools reads and sends it. Field names follow the
//! player's API (camelCase) so the screen sees the same shapes. Unknown fields are ignored, so a
//! newer player that adds some does not break an older Tools.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Capabilities {
    pub features: Vec<String>,
    pub settings_schema_version: Option<u32>,
}

impl Capabilities {
    pub fn has_settings(&self) -> bool {
        self.features.iter().any(|f| f == "settings_v1")
    }
}

/// The two numbers of `GET /status` that say whether a cached copy is still current.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Status {
    pub settings_revision: Option<u64>,
    pub settings_epoch: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Device {
    pub model: Option<String>,
    pub board: Option<String>,
    pub player_version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub label_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EnumOption {
    pub value: i64,
    pub label: String,
    pub label_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingDef {
    pub key: String,
    pub group: String,
    pub label: String,
    pub label_id: String,
    /// `bool`, `enum`, `int`, `percent`, `preset` or `string`.
    #[serde(rename = "type")]
    pub kind: String,
    /// `live`, `next_track`, `next_use` or `next_boot`.
    pub apply: String,
    pub available: bool,
    pub read_only: bool,
    pub reason: Option<String>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub unit: Option<String>,
    pub confirm_above: Option<i64>,
    pub options: Vec<EnumOption>,
    pub values: Vec<i64>,
    pub depends_on: Option<String>,
}

impl Default for SettingDef {
    fn default() -> Self {
        Self {
            key: String::new(),
            group: String::new(),
            label: String::new(),
            label_id: String::new(),
            kind: String::new(),
            apply: String::new(),
            available: true,
            read_only: false,
            reason: None,
            min: None,
            max: None,
            unit: None,
            confirm_above: None,
            options: Vec::new(),
            values: Vec::new(),
            depends_on: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ActionDef {
    pub id: String,
    pub group: String,
    pub label: String,
    pub label_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Schema {
    pub schema_version: u32,
    pub device: Device,
    pub groups: Vec<Group>,
    pub settings: Vec<SettingDef>,
    pub actions: Vec<ActionDef>,
}

/// `GET /settings`: every value, or only those changed since a revision.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Snapshot {
    pub epoch: u64,
    pub revision: u64,
    pub full: bool,
    pub writable: bool,
    pub values: BTreeMap<String, Value>,
}

/// The body of `POST /settings`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ApplyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_revision: Option<u64>,
    pub changes: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub acknowledge: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Pending {
    pub key: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Applied {
    pub epoch: u64,
    pub revision: u64,
    pub applied: Vec<String>,
    pub pending: Vec<Pending>,
    pub also_changed: Vec<Value>,
}

/// A rescan the player accepted (`202`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Accepted {
    pub accepted: bool,
}

/// Why the player said no: its HTTP status and `settings_error` code, with the per-key
/// `errors` of a failed validation and the `current` values of a revision conflict.
#[derive(Debug, Clone, Serialize)]
pub struct Refusal {
    pub status: u16,
    pub code: String,
    pub errors: Value,
    pub current: Value,
}

/// A player's answer: what it sent, or its refusal. A refusal is an answer, not a failure.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Reply<T> {
    Ok { data: T },
    Refused { refusal: Refusal },
}
