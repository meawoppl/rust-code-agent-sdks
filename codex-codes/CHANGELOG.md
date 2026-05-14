# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.130.0] - 2026-05-14

Version jumps from 0.101.x to 0.130.0 to align the crate version with the
Codex CLI version it targets (same convention as the sibling `claude-codes`
crate, which tracks the Claude Code CLI version).


### Added

- **Typed message dispatch** — New module [`crate::messages`] introduces closed enums [`Notification`] and [`ServerRequest`] that wrap the per-method param structs. The clients now return the typed [`ServerMessage`] from `next_message()`/`events()` instead of the loose `{ method, params }` shape. Mirrors the [`ContentBlock`]-style dispatch in the sibling `claude-codes` crate: hand-written `Serialize`/`Deserialize` impls inspect the `method` discriminant, route known cases through their typed struct, and route unknown methods to an `Unknown { method, params }` fallback for forward compatibility. Known methods whose payload doesn't fit error loudly — the typing contract is enforced.
- **New typed notifications** — `AccountRateLimitsUpdatedNotification` (`account/rateLimits/updated`), `McpServerStartupStatusUpdatedNotification` (`mcpServer/startupStatus/updated`), `RemoteControlStatusChangedNotification` (`remoteControl/status/changed`). The app-server emits all three during normal operation; previously they fell through to the raw fallback.
- **`RateLimits` / `RateLimitWindow` / `TokenCounts`** — Supporting structs for the above and for the new nested `TokenUsage` shape.
- **`UserMessageItem` / `UserMessageContent`** — Added `ThreadItem::UserMessage` variant for the user-prompt item the app-server emits at the start of each turn (the exec JSONL protocol doesn't typically emit this).
- **Strict typed-message audit test** — `tests/live_client_tests.rs::test_typed_message_audit_strict` runs a real turn and asserts that every notification and server request resolves to a typed variant (no `Unknown`).

### Fixed

- **`ThreadStartedNotification`** — Was `{ thread_id: String }`; the wire actually sends `{ thread: ThreadInfo }` with the full info object.
- **`TurnStartedNotification` / `TurnCompletedNotification`** — Had a top-level `turn_id` field that doesn't exist on the wire (the id lives inside `turn.id`). Both now carry `{ thread_id, turn: Turn }`.
- **`ThreadStatusChangedNotification` / `ThreadStatus`** — Status was modeled as a bare string enum; the wire actually sends an internally-tagged enum (`{"type":"idle"}`, `{"type":"active","activeFlags":[]}`). `ThreadStatus` is now `#[serde(tag = "type")]` with the `Active` variant carrying `active_flags`.
- **`ThreadTokenUsageUpdatedNotification`** — Field was named `usage` but the wire sends `tokenUsage`. The notification also carries `turn_id` which wasn't modeled. Plus the inner `TokenUsage` is actually a wrapper of `{last, total, modelContextWindow}` over a `TokenCounts` struct with `inputTokens`, `outputTokens`, `cachedInputTokens`, `reasoningOutputTokens`, `totalTokens` — restructured accordingly.
- **`ItemStartedNotification` / `ItemCompletedNotification`** — Now include `started_at_ms` / `completed_at_ms` from the wire.
- **`CommandExecutionItem`** — Was snake_case-only (`aggregated_output`, `exit_code`); the app-server protocol uses camelCase. Now carries `#[serde(rename_all = "camelCase")]` with snake_case aliases so both protocols deserialize cleanly. `aggregated_output` is now `Option<String>` since the app-server sends `null` while a command is still in progress.

### Changed (breaking)

- **`ServerMessage` shape** — `ServerMessage::Notification { method, params }` is now `ServerMessage::Notification(Notification)`; `ServerMessage::Request { id, method, params }` is now `ServerMessage::Request { id, request: ServerRequest }`. Update call sites to match on the typed enum variants instead of method-string comparisons.
- **`CommandExecutionItem.aggregated_output`** — Type changed from `String` to `Option<String>` (see above).
- **`TokenUsage`** — Restructured from a flat counts struct to `{last, total, modelContextWindow}` over `TokenCounts`. Old direct field access (`usage.input_tokens`) becomes `usage.last.input_tokens` or `usage.total.input_tokens`.
- **`ThreadStatus`** — Now a `#[serde(tag = "type")]` enum; the `Active` variant has an `active_flags: Vec<Value>` field. Pattern matches need updating.

## [0.101.2] - 2026-05-14

### Fixed

- **Stderr pipe deadlock** — `AsyncClient` and `SyncClient` now drain the app-server's stderr in a background task/thread instead of leaving it pinned to an unread `BufReader`. The Codex CLI emits ~200 KB/s of tracing to stderr, which would fill the ~64 KB kernel pipe within a fraction of a second and block the child process — manifesting as the client hanging on the first non-trivial request. Drained lines are forwarded through the `log` crate at the level encoded in the line (`error!`/`warn!`/`debug!`/`trace!`), with ANSI color codes stripped. INFO tracing (the vast majority of volume) is routed to `trace!` so `RUST_LOG=info` stays quiet by default while WARN/ERROR remain visible.

### Removed

- **`AsyncClient::take_stderr()`** — Replaced by automatic background draining; the method is incompatible with the new design and is removed without a deprecation cycle (no known external callers).

## [0.101.1] - 2026-03-17

### Added

- **Binary path resolution via `which`** — `AppServerBuilder::spawn()` and `spawn_sync()` now resolve non-absolute binary paths using `which` at spawn time, producing a clear `BinaryNotFound` error instead of an opaque OS "file not found" (#102)
- **`Error::BinaryNotFound`** — New error variant for when the CLI binary isn't found on PATH

### Changed

- **`spawn_sync()` return type** — Now returns `crate::error::Result<Child>` instead of `std::io::Result<Child>` for consistent error handling

## [0.101.0] - 2026-02-23

### Added

- **`initialize` handshake** — `AsyncClient::start()` and `SyncClient::start()` now send the required `initialize` request followed by an `initialized` notification before returning, fixing compatibility with Codex CLI 0.104.0+ which requires this handshake before accepting other methods (#87)
- **`InitializeParams`, `InitializeResponse`, `ClientInfo`, `InitializeCapabilities`** — New protocol types for the initialization handshake
- **`AsyncClient::spawn()` / `SyncClient::spawn()`** — Low-level constructors that skip automatic initialization, for callers that need custom `InitializeParams`
- **`AsyncClient::initialize()` / `SyncClient::initialize()`** — Explicit initialization method for use with `spawn()`
- **`methods::INITIALIZE` / `methods::INITIALIZED`** — Method name constants for the initialization handshake
- **`ThreadInfo`** — New struct for thread metadata returned inside `ThreadStartResponse`
- **Integration tests** — Live client tests against a real Codex app-server process (behind `integration-tests` feature flag)

### Changed

- **`ThreadStartResponse`** — Updated to match the actual app-server wire format: now contains a `thread: ThreadInfo` field with the thread ID and metadata, plus optional `model` field. Use `response.thread_id()` to get the thread ID.

### Breaking

- `ThreadStartResponse.thread_id` field replaced by `ThreadStartResponse.thread_id()` method

## [0.100.1] - 2026-02-21

### Changed

- **Replaced `codex exec` with `codex app-server` JSON-RPC protocol** — The crate now wraps `codex app-server --listen stdio://` instead of the one-shot `codex exec --json -`. This enables multi-turn conversations, approval flows, and streaming notifications.

### Added

- **`jsonrpc` module** — JSON-RPC message types (`JsonRpcMessage`, `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonRpcNotification`, `RequestId`) matching the app-server wire format (no `"jsonrpc":"2.0"` field)
- **`protocol` module** — App-server v2 protocol types including thread/turn lifecycle params, server notifications, approval flow types, and method name constants
- **`AppServerBuilder`** — Replaces `CodexCliBuilder` for spawning the long-lived app-server process
- **Multi-turn `AsyncClient`** — JSON-RPC client with `thread_start`, `turn_start`, `turn_interrupt`, `thread_archive`, `respond`, and `next_message` methods
- **Multi-turn `SyncClient`** — Blocking counterpart with the same API surface
- **Approval flow support** — `CommandExecutionApprovalParams/Response` and `FileChangeApprovalParams/Response` for handling server-to-client approval requests
- **Streaming delta notifications** — `AgentMessageDeltaNotification`, `CmdOutputDeltaNotification`, `FileChangeOutputDeltaNotification`, `ReasoningDeltaNotification`
- **camelCase serde aliases** on `ThreadItem` variants and status enums for app-server compatibility (snake_case exec format still supported)
- **`Declined` variant** on `CommandExecutionStatus` for commands rejected via approval flow
- **Comprehensive documentation** — Module-level docs with examples, lifecycle guides, error conditions, notification reference tables, and rustdoc examples across all public modules

### Removed

- `CodexCliBuilder` (replaced by `AppServerBuilder`)
- One-shot exec client API

## [0.100.0] - 2026-02-17

### Added

- Initial release of `codex-codes` crate
- Typed Rust bindings for the OpenAI Codex CLI JSON protocol
- `ThreadEvent` and `ThreadItem` types for parsing exec-format JSONL events
- `ThreadOptions` configuration types (`ApprovalMode`, `SandboxMode`, `WebSearchMode`)
- Sync and async clients wrapping `codex exec --json -`
- Feature flags: `types` (WASM-compatible), `sync-client`, `async-client`
- Integration tests with captured protocol message test cases
- Version compatibility checking against installed Codex CLI
