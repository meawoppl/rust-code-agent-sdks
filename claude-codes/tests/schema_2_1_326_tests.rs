//! Coverage for the CLI 2.1.326 stream-json drift.
//!
//! 2.1.325 → 2.1.326 added the `system/late_allow_not_run` subtype,
//! `memory_check_detail` on the result, `origin` / `label` / `plugin_name`
//! on `system/hook_started`, `backgrounded` on `system/hook_progress`,
//! `summary_is_report` on `system/task_notification`, and the input-only
//! `keeps_request_id` on `user`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, LateAllowNotRunReason, MemoryCheckHold,
    MemoryCheckWait,
};
use serde_json::json;

/// A worker reports that a kept-up permission card's call will not run.
#[test]
fn late_allow_not_run_parses() {
    let frame = json!({
        "type": "system",
        "subtype": "late_allow_not_run",
        "request_id": "req-9",
        "tool_use_id": "toolu_1",
        "reason": "declined",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert!(sys.is_late_allow_not_run());
    let Some(KnownSystemEvent::LateAllowNotRun(msg)) = sys.as_known_system_event() else {
        panic!("expected LateAllowNotRun");
    };
    assert_eq!(msg.request_id, "req-9");
    assert_eq!(msg.tool_use_id, "toolu_1");
    assert_eq!(msg.reason, LateAllowNotRunReason::Declined);
}

/// An unfamiliar reason still decodes and round-trips.
#[test]
fn late_allow_not_run_unknown_reason_round_trips() {
    let frame = json!({
        "type": "system",
        "subtype": "late_allow_not_run",
        "request_id": "req-9",
        "tool_use_id": "toolu_1",
        "reason": "some_future_reason",
        "uuid": "u1",
        "session_id": "s1"
    });
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let msg = sys
        .as_late_allow_not_run()
        .expect("typed late_allow_not_run");
    assert_eq!(
        msg.reason,
        LateAllowNotRunReason::Unknown("some_future_reason".into())
    );
    assert_eq!(
        serde_json::to_value(&msg).unwrap()["reason"],
        "some_future_reason"
    );
}

/// The result of a cloud turn reports how its memory check went.
#[test]
fn result_carries_memory_check_detail() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 10,
        "duration_api_ms": 5,
        "num_turns": 1,
        "result": "done",
        "stop_reason": "end_turn",
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.0,
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "memory_check_detail": {
            "flag": 2,
            "hold": "sent",
            "wait": "answered",
            "requests": 3
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    let mc = res
        .memory_check_detail
        .as_ref()
        .expect("memory_check_detail");
    assert_eq!(mc.flag, Some(2));
    assert_eq!(mc.hold, MemoryCheckHold::Sent);
    assert_eq!(mc.wait, Some(MemoryCheckWait::Answered));
    assert_eq!(mc.requests, Some(3));

    let wire = serde_json::to_value(&res).unwrap();
    assert_eq!(wire["memory_check_detail"]["hold"], "sent");
}

/// Only `hold` is required; unknown values pass through.
#[test]
fn memory_check_detail_minimal_and_unknown() {
    let detail: claude_codes::MemoryCheckDetail =
        serde_json::from_value(json!({"hold": "brand_new"})).unwrap();
    assert_eq!(detail.hold, MemoryCheckHold::Unknown("brand_new".into()));
    assert_eq!(detail.wait, None);
    assert_eq!(
        serde_json::to_value(&detail).unwrap(),
        json!({"hold": "brand_new"})
    );
}

/// A desktop SessionStart hook carries who configured it.
#[test]
fn hook_started_carries_origin() {
    let frame = json!({
        "type": "system",
        "subtype": "hook_started",
        "hook_id": "h1",
        "hook_name": "SessionStart:startup",
        "hook_event": "SessionStart",
        "origin": "official_plugin",
        "label": "Loading context",
        "plugin_name": "context-loader",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::HookStarted(hook)) = sys.as_known_system_event() else {
        panic!("expected HookStarted");
    };
    assert_eq!(hook.origin.as_deref(), Some("official_plugin"));
    assert_eq!(hook.label.as_deref(), Some("Loading context"));
    assert_eq!(hook.plugin_name.as_deref(), Some("context-loader"));
}

/// A hook that went to the background reports it once, with no output.
#[test]
fn hook_progress_carries_backgrounded() {
    let frame = json!({
        "type": "system",
        "subtype": "hook_progress",
        "hook_id": "h1",
        "hook_name": "SessionStart:startup",
        "hook_event": "SessionStart",
        "stdout": "",
        "stderr": "",
        "output": "",
        "backgrounded": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::HookProgress(hook)) = sys.as_known_system_event() else {
        panic!("expected HookProgress");
    };
    assert_eq!(hook.backgrounded, Some(true));
}

/// A task notification can mark its summary as the final report.
#[test]
fn task_notification_carries_summary_is_report() {
    let frame = json!({
        "type": "system",
        "subtype": "task_notification",
        "task_id": "t1",
        "status": "completed",
        "summary": "Found three call sites.",
        "summary_is_report": true,
        "output_file": "/tmp/t1.out",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::TaskNotification(task)) = sys.as_known_system_event() else {
        panic!("expected TaskNotification");
    };
    assert_eq!(task.summary_is_report, Some(true));
}

/// A user message can keep an unanswered permission card up behind it.
#[test]
fn user_carries_keeps_request_id() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": "actually, wait"}]
        },
        "parent_tool_use_id": null,
        "keeps_request_id": "req-9",
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(user.keeps_request_id.as_deref(), Some("req-9"));
    assert_eq!(
        serde_json::to_value(&user).unwrap()["keeps_request_id"],
        "req-9"
    );
}
