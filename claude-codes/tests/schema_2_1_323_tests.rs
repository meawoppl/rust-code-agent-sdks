//! Coverage for the CLI 2.1.323 stream-json drift.
//!
//! 2.1.322 → 2.1.323 added `policy_answer` / `policy_answer_reason` on the
//! result, `tool_pool_build_lapsed_count`, `tool_pool_moved_at_lapse_count`
//! and `tool_pool_limit_ms` on the success result, the
//! `managed_settings_not_delivered` startup failure reason, `owed_rows` on
//! `stream_event`, `retracted_message_uuids` on `user`, `off_host` on
//! `system/background_tasks_changed` entries and `placement` on
//! `system/ui_panes` panes. It removed `via_outputs_link` from
//! `input_attachments_detail.touch_compare`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, StartupFailureReason};
use serde_json::json;

/// The success result reports tool-list builds forced by the age limit,
/// the tools they moved, and the limit itself.
#[test]
fn result_success_tool_pool_lapse_fields() {
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
        "process_cpu_ms_turn": 140,
        "tool_pool_build_count": 3,
        "tool_pool_build_lapsed_count": 1,
        "tool_pool_moved_at_lapse_count": 0,
        "tool_pool_limit_ms": 300000
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.tool_pool_build_count, Some(3));
    assert_eq!(res.tool_pool_build_lapsed_count, Some(1));
    assert_eq!(res.tool_pool_moved_at_lapse_count, Some(0));
    assert_eq!(res.tool_pool_limit_ms, Some(300000));

    let wire = serde_json::to_value(&res).unwrap();
    assert_eq!(wire["tool_pool_build_lapsed_count"], 1);
    assert_eq!(wire["tool_pool_moved_at_lapse_count"], 0);
    assert_eq!(wire["tool_pool_limit_ms"], 300000);
}

/// The error result written when the organizations load gate refuses a
/// message carries the policy answer and its reason.
#[test]
fn result_error_carries_policy_answer() {
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
        "errors": ["Your organization's policy limits could not be loaded."],
        "terminal_reason": "turn_setup_failed",
        "policy_answer": "unavailable",
        "policy_answer_reason": "network"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.policy_answer.as_deref(), Some("unavailable"));
    assert_eq!(res.policy_answer_reason.as_deref(), Some("network"));
}

/// `managed_settings_not_delivered` is a typed startup failure reason.
#[test]
fn startup_failure_reason_managed_settings_not_delivered() {
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
        "errors": ["The initialize request has no managedSettings object."],
        "startup_failure_reason": "managed_settings_not_delivered"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::ManagedSettingsNotDelivered)
    );
    assert_eq!(
        StartupFailureReason::ManagedSettingsNotDelivered.to_string(),
        "managed_settings_not_delivered"
    );
    assert_eq!(
        StartupFailureReason::from("managed_settings_not_delivered"),
        StartupFailureReason::ManagedSettingsNotDelivered
    );
}

/// The turn's first reply event lists the Project document rows it owes;
/// an empty list is kept distinct from absent.
#[test]
fn stream_event_carries_owed_rows() {
    let frame = json!({
        "type": "stream_event",
        "event": {"type": "message_start", "message": {"id": "msg_01"}},
        "parent_tool_use_id": null,
        "user_message_uuid": "um-1",
        "user_message_uuids": ["um-1"],
        "owed_rows": ["row-1", "row-2"],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::StreamEvent(ev) = serde_json::from_value(frame).unwrap() else {
        panic!("expected StreamEvent");
    };
    assert_eq!(
        ev.owed_rows.as_deref(),
        Some(&["row-1".to_string(), "row-2".to_string()][..])
    );

    let frame = json!({
        "type": "stream_event",
        "event": {"type": "message_start", "message": {"id": "msg_02"}},
        "parent_tool_use_id": null,
        "user_message_uuid": "um-2",
        "user_message_uuids": ["um-2"],
        "owed_rows": [],
        "uuid": "u2",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);
    let ClaudeOutput::StreamEvent(ev) = serde_json::from_value(frame).unwrap() else {
        panic!("expected StreamEvent");
    };
    assert_eq!(ev.owed_rows, Some(vec![]));
    let wire = serde_json::to_value(&ev).unwrap();
    assert_eq!(wire["owed_rows"], json!([]));
}

/// The synthetic re-ask after an unparsable tool call names the dropped
/// messages in `retracted_message_uuids`.
#[test]
fn user_carries_retracted_message_uuids() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": "Your tool call could not be parsed."}]
        },
        "parent_tool_use_id": null,
        "isSynthetic": true,
        "retracted_message_uuids": ["a1", "a2"],
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(
        user.retracted_message_uuids,
        Some(vec!["a1".to_string(), "a2".to_string()])
    );
}

/// `background_tasks_changed` entries flag off-host work.
#[test]
fn background_tasks_changed_entry_off_host() {
    let frame = json!({
        "type": "system",
        "subtype": "background_tasks_changed",
        "tasks": [
            {
                "task_id": "t1",
                "task_type": "remote_agent",
                "description": "cloud run",
                "off_host": true
            },
            {
                "task_id": "t2",
                "task_type": "local_bash",
                "description": "sleep 30"
            }
        ],
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
    assert_eq!(changed.tasks[0].off_host, Some(true));
    assert_eq!(changed.tasks[1].off_host, None);
}

/// A `system/ui_panes` pane may ask to start as a widget.
#[test]
fn ui_panes_pane_placement_widget() {
    let frame = json!({
        "type": "system",
        "subtype": "ui_panes",
        "panes": [
            {"id": "clock", "title": "Clock", "plugin": "clock", "placement": "widget"},
            {"id": "notes", "title": "notes", "plugin": "scratch"}
        ],
        "shown_id": "clock",
        "focused_id": null,
        "focus_requested_id": null,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::UiPanes(panes)) = sys.as_known_system_event() else {
        panic!("expected UiPanes");
    };
    assert_eq!(panes.panes[0].placement.as_deref(), Some("widget"));
    assert_eq!(panes.panes[1].placement, None);
}
