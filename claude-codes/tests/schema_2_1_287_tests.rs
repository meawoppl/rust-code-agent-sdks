//! Coverage for the CLI 2.1.287 stream-json drift.
//!
//! 2.1.286 → 2.1.287 added `thinking_duration_ms` on `assistant` frames,
//! `first_text_post_queue_wait_ms`, `first_text_post_queued_behind` and
//! `turn_start_resume_kind` on `result`, and the `home_files` / `file_names`
//! capability markers on `system/turn_handoff_available`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, StreamPostQueuedBehind, SystemSubtype, TurnStartResumeKind,
};
use serde_json::json;

/// A thinking-only `assistant` frame carries how long the block streamed.
#[test]
fn assistant_carries_thinking_duration_ms() {
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
        "thinking_duration_ms": 1375
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.thinking_duration_ms, Some(1375));
}

/// `result` carries the first text POST's queue wait and what it was queued
/// behind, plus what the turn had to settle before its own prompt.
#[test]
fn result_carries_text_post_queue_and_resume_kind() {
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
        "time_to_request_ms": 40,
        "time_to_request_phases_ms": {
            "system_prompt": 10,
            "resume_other": 1,
            "resume_store_confirm": 4,
            "turn_start_tool_run": 20,
            "other": 5
        },
        "turn_start_resume_kind": "turn_handoff",
        "first_text_post_ms": 70,
        "first_text_post_queue_wait_ms": 12,
        "first_text_post_queued_behind": "durable_post"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.turn_start_resume_kind,
        Some(TurnStartResumeKind::TurnHandoff)
    );
    assert_eq!(res.first_text_post_queue_wait_ms, Some(12));
    assert_eq!(
        res.first_text_post_queued_behind,
        Some(StreamPostQueuedBehind::DurablePost)
    );
    let phases = res.time_to_request_phases_ms.expect("phases");
    assert_eq!(phases.get("turn_start_tool_run"), Some(&20));
}

/// An unrecognized resume kind is kept, not rejected.
#[test]
fn turn_start_resume_kind_is_an_open_set() {
    let kind: TurnStartResumeKind = serde_json::from_value(json!("future_kind")).unwrap();
    assert_eq!(kind, TurnStartResumeKind::Unknown("future_kind".into()));
    assert_eq!(kind.to_string(), "future_kind");
    assert_eq!(
        serde_json::to_value(TurnStartResumeKind::DeferredToolUse).unwrap(),
        json!("deferred_tool_use")
    );
}

/// The two new capability markers on `system/turn_handoff_available` are
/// typed rather than landing in `extra`.
#[test]
fn turn_handoff_available_carries_home_files_and_file_names() {
    let frame = json!({
        "type": "system",
        "subtype": "turn_handoff_available",
        "v": 1,
        "tools": ["Bash"],
        "worker_epoch": 5,
        "staged_files": true,
        "home_files": true,
        "file_names": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::TurnHandoffAvailable);
    let handoff = sys.as_turn_handoff_available().expect("typed accessor");
    assert_eq!(handoff.home_files, Some(true));
    assert_eq!(handoff.file_names, Some(true));
    assert!(handoff.extra.is_empty());
}
