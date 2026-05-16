//! App-server v2 protocol types for the Codex CLI.
//!
//! These types represent the JSON-RPC request parameters, response payloads,
//! and notification bodies used by `codex app-server`. All wire types use
//! camelCase field names via `#[serde(rename_all = "camelCase")]`.
//!
//! # Organization
//!
//! - **Request/Response pairs** — [`ThreadStartParams`]/[`ThreadStartResponse`],
//!   [`TurnStartParams`]/[`TurnStartResponse`], etc.
//! - **Server notifications** — Structs like [`TurnCompletedNotification`],
//!   [`AgentMessageDeltaNotification`] that can be deserialized from the `params`
//!   field of a [`ServerMessage::Notification`]
//! - **Approval flow types** — [`CommandExecutionRequestApprovalParams`] and
//!   [`FileChangeRequestApprovalParams`] for server-to-client requests that need a response
//! - **Method constants** — The [`methods`] module contains all JSON-RPC method
//!   name strings
//!
//! # Parsing notifications
//!
//! Prefer the typed dispatch in [`crate::messages`] over manual `method` checks:
//!
//! ```
//! use codex_codes::{Notification, ServerMessage};
//!
//! fn handle(msg: ServerMessage) {
//!     if let ServerMessage::Notification(Notification::TurnCompleted(c)) = msg {
//!         println!("Turn {} on thread {} completed", c.turn.id, c.thread_id);
//!     }
//! }
//! ```

use crate::io::items::ThreadItem;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// User input
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Initialization handshake
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Thread lifecycle requests
// ---------------------------------------------------------------------------

/// Parameters for `thread/start`.
///
/// Use `ThreadStartParams::default()` for a basic thread with no custom instructions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ThreadStartParams {
    /// Optional system instructions for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Optional tool definitions to make available to the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
}

/// Thread metadata returned inside a [`ThreadStartResponse`].
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

// ---------------------------------------------------------------------------
// Turn lifecycle requests
// ---------------------------------------------------------------------------

/// Parameters for `turn/start`.
///
/// Starts a new agent turn within an existing thread. The agent processes the
/// input and streams notifications until the turn completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TurnStartParams {
    /// The thread ID from [`ThreadStartResponse`].
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

// ---------------------------------------------------------------------------
// Turn status & data types
// ---------------------------------------------------------------------------

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
/// Included in [`TurnCompletedNotification`] when a turn finishes.
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

// ---------------------------------------------------------------------------
// Token usage
// ---------------------------------------------------------------------------

/// A snapshot of token counts within a single turn or aggregated across a
/// thread. Sub-field of [`TokenUsage`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TokenCounts {
    /// Input tokens consumed.
    #[serde(default)]
    pub input_tokens: u64,
    /// Output tokens generated.
    #[serde(default)]
    pub output_tokens: u64,
    /// Input tokens served from cache.
    #[serde(default)]
    pub cached_input_tokens: u64,
    /// Output tokens spent on chain-of-thought reasoning (model-dependent).
    #[serde(default)]
    pub reasoning_output_tokens: u64,
    /// Sum total — may be redundant with the other counts.
    #[serde(default)]
    pub total_tokens: u64,
}

/// Cumulative token usage for a thread.
///
/// Sent via [`ThreadTokenUsageUpdatedNotification`] after each turn. Carries
/// per-turn (`last`) and lifetime (`total`) counts plus the model's context
/// window for client-side budget tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct TokenUsage {
    /// Counts for the most recently completed turn.
    pub last: TokenCounts,
    /// Cumulative counts for the entire thread.
    pub total: TokenCounts,
    /// The model's maximum context window in tokens.
    pub model_context_window: u64,
}

// ---------------------------------------------------------------------------
// Thread status
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Server notifications
// ---------------------------------------------------------------------------

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
pub struct CmdOutputDeltaNotification {
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
pub struct ReasoningDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

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

/// `thread/tokenUsage/updated` notification.
///
/// Emitted after each turn with cumulative and per-turn token counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct ThreadTokenUsageUpdatedNotification {
    pub thread_id: String,
    /// The turn that triggered this usage update. May be absent for
    /// thread-level updates that aren't tied to a specific turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub token_usage: TokenUsage,
}

/// A rate-limit window descriptor used inside [`RateLimits`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct RateLimitWindow {
    /// Unix timestamp (seconds) at which this rate-limit window resets.
    pub resets_at: i64,
    /// Percentage of the window already consumed (0-100).
    pub used_percent: i32,
    /// Length of the rate-limit window, in minutes.
    pub window_duration_mins: i64,
}

/// Rate-limit envelope sent in [`AccountRateLimitsUpdatedNotification`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct RateLimits {
    /// Credit balance, if applicable for this plan. Shape is plan-dependent
    /// so the payload is preserved as raw JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits: Option<Value>,
    /// Stable machine identifier for the limit (e.g. `"codex"`).
    pub limit_id: String,
    /// Human-readable label, if the server provides one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_name: Option<String>,
    /// Plan tier (e.g. `"free"`, `"plus"`, `"team"`).
    pub plan_type: String,
    /// Primary (short-term) rate-limit window, if active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<RateLimitWindow>,
    /// Secondary (longer-term) rate-limit window, if active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary: Option<RateLimitWindow>,
    /// Which window (if any) the account has already hit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_reached_type: Option<String>,
}

/// `account/rateLimits/updated` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct AccountRateLimitsUpdatedNotification {
    pub rate_limits: RateLimits,
}

/// `mcpServer/startupStatus/updated` notification.
///
/// Emitted by the app-server as each managed MCP server transitions through
/// its startup lifecycle (e.g. `starting` → `ready` or `starting` → `failed`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct McpServerStartupStatusUpdatedNotification {
    /// MCP server identifier.
    pub name: String,
    /// Current lifecycle status string (e.g. `"starting"`, `"ready"`,
    /// `"failed"`). Kept as `String` so new status values don't break parsing.
    pub status: String,
    /// Error message if startup failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

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

// ---------------------------------------------------------------------------
// Approval flow types (server-to-client requests)
// ---------------------------------------------------------------------------

/// Decision for command execution approval.
///
/// Mirrors upstream's `CommandExecutionApprovalDecision` in
/// `codex-rs/app-server-protocol/src/protocol/v2/item.rs`. The two tagged
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
/// Mirrors upstream's `CommandExecutionRequestApprovalParams` in
/// `codex-rs/app-server-protocol/src/protocol/v2/item.rs`. The server sends
/// this as a [`ServerMessage::Request`] when the agent wants to execute a
/// command that requires user approval. Respond with
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
/// Mirrors upstream's `FileChangeApprovalDecision` in
/// `codex-rs/app-server-protocol/src/protocol/v2/item.rs`.
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
/// Mirrors upstream's `FileChangeRequestApprovalParams` in
/// `codex-rs/app-server-protocol/src/protocol/v2/item.rs`. The proposed file
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

// The [`ServerMessage`] enum that clients return now lives in
// [`crate::messages`] alongside the typed [`crate::Notification`] and
// [`crate::ServerRequest`] dispatch enums.

// ---------------------------------------------------------------------------
// Method name constants
// ---------------------------------------------------------------------------

/// JSON-RPC method names used by the app-server protocol.
///
/// Use these constants when matching on [`ServerMessage::Notification`] or
/// [`ServerMessage::Request`] method fields to avoid typos.
pub mod methods {
    // Client → server requests
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "initialized";
    pub const THREAD_START: &str = "thread/start";
    pub const THREAD_ARCHIVE: &str = "thread/archive";
    pub const TURN_START: &str = "turn/start";
    pub const TURN_INTERRUPT: &str = "turn/interrupt";
    pub const TURN_STEER: &str = "turn/steer";

    // Server → client notifications
    pub const THREAD_STARTED: &str = "thread/started";
    pub const THREAD_STATUS_CHANGED: &str = "thread/status/changed";
    pub const THREAD_TOKEN_USAGE_UPDATED: &str = "thread/tokenUsage/updated";
    pub const TURN_STARTED: &str = "turn/started";
    pub const TURN_COMPLETED: &str = "turn/completed";
    pub const ITEM_STARTED: &str = "item/started";
    pub const ITEM_COMPLETED: &str = "item/completed";
    pub const AGENT_MESSAGE_DELTA: &str = "item/agentMessage/delta";
    pub const CMD_OUTPUT_DELTA: &str = "item/commandExecution/outputDelta";
    pub const FILE_CHANGE_OUTPUT_DELTA: &str = "item/fileChange/outputDelta";
    pub const REASONING_DELTA: &str = "item/reasoning/summaryTextDelta";
    pub const ERROR: &str = "error";
    pub const ACCOUNT_RATE_LIMITS_UPDATED: &str = "account/rateLimits/updated";
    pub const MCP_SERVER_STARTUP_STATUS_UPDATED: &str = "mcpServer/startupStatus/updated";
    pub const REMOTE_CONTROL_STATUS_CHANGED: &str = "remoteControl/status/changed";

    // Server → client requests (approval)
    pub const CMD_EXEC_APPROVAL: &str = "item/commandExecution/requestApproval";
    pub const FILE_CHANGE_APPROVAL: &str = "item/fileChange/requestApproval";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_params() {
        let params = InitializeParams {
            client_info: ClientInfo {
                name: "my-app".to_string(),
                version: "1.0.0".to_string(),
                title: Some("My App".to_string()),
            },
            capabilities: None,
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("clientInfo"));
        assert!(json.contains("my-app"));
        assert!(!json.contains("capabilities"));
    }

    #[test]
    fn test_initialize_response() {
        let json = r#"{
            "userAgent": "codex-cli/0.130.0",
            "codexHome": "/home/u/.codex",
            "platformFamily": "unix",
            "platformOs": "linux"
        }"#;
        let resp: InitializeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.user_agent, "codex-cli/0.130.0");
        assert_eq!(resp.codex_home, "/home/u/.codex");
        assert_eq!(resp.platform_family, "unix");
        assert_eq!(resp.platform_os, "linux");
    }

    #[test]
    fn test_initialize_capabilities() {
        let params = InitializeParams {
            client_info: ClientInfo {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                title: None,
            },
            capabilities: Some(InitializeCapabilities {
                experimental_api: true,
                opt_out_notification_methods: Some(vec!["thread/started".to_string()]),
            }),
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("experimentalApi"));
        assert!(json.contains("optOutNotificationMethods"));
    }

    #[test]
    fn test_user_input_text() {
        let input = UserInput::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains(r#""type":"text""#));
        let parsed: UserInput = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, UserInput::Text { text } if text == "Hello"));
    }

    #[test]
    fn test_thread_start_params() {
        let params = ThreadStartParams {
            instructions: Some("Be helpful".to_string()),
            tools: None,
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("instructions"));
        assert!(!json.contains("tools"));
    }

    #[test]
    fn test_thread_start_response() {
        let json = r#"{"thread":{"id":"th_abc123"},"model":"gpt-4","approvalPolicy":"never","cwd":"/tmp","modelProvider":"openai","sandbox":{}}"#;
        let resp: ThreadStartResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.thread_id(), "th_abc123");
        assert_eq!(resp.model.as_deref(), Some("gpt-4"));
    }

    #[test]
    fn test_turn_start_params() {
        let params = TurnStartParams {
            thread_id: "th_1".to_string(),
            input: vec![UserInput::Text {
                text: "What is 2+2?".to_string(),
            }],
            model: None,
            effort: None,
            sandbox_policy: None,
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("threadId"));
        assert!(json.contains("input"));
    }

    #[test]
    fn test_turn_status() {
        let json = r#""completed""#;
        let status: TurnStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status, TurnStatus::Completed);
    }

    #[test]
    fn test_turn_completed_notification() {
        // Upstream `TurnCompletedNotification` has only `threadId` and `turn`
        // — no top-level `turnId`. The turn's own id is nested under `turn.id`.
        let json = r#"{
            "threadId": "th_1",
            "turn": {
                "id": "t_1",
                "items": [],
                "status": "completed"
            }
        }"#;
        let notif: TurnCompletedNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notif.thread_id, "th_1");
        assert_eq!(notif.turn.status, TurnStatus::Completed);
    }

    #[test]
    fn test_agent_message_delta() {
        let json = r#"{"threadId":"th_1","itemId":"msg_1","delta":"Hello "}"#;
        let notif: AgentMessageDeltaNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notif.delta, "Hello ");
    }

    #[test]
    fn test_command_execution_approval_decision_bare_variants() {
        let json = r#""accept""#;
        let decision: CommandExecutionApprovalDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision, CommandExecutionApprovalDecision::Accept);

        let json = r#""acceptForSession""#;
        let decision: CommandExecutionApprovalDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision, CommandExecutionApprovalDecision::AcceptForSession);
    }

    #[test]
    fn test_command_execution_approval_decision_tagged_variants() {
        // Shape observed on the wire from codex-cli 0.130 in the
        // `availableDecisions` field. The inner field name on the
        // execpolicy variant is snake_case because `rename_all` on the
        // outer enum only renames variants, not struct-variant fields —
        // matching upstream's definition.
        let json = r#"{
            "acceptWithExecpolicyAmendment": {
                "execpolicy_amendment": ["touch"]
            }
        }"#;
        let decision: CommandExecutionApprovalDecision = serde_json::from_str(json).unwrap();
        assert!(matches!(
            decision,
            CommandExecutionApprovalDecision::AcceptWithExecpolicyAmendment { .. }
        ));
    }

    #[test]
    fn test_command_execution_request_approval_params_current_wire() {
        // Shape observed on codex-cli 0.130. Mirrors upstream's
        // `CommandExecutionRequestApprovalParams`.
        let json = r#"{
            "threadId": "th_1",
            "turnId": "t_1",
            "itemId": "call_X7SaEBLhZSJlFA1PGftlLsTP",
            "startedAtMs": 1778939797972,
            "reason": "Do you want to allow creating quicksort.rs?",
            "command": "/bin/bash -lc 'touch quicksort.rs'",
            "cwd": "/tmp/work",
            "commandActions": [{"type":"unknown","command":"touch quicksort.rs"}],
            "proposedExecpolicyAmendment": ["touch"],
            "availableDecisions": [
                "accept",
                {"acceptWithExecpolicyAmendment": {"execpolicy_amendment": ["touch"]}},
                "cancel"
            ]
        }"#;
        let params: CommandExecutionRequestApprovalParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.item_id, "call_X7SaEBLhZSJlFA1PGftlLsTP");
        assert_eq!(params.started_at_ms, 1778939797972);
        assert_eq!(params.command.as_deref(), Some("/bin/bash -lc 'touch quicksort.rs'"));
        assert_eq!(params.command_actions.as_ref().map(|v| v.len()), Some(1));
        assert_eq!(
            params.available_decisions.as_ref().map(|v| v.len()),
            Some(3)
        );
    }

    #[test]
    fn test_file_change_request_approval_params_minimal() {
        // Upstream `FileChangeRequestApprovalParams` only requires the
        // three ids and a timestamp; `reason` and `grant_root` are optional.
        let json = r#"{
            "threadId": "th_1",
            "turnId": "t_1",
            "itemId": "call_2",
            "startedAtMs": 1778939797000
        }"#;
        let params: FileChangeRequestApprovalParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.item_id, "call_2");
        assert_eq!(params.started_at_ms, 1778939797000);
        assert!(params.reason.is_none());
        assert!(params.grant_root.is_none());
    }

    #[test]
    fn test_error_notification() {
        let json = r#"{"error":"something failed","willRetry":true}"#;
        let notif: ErrorNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notif.error, "something failed");
        assert!(notif.will_retry);
    }

    #[test]
    fn test_thread_status_idle() {
        let json = r#"{"type":"idle"}"#;
        let status: ThreadStatus = serde_json::from_str(json).unwrap();
        assert!(matches!(status, ThreadStatus::Idle));
    }

    #[test]
    fn test_thread_status_active_with_flags() {
        let json = r#"{"type":"active","activeFlags":[]}"#;
        let status: ThreadStatus = serde_json::from_str(json).unwrap();
        match status {
            ThreadStatus::Active { active_flags } => assert!(active_flags.is_empty()),
            other => panic!("expected Active, got {:?}", other),
        }
    }

    #[test]
    fn test_token_usage() {
        let json = r#"{
            "last":{"inputTokens":100,"outputTokens":200,"cachedInputTokens":50,"reasoningOutputTokens":0,"totalTokens":300},
            "total":{"inputTokens":1000,"outputTokens":2000,"cachedInputTokens":500,"reasoningOutputTokens":10,"totalTokens":3000},
            "modelContextWindow":200000
        }"#;
        let usage: TokenUsage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.last.input_tokens, 100);
        assert_eq!(usage.last.output_tokens, 200);
        assert_eq!(usage.last.cached_input_tokens, 50);
        assert_eq!(usage.total.input_tokens, 1000);
        assert_eq!(usage.model_context_window, 200000);
    }
}
