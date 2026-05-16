//! Turn data structures shared across notifications.
//!
//! Mirrors upstream's `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs`.

use crate::io::items::ThreadItem;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Status of a turn within a [`Turn`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub enum TurnStatus {
    /// The agent finished normally.
    Completed,
    /// The turn was interrupted by the client via `turn/interrupt`.
    Interrupted,
    /// The turn failed with an error (see [`Turn::error`]).
    Failed,
    /// The turn is still being processed.
    InProgress,
}

/// Error information from a failed turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TurnError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex_error_info: Option<Value>,
}

/// A completed turn with its items and final status.
///
/// Included in [`crate::TurnCompletedNotification`] when a turn finishes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct Turn {
    /// Unique turn identifier.
    pub id: String,
    /// All items produced during this turn (messages, commands, file changes, etc.).
    #[serde(default)]
    pub items: Vec<ThreadItem>,
    /// Description of how much of `items` has been loaded for this turn.
    /// Shape is upstream's `TurnItemsView`; preserved as raw JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items_view: Option<Value>,
    /// Final status of the turn.
    pub status: TurnStatus,
    /// Error details if `status` is [`TurnStatus::Failed`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<TurnError>,
    /// Unix timestamp (seconds) when the turn started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    /// Unix timestamp (seconds) when the turn completed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    /// Wall-clock duration between turn start and completion, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

