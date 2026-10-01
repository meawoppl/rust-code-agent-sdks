//! Coverage for the CLI 2.1.286 stream-json drift.
//!
//! 2.1.285 → 2.1.286 added `server_received_wall_ms` on `user` frames and
//! `first_request_input_tokens` plus `user_message_server_received_wall_ms`
//! on `result`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

/// A `user` frame stamped with the session server's receipt time.
#[test]
fn user_carries_server_received_wall_ms() {
    let frame = json!({
        "type": "user",
        "message": {"role": "user", "content": [{"type": "text", "text": "hello"}]},
        "parent_tool_use_id": null,
        "session_id": "1d1c4a2e-6f1b-4b7e-9c3d-2e5f6a7b8c9d",
        "uuid": "3f8b2c1a-9d4e-4f6a-8b7c-1a2b3c4d5e6f",
        "server_received_wall_ms": 1753212345500.75
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(user.server_received_wall_ms, Some(1753212345500.75));
}

/// `result` carries the first request's prompt size and the triggering user
/// message's server receipt time alongside `request_sent_wall_ms`.
#[test]
fn result_carries_first_request_input_tokens_and_server_receipt() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 1200,
        "duration_api_ms": 900,
        "num_turns": 1,
        "result": "done",
        "stop_reason": "end_turn",
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.01,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 38,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0
        },
        "modelUsage": {},
        "permission_denials": [],
        "request_sent_wall_ms": 1753212345678.25,
        "first_request_input_tokens": 18234,
        "user_message_uuid": "um-1",
        "user_message_server_received_wall_ms": 1753212345500.75
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.first_request_input_tokens, Some(18234));
    assert_eq!(
        res.user_message_server_received_wall_ms,
        Some(1753212345500.75)
    );
}
