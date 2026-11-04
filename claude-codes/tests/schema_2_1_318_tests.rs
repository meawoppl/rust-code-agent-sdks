//! Coverage for the CLI 2.1.318 stream-json drift.
//!
//! 2.1.317 → 2.1.318 added `credential_expires_at` to both `result`
//! variants and the `config_file_invalid` startup failure reason. No
//! removals.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput, StartupFailureReason};
use serde_json::json;

/// `credential_expires_at` on a success result.
#[test]
fn result_success_credential_expires_at_fully_wrapped() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 1200,
        "duration_api_ms": 900,
        "num_turns": 1,
        "result": "ok",
        "stop_reason": "end_turn",
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.01,
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "result_index": 0,
        "credential_expires_at": "2026-12-03T10:00:00.000Z"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.credential_expires_at.as_deref(),
        Some("2026-12-03T10:00:00.000Z")
    );

    let wire = serde_json::to_value(&res).unwrap();
    assert_eq!(wire["credential_expires_at"], "2026-12-03T10:00:00.000Z");
}

/// Absent `credential_expires_at` stays absent on the way back out.
#[test]
fn result_credential_expires_at_absent_roundtrip() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 1,
        "duration_api_ms": 1,
        "num_turns": 1,
        "session_id": "s1",
        "total_cost_usd": 0.0
    });
    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert!(res.credential_expires_at.is_none());
    let wire = serde_json::to_value(&res).unwrap();
    assert!(wire.get("credential_expires_at").is_none());
}

/// `config_file_invalid` is a typed startup failure reason, and the error
/// result can carry `credential_expires_at` too.
#[test]
fn startup_failure_reason_config_file_invalid() {
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
        "errors": ["~/.claude.json is not valid JSON."],
        "startup_failure_reason": "config_file_invalid",
        "credential_expires_at": "2026-12-03T10:00:00.000Z"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::ConfigFileInvalid)
    );
    assert_eq!(
        StartupFailureReason::ConfigFileInvalid.to_string(),
        "config_file_invalid"
    );
    assert_eq!(
        StartupFailureReason::from("config_file_invalid"),
        StartupFailureReason::ConfigFileInvalid
    );
    assert!(res.credential_expires_at.is_some());
}
