//! Coverage for the CLI 2.1.268 stream-json additive drift.
//!
//! 2.1.267 → 2.1.268 added top-level fields to the `assistant`,
//! `stream_event`, and `result` wire types, all additive (no removals):
//! `resume_reason` on all three, plus `result_index` and `local_command` on
//! `result`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

/// An `assistant` frame stamped with `resume_reason` on a restart re-run
/// round-trips without loss, and older frames leave the key absent.
#[test]
fn assistant_carries_resume_reason() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-8",
            "content": [{"type": "text", "text": "done"}],
            "stop_reason": null,
            "usage": {
                "input_tokens": 1,
                "output_tokens": 2,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            }
        },
        "session_id": "s1",
        "uuid": "u1",
        "parent_tool_use_id": null,
        "user_message_uuid": "um1",
        "resume_reason": "host_draining"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected assistant");
    };
    assert_eq!(msg.resume_reason.as_deref(), Some("host_draining"));
    assert_eq!(msg.user_message_uuid.as_deref(), Some("um1"));

    let old = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-8",
            "content": [],
            "stop_reason": null,
            "usage": {
                "input_tokens": 1,
                "output_tokens": 2,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            }
        },
        "session_id": "s1"
    });
    let ClaudeOutput::Assistant(msg) = serde_json::from_value(old).unwrap() else {
        panic!("expected assistant");
    };
    assert_eq!(msg.resume_reason, None);
    let reserialized = serde_json::to_string(&msg).unwrap();
    assert!(!reserialized.contains("resume_reason"));
}

/// A `stream_event` frame stamped with `resume_reason` round-trips fully
/// wrapped.
#[test]
fn stream_event_carries_resume_reason() {
    let frame = json!({
        "type": "stream_event",
        "event": {"type": "message_start"},
        "parent_tool_use_id": null,
        "uuid": "u1",
        "session_id": "s1",
        "user_message_uuid": "um1",
        "user_message_uuids": ["um1"],
        "resume_reason": "interrupted_turn"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::StreamEvent(ev) = serde_json::from_value(frame).unwrap() else {
        panic!("expected stream_event");
    };
    assert_eq!(ev.resume_reason.as_deref(), Some("interrupted_turn"));
}

/// A success `result` carrying the 2.1.268 `resume_reason`, `local_command`,
/// and `result_index` fields round-trips fully wrapped.
#[test]
fn result_success_carries_resume_reason_local_command_and_index() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 5,
        "duration_api_ms": 0,
        "num_turns": 1,
        "result": "ok",
        "session_id": "s1",
        "total_cost_usd": 0.0,
        "resume_reason": "checkpoint_restore",
        "local_command": "/status",
        "result_index": 3
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected result");
    };
    assert_eq!(res.resume_reason.as_deref(), Some("checkpoint_restore"));
    assert_eq!(res.local_command.as_deref(), Some("/status"));
    assert_eq!(res.result_index, Some(3));
}

/// An error `result` carries `resume_reason` and `result_index` too (the CLI
/// stamps the reason on the re-run's result whether success or error), and a
/// result without them leaves the keys absent on reserialization.
#[test]
fn result_error_carries_resume_reason_and_index() {
    let frame = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "duration_ms": 5,
        "duration_api_ms": 0,
        "num_turns": 0,
        "session_id": "s1",
        "total_cost_usd": 0.0,
        "resume_reason": "container_recreated",
        "result_index": 0
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected result");
    };
    assert_eq!(res.resume_reason.as_deref(), Some("container_recreated"));
    assert_eq!(res.result_index, Some(0));
    assert_eq!(res.local_command, None);

    let old = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "duration_ms": 5,
        "duration_api_ms": 0,
        "num_turns": 0,
        "session_id": "s1",
        "total_cost_usd": 0.0
    });
    let ClaudeOutput::Result(res) = serde_json::from_value(old).unwrap() else {
        panic!("expected result");
    };
    assert_eq!(res.resume_reason, None);
    assert_eq!(res.result_index, None);
    let reserialized = serde_json::to_string(&res).unwrap();
    assert!(!reserialized.contains("resume_reason"));
    assert!(!reserialized.contains("result_index"));
    assert!(!reserialized.contains("local_command"));
}
