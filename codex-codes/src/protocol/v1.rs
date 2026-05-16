//! v1 protocol types — the initialization handshake.
//!
//! Mirrors upstream's `codex-rs/app-server-protocol/src/protocol/v1.rs`.

use serde::{Deserialize, Serialize};

/// Client info sent during the `initialize` handshake.
///
/// Identifies the connecting client to the app-server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ClientInfo {
    /// Client application name (e.g., `"my-codex-app"`).
    pub name: String,
    /// Client version string (e.g., `"1.0.0"`).
    pub version: String,
    /// Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Client capabilities negotiated during `initialize`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct InitializeCapabilities {
    /// Opt into receiving experimental API methods and fields.
    #[serde(default)]
    pub experimental_api: bool,
    /// Notification method names to suppress for this connection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opt_out_notification_methods: Option<Vec<String>>,
}

/// Parameters for the `initialize` request.
///
/// Must be the first request sent after connecting to the app-server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct InitializeParams {
    /// Information about the connecting client.
    pub client_info: ClientInfo,
    /// Optional client capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<InitializeCapabilities>,
}

/// Response from the `initialize` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct InitializeResponse {
    /// The server's user-agent string.
    pub user_agent: String,
    /// Absolute path to the server's `$CODEX_HOME` directory.
    pub codex_home: String,
    /// Platform family of the running app-server target (`"unix"` /
    /// `"windows"`).
    pub platform_family: String,
    /// Operating system of the running app-server target (`"linux"`,
    /// `"macos"`, `"windows"`, ...).
    pub platform_os: String,
}
