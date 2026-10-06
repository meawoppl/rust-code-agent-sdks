//! Coverage for the CLI 2.1.292 stream-json drift.
//!
//! 2.1.291 → 2.1.292 added the `system/permission_check_status` subtype,
//! `agent_id` and the `usage_limit_reached` API error (with
//! `api_error_params.rate_limit_info`) on `assistant`, `safety_stops` on
//! `result`, `abandoned_blocks` on `stream_event`, `run_id` on every
//! `task_*` frame and `background_tasks_changed` entry, `parent_task_id` on
//! `task_started` and `background_tasks_changed` entries, and `handback` /
//! `handback_report` on `task_notification`. The live wrapping audit also
//! found `agent_id` on the prompt echoed into a subagent (`user` frames),
//! which the published user schema does not list.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, PermissionCheckStatus, RateLimitScope,
    RateLimitStatus, RateLimitWindow, SystemSubtype, TaskHandback,
};
use serde_json::json;

/// The new subtype parses through both the accessor and the typed event view.
#[test]
fn permission_check_status_is_typed() {
    let frame = json!({
        "type": "system",
        "subtype": "permission_check_status",
        "tool_use_id": "toolu_01",
        "agent_id": "a1",
        "status": "checking",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::PermissionCheckStatus);
    assert!(sys.is_permission_check_status());
    let check = sys.as_permission_check_status().expect("typed accessor");
    assert_eq!(check.tool_use_id, "toolu_01");
    assert_eq!(check.agent_id.as_deref(), Some("a1"));
    assert_eq!(check.status, PermissionCheckStatus::Checking);
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::PermissionCheckStatus(_))
    ));
}

/// `done` and unknown statuses round-trip.
#[test]
fn permission_check_status_values_round_trip() {
    for (wire, typed) in [
        ("checking", PermissionCheckStatus::Checking),
        ("done", PermissionCheckStatus::Done),
        (
            "some_future_status",
            PermissionCheckStatus::Unknown("some_future_status".into()),
        ),
    ] {
        let parsed: PermissionCheckStatus = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, typed);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// A subagent's assistant message names its agent, and a usage-limit refusal
/// carries the limit's record.
#[test]
fn assistant_carries_agent_id_and_usage_limit_info() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_01",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-5-5",
            "content": [{"type": "text", "text": "You've hit your limit."}],
            "stop_reason": "stop_sequence",
            "stop_sequence": "",
            "usage": {"input_tokens": 0, "output_tokens": 0}
        },
        "parent_tool_use_id": null,
        "agent_id": "a1",
        "error": "rate_limit",
        "is_api_error_message": true,
        "api_error": "usage_limit_reached",
        "api_error_params": {
            "rate_limit_info": {
                "status": "rejected",
                "resetsAt": 1760000000,
                "rateLimitType": "seven_day",
                "isUsingOverage": false,
                "limitScope": "group_pool"
            }
        },
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.agent_id.as_deref(), Some("a1"));
    assert_eq!(msg.api_error.as_deref(), Some("usage_limit_reached"));
    let info = msg
        .api_error_params
        .and_then(|p| p.rate_limit_info)
        .expect("rate_limit_info");
    assert_eq!(info.status, RateLimitStatus::Rejected);
    assert_eq!(info.rate_limit_type, Some(RateLimitWindow::SevenDay));
    assert_eq!(info.resets_at, Some(1760000000));
    assert_eq!(info.limit_scope, Some(RateLimitScope::GroupPool));
}

/// The prompt echoed into a subagent names the subagent. Not in the
/// published user schema; captured from a live 2.1.292 session.
#[test]
fn user_carries_agent_id() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": "Compute 6 times 7."}]
        },
        "parent_tool_use_id": "toolu_01",
        "agent_id": "ab0fdef6f23603198",
        "subagent_type": "general-purpose",
        "task_description": "Compute 6 times 7",
        "timestamp": "2026-10-06T20:30:02.748Z",
        "uuid": "fa290d23-e2b8-4663-a67c-0f73bd952bdc",
        "session_id": "01a59841-f4cd-468a-96d4-53a255cac787"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(user.agent_id.as_deref(), Some("ab0fdef6f23603198"));
}

/// `limitScope` values, known and unknown, round-trip.
#[test]
fn rate_limit_scope_round_trips() {
    for (wire, typed) in [
        ("service", RateLimitScope::Service),
        ("channel", RateLimitScope::Channel),
        ("group_pool", RateLimitScope::GroupPool),
        ("team", RateLimitScope::Unknown("team".into())),
    ] {
        let parsed: RateLimitScope = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, typed);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// `result` counts safety stops.
#[test]
fn result_carries_safety_stops() {
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
        "safety_stops": 2
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.safety_stops, Some(2));
}

/// A CLI-produced `message_stop` names the blocks that will never get an
/// assistant message.
#[test]
fn stream_event_carries_abandoned_blocks() {
    let frame = json!({
        "type": "stream_event",
        "event": {"type": "message_stop"},
        "parent_tool_use_id": null,
        "abandoned_blocks": {"api_message_id": "msg_01", "from_block_index": 1},
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::StreamEvent(ev) = serde_json::from_value(frame).unwrap() else {
        panic!("expected StreamEvent");
    };
    let abandoned = ev.abandoned_blocks.expect("abandoned_blocks");
    assert_eq!(abandoned.api_message_id, "msg_01");
    assert_eq!(abandoned.from_block_index, 1);
}

/// `task_started` carries its run and parent task.
#[test]
fn task_started_carries_run_and_parent() {
    let frame = json!({
        "type": "system",
        "subtype": "task_started",
        "task_id": "t2",
        "run_id": "r1",
        "parent_task_id": "t1",
        "description": "nested agent",
        "task_type": "local_agent",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let started = sys.as_task_started().expect("task_started");
    assert_eq!(started.run_id.as_deref(), Some("r1"));
    assert_eq!(started.parent_task_id.as_deref(), Some("t1"));
}

/// `task_updated` and `task_progress` carry the run id.
#[test]
fn task_updated_and_progress_carry_run_id() {
    let updated = json!({
        "type": "system",
        "subtype": "task_updated",
        "task_id": "t1",
        "run_id": "r1",
        "patch": {"status": "running"},
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&updated);
    let ClaudeOutput::System(sys) = serde_json::from_value(updated).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(
        sys.as_task_updated()
            .expect("task_updated")
            .run_id
            .as_deref(),
        Some("r1")
    );

    let progress = json!({
        "type": "system",
        "subtype": "task_progress",
        "task_id": "t1",
        "run_id": "r1",
        "description": "working",
        "usage": {"total_tokens": 10, "tool_uses": 1, "duration_ms": 100},
        "uuid": "u2",
        "session_id": "s1"
    });
    assert_fully_wrapped(&progress);
    let ClaudeOutput::System(sys) = serde_json::from_value(progress).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(
        sys.as_task_progress()
            .expect("task_progress")
            .run_id
            .as_deref(),
        Some("r1")
    );
}

/// A subagent hand-back delivers its report on `task_notification`.
#[test]
fn task_notification_carries_handback_report() {
    let frame = json!({
        "type": "system",
        "subtype": "task_notification",
        "task_id": "t1",
        "run_id": "r1",
        "status": "completed",
        "output_file": "",
        "summary": "harness notes",
        "handback": "flagged",
        "handback_report": {"text": "the report", "warning": "SECURITY WARNING"},
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let note = sys.as_task_notification().expect("task_notification");
    assert_eq!(note.run_id.as_deref(), Some("r1"));
    assert_eq!(note.handback, Some(TaskHandback::Flagged));
    let report = note.handback_report.expect("handback_report");
    assert_eq!(report.text, "the report");
    assert_eq!(report.warning.as_deref(), Some("SECURITY WARNING"));

    let withheld: TaskHandback = serde_json::from_value(json!("withheld")).unwrap();
    assert_eq!(withheld, TaskHandback::Withheld);
    let future: TaskHandback = serde_json::from_value(json!("later")).unwrap();
    assert_eq!(future, TaskHandback::Unknown("later".into()));
    assert_eq!(serde_json::to_value(&future).unwrap(), json!("later"));
}

/// `background_tasks_changed` entries carry run and parent ids.
#[test]
fn background_tasks_changed_entries_carry_run_and_parent() {
    let frame = json!({
        "type": "system",
        "subtype": "background_tasks_changed",
        "tasks": [{
            "task_id": "t2",
            "run_id": "r1",
            "task_type": "local_agent",
            "description": "nested agent",
            "parent_task_id": "t1"
        }],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::BackgroundTasksChanged(changed)) = sys.as_known_system_event()
    else {
        panic!("expected BackgroundTasksChanged");
    };
    assert_eq!(changed.tasks[0].run_id.as_deref(), Some("r1"));
    assert_eq!(changed.tasks[0].parent_task_id.as_deref(), Some("t1"));
}
