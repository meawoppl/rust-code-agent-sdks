//! Coverage for the CLI 2.1.282 stream-json drift.
//!
//! 2.1.281 → 2.1.282 added one optional top-level field to `result`:
//! `frame_intake_phases_ms`, a per-step breakdown of the time between
//! `frame_received_wall_ms` and `frame_enqueued_wall_ms`. No removals, no
//! new subtypes.
//!
//! The frame below carries the new field and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

fn result_frame() -> serde_json::Value {
    json!({
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
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "request_sent_wall_ms": 1753212345678.25,
        "frame_received_wall_ms": 1753212345600.5,
        "frame_enqueued_wall_ms": 1753212345612.5,
        "frame_intake_phases_ms": {
            "before_read": 3,
            "dedup": 0,
            "admission_wait": 7,
            "other": 2
        },
        "turn_started_wall_ms": 1753212345613.0
    })
}

/// `frame_intake_phases_ms` is typed as an open-set step→ms map and
/// round-trips fully wrapped.
#[test]
fn result_carries_frame_intake_phases() {
    let frame = result_frame();
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    let phases = res.frame_intake_phases_ms.as_ref().expect("typed phases");
    assert_eq!(phases.len(), 4);
    assert_eq!(phases.get("before_read"), Some(&3));
    assert_eq!(phases.get("dedup"), Some(&0));
    assert_eq!(phases.get("admission_wait"), Some(&7));
    assert_eq!(phases.get("other"), Some(&2));
    assert_eq!(phases.get("attachments"), None);
    assert_eq!(phases.values().sum::<u64>(), 12);
}

/// A step name this crate version does not know is kept, not dropped.
#[test]
fn frame_intake_phases_is_open_set() {
    let mut frame = result_frame();
    frame["frame_intake_phases_ms"]["future_step"] = json!(5);
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    let phases = res.frame_intake_phases_ms.unwrap();
    assert_eq!(phases.get("future_step"), Some(&5));
}

/// A pre-2.1.282 `result` (no intake breakdown) still parses with `None`
/// and does not serialize the key.
#[test]
fn frame_intake_phases_is_optional() {
    let mut frame = result_frame();
    frame
        .as_object_mut()
        .unwrap()
        .remove("frame_intake_phases_ms");
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert!(res.frame_intake_phases_ms.is_none());
    let back = serde_json::to_value(&res).unwrap();
    assert!(back.get("frame_intake_phases_ms").is_none());
}
