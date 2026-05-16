//! Turn lifecycle requests and notifications.
//!
//! Mirrors upstream's `codex-rs/app-server-protocol/src/protocol/v2/turn.rs`.

use super::thread_data::Turn;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// User input sent as part of a [`TurnStartParams`].
///
/// # Example
///
/// ```
/// use codex_codes::UserInput;
///
/// let text = UserInput::Text { text: "What is 2+2?".into() };
/// let json = serde_json::to_string(&text).unwrap();
/// assert!(json.contains(r#""type":"text""#));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub enum UserInput {
    /// Text input from the user.
    Text { text: String },
    /// Pre-encoded image as a data URI (e.g., `data:image/png;base64,...`).
    Image { data: String },
}

/// Parameters for `turn/start`.
///
/// Starts a new agent turn within an existing thread. The agent processes the
/// input and streams notifications until the turn completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TurnStartParams {
    /// The thread ID from [`crate::ThreadStartResponse`].
    pub thread_id: String,
    /// One or more user inputs (text and/or images).
    pub input: Vec<UserInput>,
    /// Override the model for this turn and subsequent turns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Override reasoning effort for this turn and subsequent turns
    /// (e.g., `"low"`, `"medium"`, `"high"`). Upstream names this `effort`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// Override the sandbox policy for this turn and subsequent turns.
    /// Shape varies; we preserve it as raw JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_policy: Option<Value>,
}

/// Response from `turn/start`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TurnStartResponse {
    /// The newly-started turn (id, items, status).
    pub turn: Turn,
}

/// Parameters for `turn/interrupt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TurnInterruptParams {
    pub thread_id: String,
}

/// Response from `turn/interrupt`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TurnInterruptResponse {}

/// `turn/started` notification.
///
/// Carries the freshly-created [`Turn`] (with `status: in_progress`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TurnStartedNotification {
    pub thread_id: String,
    pub turn: Turn,
}

/// `turn/completed` notification.
///
/// Carries the final [`Turn`] state with its full item list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TurnCompletedNotification {
    pub thread_id: String,
    pub turn: Turn,
}
