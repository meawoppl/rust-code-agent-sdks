//! Coverage for the CLI 2.1.314 stream-json drift.
//!
//! 2.1.312 → 2.1.314 added the `system/ui_placed` subtype, the
//! `sandbox_required_unavailable` startup failure reason, and four
//! `api_error` kinds (`stored_login_refused`, `connection_failed`,
//! `stream_idle_timeout`, `computer_slept`). No removals.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, StartupFailureReason, SystemSubtype,
};
use serde_json::json;

/// `system/ui_placed` carries the placed rows in order; the first row sits
/// at the conversation's start and has no `after_uuid`.
#[test]
fn system_ui_placed_fully_wrapped() {
    let frame = json!({
        "type": "system",
        "subtype": "ui_placed",
        "rows": [
            {
                "plugin": "deploy",
                "id": "banner",
                "request_id": "req-1"
            },
            {
                "plugin": "deploy",
                "id": "status",
                "request_id": "req-2",
                "after_uuid": "msg-7"
            }
        ],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::UiPlaced);
    assert!(sys.is_ui_placed());
    let placed = sys.as_ui_placed().expect("typed accessor");
    assert_eq!(placed.rows.len(), 2);
    assert_eq!(placed.rows[0].plugin, "deploy");
    assert_eq!(placed.rows[0].after_uuid, None);
    assert_eq!(placed.rows[1].request_id, "req-2");
    assert_eq!(placed.rows[1].after_uuid.as_deref(), Some("msg-7"));
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::UiPlaced(_))
    ));

    let wire = serde_json::to_value(&placed).unwrap();
    assert!(wire["rows"][0].get("after_uuid").is_none());
}

/// `sandbox_required_unavailable` is a typed startup failure reason.
#[test]
fn startup_failure_reason_sandbox_required_unavailable() {
    let frame = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "duration_ms": 0,
        "duration_api_ms": 0,
        "num_turns": 0,
        "stop_reason": null,
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.0,
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "errors": ["A sandbox is required, but bubblewrap is not installed."],
        "startup_failure_reason": "sandbox_required_unavailable"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::SandboxRequiredUnavailable)
    );
    assert_eq!(
        StartupFailureReason::SandboxRequiredUnavailable.to_string(),
        "sandbox_required_unavailable"
    );
}

/// The new `api_error` kinds pass through on an assistant API-error frame.
#[test]
fn assistant_api_error_new_kinds() {
    for kind in [
        "stored_login_refused",
        "connection_failed",
        "stream_idle_timeout",
        "computer_slept",
    ] {
        let frame = json!({
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "type": "message",
                "role": "assistant",
                "model": "<synthetic>",
                "content": [{"type": "text", "text": "API Error"}],
                "stop_reason": null,
                "usage": {
                    "input_tokens": 0,
                    "output_tokens": 0,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0
                }
            },
            "session_id": "s1",
            "uuid": "u1",
            "parent_tool_use_id": null,
            "is_api_error_message": true,
            "api_error": kind
        });
        assert_fully_wrapped(&frame);

        let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
            panic!("expected Assistant");
        };
        assert_eq!(msg.api_error.as_deref(), Some(kind));
    }
}
