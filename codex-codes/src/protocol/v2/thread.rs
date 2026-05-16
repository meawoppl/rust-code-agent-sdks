//! Thread lifecycle requests, status, and thread-level notifications.
//!
//! Mirrors upstream's `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for `thread/start`.
///
/// Use `ThreadStartParams::default()` for a basic thread with all defaults.
/// Upstream's struct has many more fields (model overrides, sandbox/approval
/// policy, etc.) — none are modeled yet; add them in this file as needed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ThreadStartParams {}

/// Thread metadata returned inside a [`ThreadStartResponse`].
///
/// Local convenience wrapper — upstream exposes the equivalent via the
/// `Thread` struct (in `thread_data.rs`); we capture only the `id` field
/// strictly and route the rest through `extra`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadInfo {
    /// Unique thread identifier.
    pub id: String,
    /// All other fields are captured but not typed.
    #[serde(flatten)]
    pub extra: Value,
}

/// Response from `thread/start`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartResponse {
    /// The created thread.
    pub thread: ThreadInfo,
    /// The model assigned to this thread.
    #[serde(default)]
    pub model: Option<String>,
    /// All other fields are captured but not typed.
    #[serde(flatten)]
    pub extra: Value,
}

impl ThreadStartResponse {
    /// Convenience accessor for the thread ID.
    pub fn thread_id(&self) -> &str {
        &self.thread.id
    }
}

/// Parameters for `thread/archive`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ThreadArchiveParams {
    pub thread_id: String,
}

/// Response from `thread/archive`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ThreadArchiveResponse {}

/// Status of a thread, sent via [`ThreadStatusChangedNotification`].
///
/// Wire format is internally tagged on `"type"`, with the `Active` variant
/// carrying an `activeFlags` array of in-progress markers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub enum ThreadStatus {
    /// Thread is not yet loaded.
    NotLoaded,
    /// Thread is idle (no active turn).
    Idle,
    /// Thread has an active turn being processed.
    #[serde(rename_all = "camelCase")]
    Active {
        /// Tags identifying what is in flight (e.g. running tools).
        /// Shape is codex-version-dependent; preserved as raw JSON.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        active_flags: Vec<Value>,
    },
    /// Thread encountered an unrecoverable error.
    SystemError,
}

/// `thread/started` notification.
///
/// Sent once when a thread is created. Carries the full [`ThreadInfo`] for
/// the new thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ThreadStartedNotification {
    pub thread: ThreadInfo,
}

/// `thread/status/changed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ThreadStatusChangedNotification {
    pub thread_id: String,
    pub status: ThreadStatus,
}

/// `thread/tokenUsage/updated` notification.
///
/// Emitted after each turn with cumulative and per-turn token counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ThreadTokenUsageUpdatedNotification {
    pub thread_id: String,
    /// The turn that triggered this usage update.
    pub turn_id: String,
    pub token_usage: ThreadTokenUsage,
}

/// Cumulative token usage for a thread.
///
/// Carries per-turn (`last`) and lifetime (`total`) counts plus the model's
/// context window for client-side budget tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ThreadTokenUsage {
    /// Cumulative counts for the entire thread.
    pub total: TokenUsageBreakdown,
    /// Counts for the most recently completed turn.
    pub last: TokenUsageBreakdown,
    /// The model's maximum context window in tokens, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_context_window: Option<i64>,
}

/// A snapshot of token counts within a single turn or aggregated across a
/// thread. Sub-field of [`ThreadTokenUsage`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TokenUsageBreakdown {
    /// Sum total — may be redundant with the other counts.
    pub total_tokens: i64,
    /// Input tokens consumed.
    pub input_tokens: i64,
    /// Input tokens served from cache.
    pub cached_input_tokens: i64,
    /// Output tokens generated.
    pub output_tokens: i64,
    /// Output tokens spent on chain-of-thought reasoning (model-dependent).
    pub reasoning_output_tokens: i64,
}
