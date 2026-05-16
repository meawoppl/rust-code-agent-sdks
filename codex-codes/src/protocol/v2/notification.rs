//! Cross-cutting notifications (errors, warnings, deprecations).
//!
//! Mirrors upstream's `codex-rs/app-server-protocol/src/protocol/v2/notification.rs`.

use serde::{Deserialize, Serialize};

/// `error` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ErrorNotification {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub will_retry: bool,
}
