//! Coverage for the CLI 2.1.296 stream-json drift.
//!
//! 2.1.295 → 2.1.296 added `thinking_display` on `assistant` frames and the
//! `hydrates_carried_lines` capability marker on
//! `system/turn_handoff_available`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

/// A thinking-only `assistant` frame names the display its request asked for.
#[test]
fn assistant_carries_thinking_display() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_01",
            "type": "message",
            "role": "assistant",
            "model": "claude-fable-5-1",
            "content": [{"type": "thinking", "thinking": "hmm", "signature": "sig"}],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {"input_tokens": 1, "output_tokens": 1}
        },
        "parent_tool_use_id": null,
        "session_id": "s1",
        "uuid": "u1",
        "thinking_duration_ms": 1375,
        "thinking_display": "omitted"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.thinking_display.as_deref(), Some("omitted"));
    assert_eq!(msg.thinking_duration_ms, Some(1375));
}

/// The `hydrates_carried_lines` capability marker is typed rather than
/// landing in `extra`.
#[test]
fn turn_handoff_available_carries_hydrates_carried_lines() {
    let frame = json!({
        "type": "system",
        "subtype": "turn_handoff_available",
        "v": 1,
        "tools": ["Write"],
        "worker_epoch": 7,
        "carried_writes": true,
        "hydrates_carried_lines": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let handoff = sys.as_turn_handoff_available().expect("typed accessor");
    assert_eq!(handoff.hydrates_carried_lines, Some(true));
    assert_eq!(handoff.carried_writes, Some(true));
    assert!(handoff.extra.is_empty());
}
