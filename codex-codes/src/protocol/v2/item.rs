//! Per-item notifications and the approval-flow request/response types.
//!
//! Mirrors upstream's `codex-rs/app-server-protocol/src/protocol/v2/item.rs`.

use crate::io::items::ThreadItem;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `item/started` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ItemStartedNotification {
    pub thread_id: String,
    pub turn_id: String,
    /// Server-side timestamp (milliseconds since Unix epoch) when the item
    /// began. Optional — older codex builds omit it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    pub item: ThreadItem,
}

/// `item/completed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ItemCompletedNotification {
    pub thread_id: String,
    pub turn_id: String,
    /// Server-side timestamp (milliseconds since Unix epoch) when the item
    /// finished. Optional — older codex builds omit it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<i64>,
    pub item: ThreadItem,
}

/// `item/agentMessage/delta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct AgentMessageDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `item/commandExecution/outputDelta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct CommandExecutionOutputDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `item/fileChange/outputDelta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct FileChangeOutputDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `item/reasoning/summaryTextDelta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ReasoningSummaryTextDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

// ── Approval flow types (server-to-client requests) ─────────────────

/// Decision for command execution approval.
///
/// Mirrors upstream's `CommandExecutionApprovalDecision`. The two tagged
/// variants carry payloads whose precise shape (`ExecPolicyAmendment` /
/// `NetworkPolicyAmendment`) is not modeled here; raw [`Value`] preserves
/// the wire data without dragging in the upstream subtypes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub enum CommandExecutionApprovalDecision {
    /// Allow this specific command to execute.
    Accept,
    /// Allow this command and similar future commands in this session.
    AcceptForSession,
    /// Allow this command and apply the proposed execpolicy amendment so
    /// future matching commands can run without prompting.
    AcceptWithExecpolicyAmendment {
        execpolicy_amendment: Value,
    },
    /// User chose a persistent network policy rule for this host.
    ApplyNetworkPolicyAmendment {
        network_policy_amendment: Value,
    },
    /// Reject this command; the turn will continue.
    Decline,
    /// Cancel the entire turn.
    Cancel,
}

/// Parameters for `item/commandExecution/requestApproval` (server → client).
///
/// Mirrors upstream's `CommandExecutionRequestApprovalParams`. The server
/// sends this as a [`crate::ServerMessage::Request`] when the agent wants
/// to execute a command that requires user approval. Respond with
/// [`CommandExecutionRequestApprovalResponse`].
///
/// Several upstream fields reference structured subtypes
/// (`NetworkApprovalContext`, `AdditionalPermissionProfile`,
/// `ExecPolicyAmendment`, `NetworkPolicyAmendment`, `CommandAction`) that we
/// do not model here; those are captured as [`Value`] so the wire round-trips
/// without losing data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct CommandExecutionRequestApprovalParams {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    /// Server-side timestamp (ms since Unix epoch) when this approval
    /// request was raised.
    pub started_at_ms: i64,
    /// Disambiguates multiple approval callbacks under the same `item_id`
    /// (used by zsh-exec-bridge subcommand prompts). `None` for regular
    /// shell/unified_exec approvals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<String>,
    /// Human-readable explanation of why the command is needed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Context for a managed-network approval prompt, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_approval_context: Option<Value>,
    /// The shell command the agent wants to run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Working directory for the command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Best-effort parsed command actions for friendly display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_actions: Option<Vec<Value>>,
    /// Additional permissions the agent is requesting for this command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_permissions: Option<Value>,
    /// Proposed execpolicy amendment to allow similar commands without
    /// prompting in the future.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_execpolicy_amendment: Option<Value>,
    /// Proposed network policy amendments (allow/deny host) for future
    /// requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_network_policy_amendments: Option<Vec<Value>>,
    /// Ordered list of decisions the client may present for this prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_decisions: Option<Vec<CommandExecutionApprovalDecision>>,
}

/// Response for `item/commandExecution/requestApproval`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct CommandExecutionRequestApprovalResponse {
    pub decision: CommandExecutionApprovalDecision,
}

/// Decision for file change approval.
///
/// Mirrors upstream's `FileChangeApprovalDecision`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub enum FileChangeApprovalDecision {
    /// Allow this specific file change.
    Accept,
    /// Allow this and similar future file changes in this session.
    AcceptForSession,
    /// Reject this file change; the turn will continue.
    Decline,
    /// Cancel the entire turn.
    Cancel,
}

/// Parameters for `item/fileChange/requestApproval` (server → client).
///
/// Mirrors upstream's `FileChangeRequestApprovalParams`. The proposed file
/// changes themselves are carried on the parent `FileChangeItem` (via
/// `item/started`), not on this approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct FileChangeRequestApprovalParams {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    /// Server-side timestamp (ms since Unix epoch) when this approval
    /// request was raised.
    pub started_at_ms: i64,
    /// Human-readable explanation of why the changes are needed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// When set, the agent is asking the user to grant writes under this
    /// root for the remainder of the session. Upstream marks this UNSTABLE.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_root: Option<String>,
}

/// Response for `item/fileChange/requestApproval`.
///
/// Upstream omits `#[serde(rename_all)]` on this struct, so the `decision`
/// field is wired as-is.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct FileChangeRequestApprovalResponse {
    pub decision: FileChangeApprovalDecision,
}
