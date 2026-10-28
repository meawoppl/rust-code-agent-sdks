//! Coverage for the CLI 2.1.312 stream-json drift.
//!
//! 2.1.311 → 2.1.312 added shell CPU accounting telemetry on `result`
//! (`shell_cpu_ms_turn`, `shell_cpu_ms_since_last_result`,
//! `shell_wall_ms_since_last_result`). No removals.
//!
//! The frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

/// A success result carries the shell CPU accounting group figures.
#[test]
fn success_result_carries_shell_cpu_telemetry() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 5000,
        "duration_api_ms": 4500,
        "num_turns": 1,
        "result": "Done",
        "stop_reason": "end_turn",
        "total_cost_usd": 0.02,
        "process_cpu_ms_turn": 900,
        "process_wall_ms_turn": 5000,
        "shell_cpu_ms_turn": 1250,
        "shell_cpu_ms_since_last_result": 40,
        "shell_wall_ms_since_last_result": 8000,
        "permission_denials": [],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.shell_cpu_ms_turn, Some(1250));
    assert_eq!(res.shell_cpu_ms_since_last_result, Some(40));
    assert_eq!(res.shell_wall_ms_since_last_result, Some(8000));
}

/// The process's first shell-accounted result has `shell_cpu_ms_turn` but no
/// since-last-result stretch, and the absent fields stay off the wire.
#[test]
fn first_result_omits_since_last_result_stretch() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 1000,
        "duration_api_ms": 900,
        "num_turns": 1,
        "result": "Done",
        "stop_reason": "end_turn",
        "total_cost_usd": 0.01,
        "shell_cpu_ms_turn": 0,
        "permission_denials": [],
        "uuid": "u2",
        "session_id": "s1"
    });
    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.shell_cpu_ms_turn, Some(0));
    assert_eq!(res.shell_cpu_ms_since_last_result, None);
    let wire = serde_json::to_value(&res).unwrap();
    assert!(wire.get("shell_cpu_ms_since_last_result").is_none());
    assert!(wire.get("shell_wall_ms_since_last_result").is_none());
}
