//! Remote-control status notifications.
//!
//! Mirrors upstream's `codex-rs/app-server-protocol/src/protocol/v2/remote_control.rs`.

use serde::{Deserialize, Serialize};

/// `remoteControl/status/changed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct RemoteControlStatusChangedNotification {
    /// Status string (e.g. `"disabled"`, `"enabled"`).
    pub status: String,
    /// Connected environment id, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<String>,
}
