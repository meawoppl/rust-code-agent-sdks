//! App-server protocol types for the Codex CLI.
//!
//! This module's tree mirrors upstream's
//! `codex-rs/app-server-protocol/src/protocol/` layout one file at a time, so
//! the canonical wire shape for any type can be found by reading the
//! identically-named upstream file in
//! [`tests/test_data/upstream/`](https://github.com/meawoppl/rust-code-agent-sdks/tree/main/codex-codes/tests/test_data/upstream).
//! That snapshot is enforced as a contract by
//! [`tests/protocol_name_conformance.rs`](https://github.com/meawoppl/rust-code-agent-sdks/blob/main/codex-codes/tests/protocol_name_conformance.rs).
//!
//! # Organization
//!
//! - [`v1`] — Initialize handshake types (`InitializeParams`,
//!   `InitializeResponse`, `ClientInfo`, ...).
//! - [`v2`] — Everything else, broken down further: [`v2::turn`],
//!   [`v2::thread`], [`v2::thread_data`], [`v2::item`], [`v2::notification`],
//!   [`v2::account`], [`v2::mcp`], [`v2::remote_control`].
//! - [`methods`] — JSON-RPC method-name string constants.
//!
//! Re-exports at the module root and at the crate root keep historic
//! `use codex_codes::TurnStartParams` paths working — moving a type between
//! files in upstream is transparent to consumers.
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

pub mod v1;
pub mod v2;

pub use v1::*;
pub use v2::*;

/// JSON-RPC method names used by the app-server protocol.
///
/// Use these constants when matching on [`crate::ServerMessage::Notification`] or
/// [`crate::ServerMessage::Request`] method fields to avoid typos.
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
    fn test_thread_start_params_default() {
        let params = ThreadStartParams::default();
        let json = serde_json::to_string(&params).unwrap();
        // Upstream's ThreadStartParams has only optional fields. None are
        // modeled here yet, so the wire payload is an empty object.
        assert_eq!(json, "{}");
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
        let json = r#"{"threadId":"th_1","turnId":"t_1","itemId":"msg_1","delta":"Hello "}"#;
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
        assert_eq!(
            params.command.as_deref(),
            Some("/bin/bash -lc 'touch quicksort.rs'")
        );
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
    fn test_thread_token_usage() {
        let json = r#"{
            "last":{"inputTokens":100,"outputTokens":200,"cachedInputTokens":50,"reasoningOutputTokens":0,"totalTokens":300},
            "total":{"inputTokens":1000,"outputTokens":2000,"cachedInputTokens":500,"reasoningOutputTokens":10,"totalTokens":3000},
            "modelContextWindow":200000
        }"#;
        let usage: ThreadTokenUsage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.last.input_tokens, 100);
        assert_eq!(usage.last.output_tokens, 200);
        assert_eq!(usage.last.cached_input_tokens, 50);
        assert_eq!(usage.total.input_tokens, 1000);
        assert_eq!(usage.model_context_window, Some(200000));
    }
}
