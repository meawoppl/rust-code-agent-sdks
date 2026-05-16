//! MCP server lifecycle notifications.
//!
//! Mirrors upstream's `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs`.

use serde::{Deserialize, Serialize};

/// `mcpServer/startupStatus/updated` notification.
///
/// Emitted by the app-server as each managed MCP server transitions through
/// its startup lifecycle (e.g. `starting` → `ready` or `starting` → `failed`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct McpServerStatusUpdatedNotification {
    /// MCP server identifier.
    pub name: String,
    /// Current lifecycle status string (e.g. `"starting"`, `"ready"`,
    /// `"failed"`). Kept as `String` so new status values don't break parsing.
    pub status: String,
    /// Error message if startup failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
