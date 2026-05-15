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
//! - **Approval flow types** — [`CommandExecutionApprovalParams`] and
//!   [`FileChangeApprovalParams`] for server-to-client requests that need a response
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
pub struct InitializeResponse {
    /// The server's user-agent string.
    pub user_agent: String,
}

// ---------------------------------------------------------------------------
// Thread lifecycle requests
// ---------------------------------------------------------------------------

/// Parameters for `thread/start`.
///
/// Use `ThreadStartParams::default()` for a basic thread with no custom instructions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub struct ThreadArchiveParams {
    pub thread_id: String,
}

/// Response from `thread/archive`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub struct TurnStartParams {
    /// The thread ID from [`ThreadStartResponse`].
    pub thread_id: String,
    /// One or more user inputs (text and/or images).
    pub input: Vec<UserInput>,
    /// Override the model for this turn (e.g., `"o4-mini"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Override reasoning effort for this turn (e.g., `"low"`, `"medium"`, `"high"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    /// Override sandbox policy for this turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_policy: Option<Value>,
}

/// Response from `turn/start`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartResponse {}

/// Parameters for `turn/interrupt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnInterruptParams {
    pub thread_id: String,
}

/// Response from `turn/interrupt`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnInterruptResponse {}

// ---------------------------------------------------------------------------
// Turn status & data types
// ---------------------------------------------------------------------------

/// Status of a turn within a [`Turn`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub struct Turn {
    /// Unique turn identifier.
    pub id: String,
    /// All items produced during this turn (messages, commands, file changes, etc.).
    #[serde(default)]
    pub items: Vec<ThreadItem>,
    /// Final status of the turn.
    pub status: TurnStatus,
    /// Error details if `status` is [`TurnStatus::Failed`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TurnError>,
}

// ---------------------------------------------------------------------------
// Token usage
// ---------------------------------------------------------------------------

/// A snapshot of token counts within a single turn or aggregated across a
/// thread. Sub-field of [`TokenUsage`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub enum ThreadStatus {
    /// Thread is not yet loaded.
    NotLoaded,
    /// Thread is idle (no active turn).
    Idle,
    /// Thread has an active turn being processed.
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
pub struct ThreadStartedNotification {
    pub thread: ThreadInfo,
}

/// `thread/status/changed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStatusChangedNotification {
    pub thread_id: String,
    pub status: ThreadStatus,
}

/// `turn/started` notification.
///
/// Carries the freshly-created [`Turn`] (with `status: in_progress`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartedNotification {
    pub thread_id: String,
    pub turn: Turn,
}

/// `turn/completed` notification.
///
/// Carries the final [`Turn`] state with its full item list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCompletedNotification {
    pub thread_id: String,
    pub turn: Turn,
}

/// `item/started` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub struct AgentMessageDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `item/commandExecution/outputDelta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CmdOutputDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `item/fileChange/outputDelta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeOutputDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `item/reasoning/summaryTextDelta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `error` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub struct AccountRateLimitsUpdatedNotification {
    pub rate_limits: RateLimits,
}

/// `mcpServer/startupStatus/updated` notification.
///
/// Emitted by the app-server as each managed MCP server transitions through
/// its startup lifecycle (e.g. `starting` → `ready` or `starting` → `failed`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub struct RemoteControlStatusChangedNotification {
    /// Status string (e.g. `"disabled"`, `"enabled"`).
    pub status: String,
    /// Connected environment id, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<String>,
}

/// `item/fileChange/patchUpdated` notification.
///
/// Emitted as the agent's tentative patch evolves during a turn. The
/// `changes` array carries the current file-by-file diff snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangePatchUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub changes: Vec<crate::FileUpdateChange>,
}

/// `item/plan/delta` notification (EXPERIMENTAL).
///
/// Proposed plan streaming deltas for plan items. Clients should not assume
/// concatenated deltas match the completed plan item content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

/// One step in a turn-level plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnPlanStep {
    pub step: String,
    pub status: TurnPlanStepStatus,
}

/// Lifecycle state of a [`TurnPlanStep`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnPlanStepStatus {
    Pending,
    InProgress,
    Completed,
}

/// `turn/plan/updated` notification.
///
/// Emitted when the agent updates its turn-level plan; the full plan is
/// resent on each update.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnPlanUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub plan: Vec<TurnPlanStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

/// `turn/diff/updated` notification.
///
/// Aggregated unified diff across all file changes in the current turn.
/// Sent whenever the diff materially changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnDiffUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub diff: String,
}

/// `item/reasoning/summaryPartAdded` notification.
///
/// Signals that a new summary block is starting in the agent's reasoning
/// stream. `summary_index` is the 0-based index of the new block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningSummaryPartAddedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub summary_index: i64,
}

/// `item/reasoning/textDelta` notification.
///
/// Streaming delta for the agent's verbose reasoning text (separate from
/// the summary stream).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningTextDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub content_index: i64,
    pub delta: String,
}

/// `mcpServer/oauthLogin/completed` notification.
///
/// Emitted when an MCP server's OAuth login flow completes (or fails).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerOauthLoginCompletedNotification {
    pub name: String,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// `account/login/completed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountLoginCompletedNotification {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// `deprecationNotice` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeprecationNoticeNotification {
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// `guardianWarning` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardianWarningNotification {
    pub thread_id: String,
    pub message: String,
}

/// `warning` notification (general advisory).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarningNotification {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

/// `thread/archived` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadArchivedNotification {
    pub thread_id: String,
}

/// `thread/closed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadClosedNotification {
    pub thread_id: String,
}

/// `thread/unarchived` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadUnarchivedNotification {
    pub thread_id: String,
}

/// `thread/goal/cleared` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadGoalClearedNotification {
    pub thread_id: String,
}

/// `thread/name/updated` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadNameUpdatedNotification {
    pub thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_name: Option<String>,
}

/// `skills/changed` notification (no payload).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsChangedNotification {}

/// `fs/changed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FsChangedNotification {
    pub watch_id: String,
    pub changed_paths: Vec<String>,
}

/// `configWarning` notification.
///
/// The `range` field's shape is left as raw JSON for now; expand to a typed
/// `SourceRange` if/when callers need it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigWarningNotification {
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<Value>,
}

// ---------------------------------------------------------------------------
// Notification stubs — these wrap the wire `Value` unchanged so that wire
// shape is preserved and the typed `Notification` enum can dispatch every
// method the schema enumerates. Each can be promoted to a fully-typed
// struct later without changing the dispatch surface.
// ---------------------------------------------------------------------------

/// `account/updated` notification (stub — accepts the wire shape unchanged).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct AccountUpdatedNotification(pub Value);

/// `app/list/updated` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct AppListUpdatedNotification(pub Value);

/// `command/exec/outputDelta` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct CommandExecOutputDeltaNotification(pub Value);

/// `externalAgentConfig/import/completed` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ExternalAgentConfigImportCompletedNotification(pub Value);

/// `fuzzyFileSearch/sessionCompleted` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FuzzyFileSearchSessionCompletedNotification(pub Value);

/// `fuzzyFileSearch/sessionUpdated` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FuzzyFileSearchSessionUpdatedNotification(pub Value);

/// `hook/completed` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct HookCompletedNotification(pub Value);

/// `hook/started` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct HookStartedNotification(pub Value);

/// `item/autoApprovalReview/completed` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ItemGuardianApprovalReviewCompletedNotification(pub Value);

/// `item/autoApprovalReview/started` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ItemGuardianApprovalReviewStartedNotification(pub Value);

/// `item/commandExecution/terminalInteraction` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct TerminalInteractionNotification(pub Value);

/// `item/mcpToolCall/progress` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct McpToolCallProgressNotification(pub Value);

/// `model/rerouted` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ModelReroutedNotification(pub Value);

/// `model/verification` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ModelVerificationNotification(pub Value);

/// `process/exited` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ProcessExitedNotification(pub Value);

/// `process/outputDelta` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ProcessOutputDeltaNotification(pub Value);

/// `serverRequest/resolved` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ServerRequestResolvedNotification(pub Value);

/// `thread/compacted` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ContextCompactedNotification(pub Value);

/// `thread/goal/updated` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ThreadGoalUpdatedNotification(pub Value);

/// `thread/realtime/closed` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ThreadRealtimeClosedNotification(pub Value);

/// `thread/realtime/error` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ThreadRealtimeErrorNotification(pub Value);

/// `thread/realtime/itemAdded` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ThreadRealtimeItemAddedNotification(pub Value);

/// `thread/realtime/outputAudio/delta` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ThreadRealtimeOutputAudioDeltaNotification(pub Value);

/// `thread/realtime/sdp` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ThreadRealtimeSdpNotification(pub Value);

/// `thread/realtime/started` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ThreadRealtimeStartedNotification(pub Value);

/// `thread/realtime/transcript/delta` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ThreadRealtimeTranscriptDeltaNotification(pub Value);

/// `thread/realtime/transcript/done` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ThreadRealtimeTranscriptDoneNotification(pub Value);

/// `windows/worldWritableWarning` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WindowsWorldWritableWarningNotification(pub Value);

/// `windowsSandbox/setupCompleted` notification (stub).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WindowsSandboxSetupCompletedNotification(pub Value);

// ---------------------------------------------------------------------------
// Approval flow types (server-to-client requests)
// ---------------------------------------------------------------------------

/// Decision for command execution approval.
///
/// Sent as part of [`CommandExecutionApprovalResponse`] when responding to
/// a command approval request from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandApprovalDecision {
    /// Allow this specific command to execute.
    Accept,
    /// Allow this command and similar future commands in this session.
    AcceptForSession,
    /// Reject this command.
    Decline,
    /// Cancel the entire turn.
    Cancel,
}

/// Parameters for `item/commandExecution/requestApproval` (server → client).
///
/// The server sends this as a [`ServerMessage::Request`] when the agent wants
/// to execute a command that requires user approval. Respond with
/// [`CommandExecutionApprovalResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionApprovalParams {
    pub thread_id: String,
    pub turn_id: String,
    /// Unique identifier for this tool call.
    pub call_id: String,
    /// The shell command the agent wants to run.
    pub command: String,
    /// Working directory for the command.
    pub cwd: String,
    /// Human-readable explanation of why the command is needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Response for `item/commandExecution/requestApproval`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionApprovalResponse {
    pub decision: CommandApprovalDecision,
}

/// Decision for file change approval.
///
/// Sent as part of [`FileChangeApprovalResponse`] when responding to
/// a file change approval request from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileChangeApprovalDecision {
    /// Allow this specific file change.
    Accept,
    /// Allow this and similar future file changes in this session.
    AcceptForSession,
    /// Reject this file change.
    Decline,
    /// Cancel the entire turn.
    Cancel,
}

/// Parameters for `item/fileChange/requestApproval` (server → client).
///
/// The server sends this as a [`ServerMessage::Request`] when the agent wants
/// to modify files and requires user approval. Respond with
/// [`FileChangeApprovalResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeApprovalParams {
    pub thread_id: String,
    pub turn_id: String,
    /// Unique identifier for this tool call.
    pub call_id: String,
    /// The proposed file changes (structure varies by patch format).
    pub changes: Value,
    /// Human-readable explanation of why the changes are needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Response for `item/fileChange/requestApproval`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeApprovalResponse {
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
    // Additional client → server requests (modeled at the method-string level
    // for coverage; typed *Params/*Response structs are added on demand).
    pub const THREAD_RESUME: &str = "thread/resume";
    pub const THREAD_FORK: &str = "thread/fork";
    pub const THREAD_UNSUBSCRIBE: &str = "thread/unsubscribe";
    pub const THREAD_NAME_SET: &str = "thread/name/set";
    pub const THREAD_METADATA_UPDATE: &str = "thread/metadata/update";
    pub const THREAD_UNARCHIVE: &str = "thread/unarchive";
    pub const THREAD_COMPACT_START: &str = "thread/compact/start";
    pub const THREAD_SHELLCOMMAND: &str = "thread/shellCommand";
    pub const THREAD_APPROVEGUARDIANDENIEDACTION: &str = "thread/approveGuardianDeniedAction";
    pub const THREAD_ROLLBACK: &str = "thread/rollback";
    pub const THREAD_LIST: &str = "thread/list";
    pub const THREAD_LOADED_LIST: &str = "thread/loaded/list";
    pub const THREAD_READ: &str = "thread/read";
    pub const THREAD_INJECT_ITEMS: &str = "thread/inject_items";
    pub const SKILLS_LIST: &str = "skills/list";
    pub const HOOKS_LIST: &str = "hooks/list";
    pub const MARKETPLACE_ADD: &str = "marketplace/add";
    pub const MARKETPLACE_REMOVE: &str = "marketplace/remove";
    pub const MARKETPLACE_UPGRADE: &str = "marketplace/upgrade";
    pub const PLUGIN_LIST: &str = "plugin/list";
    pub const PLUGIN_READ: &str = "plugin/read";
    pub const PLUGIN_SKILL_READ: &str = "plugin/skill/read";
    pub const PLUGIN_SHARE_SAVE: &str = "plugin/share/save";
    pub const PLUGIN_SHARE_UPDATETARGETS: &str = "plugin/share/updateTargets";
    pub const PLUGIN_SHARE_LIST: &str = "plugin/share/list";
    pub const PLUGIN_SHARE_CHECKOUT: &str = "plugin/share/checkout";
    pub const PLUGIN_SHARE_DELETE: &str = "plugin/share/delete";
    pub const APP_LIST: &str = "app/list";
    pub const FS_READFILE: &str = "fs/readFile";
    pub const FS_WRITEFILE: &str = "fs/writeFile";
    pub const FS_CREATEDIRECTORY: &str = "fs/createDirectory";
    pub const FS_GETMETADATA: &str = "fs/getMetadata";
    pub const FS_READDIRECTORY: &str = "fs/readDirectory";
    pub const FS_REMOVE: &str = "fs/remove";
    pub const FS_COPY: &str = "fs/copy";
    pub const FS_WATCH: &str = "fs/watch";
    pub const FS_UNWATCH: &str = "fs/unwatch";
    pub const SKILLS_CONFIG_WRITE: &str = "skills/config/write";
    pub const PLUGIN_INSTALL: &str = "plugin/install";
    pub const PLUGIN_UNINSTALL: &str = "plugin/uninstall";
    pub const REVIEW_START: &str = "review/start";
    pub const MODEL_LIST: &str = "model/list";
    pub const MODELPROVIDER_CAPABILITIES_READ: &str = "modelProvider/capabilities/read";
    pub const EXPERIMENTALFEATURE_LIST: &str = "experimentalFeature/list";
    pub const EXPERIMENTALFEATURE_ENABLEMENT_SET: &str = "experimentalFeature/enablement/set";
    pub const MCPSERVER_OAUTH_LOGIN: &str = "mcpServer/oauth/login";
    pub const CONFIG_MCPSERVER_RELOAD: &str = "config/mcpServer/reload";
    pub const MCPSERVERSTATUS_LIST: &str = "mcpServerStatus/list";
    pub const MCPSERVER_RESOURCE_READ: &str = "mcpServer/resource/read";
    pub const MCPSERVER_TOOL_CALL: &str = "mcpServer/tool/call";
    pub const WINDOWSSANDBOX_SETUPSTART: &str = "windowsSandbox/setupStart";
    pub const WINDOWSSANDBOX_READINESS: &str = "windowsSandbox/readiness";
    pub const ACCOUNT_LOGIN_START: &str = "account/login/start";
    pub const ACCOUNT_LOGIN_CANCEL: &str = "account/login/cancel";
    pub const ACCOUNT_LOGOUT: &str = "account/logout";
    pub const ACCOUNT_RATELIMITS_READ: &str = "account/rateLimits/read";
    pub const ACCOUNT_SENDADDCREDITSNUDGEEMAIL: &str = "account/sendAddCreditsNudgeEmail";
    pub const FEEDBACK_UPLOAD: &str = "feedback/upload";
    pub const COMMAND_EXEC: &str = "command/exec";
    pub const COMMAND_EXEC_WRITE: &str = "command/exec/write";
    pub const COMMAND_EXEC_TERMINATE: &str = "command/exec/terminate";
    pub const COMMAND_EXEC_RESIZE: &str = "command/exec/resize";
    pub const CONFIG_READ: &str = "config/read";
    pub const EXTERNALAGENTCONFIG_DETECT: &str = "externalAgentConfig/detect";
    pub const EXTERNALAGENTCONFIG_IMPORT: &str = "externalAgentConfig/import";
    pub const CONFIG_VALUE_WRITE: &str = "config/value/write";
    pub const CONFIG_BATCHWRITE: &str = "config/batchWrite";
    pub const CONFIGREQUIREMENTS_READ: &str = "configRequirements/read";
    pub const ACCOUNT_READ: &str = "account/read";
    pub const FUZZYFILESEARCH: &str = "fuzzyFileSearch";

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
    pub const MCP_SERVER_OAUTH_LOGIN_COMPLETED: &str = "mcpServer/oauthLogin/completed";
    pub const REMOTE_CONTROL_STATUS_CHANGED: &str = "remoteControl/status/changed";
    pub const FILE_CHANGE_PATCH_UPDATED: &str = "item/fileChange/patchUpdated";
    pub const PLAN_DELTA: &str = "item/plan/delta";
    pub const TURN_PLAN_UPDATED: &str = "turn/plan/updated";
    pub const TURN_DIFF_UPDATED: &str = "turn/diff/updated";
    pub const REASONING_SUMMARY_PART_ADDED: &str = "item/reasoning/summaryPartAdded";
    pub const REASONING_TEXT_DELTA: &str = "item/reasoning/textDelta";
    pub const ACCOUNT_LOGIN_COMPLETED: &str = "account/login/completed";
    pub const DEPRECATION_NOTICE: &str = "deprecationNotice";
    pub const GUARDIAN_WARNING: &str = "guardianWarning";
    pub const WARNING: &str = "warning";
    pub const THREAD_ARCHIVED: &str = "thread/archived";
    pub const THREAD_CLOSED: &str = "thread/closed";
    pub const THREAD_UNARCHIVED: &str = "thread/unarchived";
    pub const THREAD_GOAL_CLEARED: &str = "thread/goal/cleared";
    pub const THREAD_NAME_UPDATED: &str = "thread/name/updated";
    pub const SKILLS_CHANGED: &str = "skills/changed";
    pub const FS_CHANGED: &str = "fs/changed";
    pub const CONFIG_WARNING: &str = "configWarning";
    pub const ACCOUNT_UPDATED: &str = "account/updated";
    pub const APP_LIST_UPDATED: &str = "app/list/updated";
    pub const COMMAND_EXEC_OUTPUT_DELTA: &str = "command/exec/outputDelta";
    pub const EXTERNAL_AGENT_CONFIG_IMPORT_COMPLETED: &str = "externalAgentConfig/import/completed";
    pub const FUZZY_FILE_SEARCH_SESSION_COMPLETED: &str = "fuzzyFileSearch/sessionCompleted";
    pub const FUZZY_FILE_SEARCH_SESSION_UPDATED: &str = "fuzzyFileSearch/sessionUpdated";
    pub const HOOK_COMPLETED: &str = "hook/completed";
    pub const HOOK_STARTED: &str = "hook/started";
    pub const ITEM_AUTO_APPROVAL_REVIEW_COMPLETED: &str = "item/autoApprovalReview/completed";
    pub const ITEM_AUTO_APPROVAL_REVIEW_STARTED: &str = "item/autoApprovalReview/started";
    pub const ITEM_COMMAND_EXEC_TERMINAL_INTERACTION: &str =
        "item/commandExecution/terminalInteraction";
    pub const ITEM_MCP_TOOL_CALL_PROGRESS: &str = "item/mcpToolCall/progress";
    pub const MODEL_REROUTED: &str = "model/rerouted";
    pub const MODEL_VERIFICATION: &str = "model/verification";
    pub const PROCESS_EXITED: &str = "process/exited";
    pub const PROCESS_OUTPUT_DELTA: &str = "process/outputDelta";
    pub const SERVER_REQUEST_RESOLVED: &str = "serverRequest/resolved";
    pub const THREAD_COMPACTED: &str = "thread/compacted";
    pub const THREAD_GOAL_UPDATED: &str = "thread/goal/updated";
    pub const THREAD_REALTIME_CLOSED: &str = "thread/realtime/closed";
    pub const THREAD_REALTIME_ERROR: &str = "thread/realtime/error";
    pub const THREAD_REALTIME_ITEM_ADDED: &str = "thread/realtime/itemAdded";
    pub const THREAD_REALTIME_OUTPUT_AUDIO_DELTA: &str = "thread/realtime/outputAudio/delta";
    pub const THREAD_REALTIME_SDP: &str = "thread/realtime/sdp";
    pub const THREAD_REALTIME_STARTED: &str = "thread/realtime/started";
    pub const THREAD_REALTIME_TRANSCRIPT_DELTA: &str = "thread/realtime/transcript/delta";
    pub const THREAD_REALTIME_TRANSCRIPT_DONE: &str = "thread/realtime/transcript/done";
    pub const WINDOWS_WORLD_WRITABLE_WARNING: &str = "windows/worldWritableWarning";
    pub const WINDOWS_SANDBOX_SETUP_COMPLETED: &str = "windowsSandbox/setupCompleted";

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
        let json = r#"{"userAgent":"codex-cli/0.104.0"}"#;
        let resp: InitializeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.user_agent, "codex-cli/0.104.0");
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
            reasoning_effort: None,
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
        let json = r#"{
            "threadId": "th_1",
            "turnId": "t_1",
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
    fn test_command_approval_decision() {
        let json = r#""accept""#;
        let decision: CommandApprovalDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision, CommandApprovalDecision::Accept);

        let json = r#""acceptForSession""#;
        let decision: CommandApprovalDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision, CommandApprovalDecision::AcceptForSession);
    }

    #[test]
    fn test_command_approval_params() {
        let json = r#"{
            "threadId": "th_1",
            "turnId": "t_1",
            "callId": "call_1",
            "command": "rm -rf /tmp/test",
            "cwd": "/home/user"
        }"#;
        let params: CommandExecutionApprovalParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.command, "rm -rf /tmp/test");
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
