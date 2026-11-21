//! Coverage for the CLI 2.1.330 stream-json drift.
//!
//! 2.1.329 → 2.1.330 added `begun_message_ids` on the result, `saved_from`
//! on `system/informational`, `unhooked` on `system/ui_invalidate`,
//! `tool_use_id` on `system/background_tasks_changed` entries, and the
//! `named_profile_unusable` `api_error` kind.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput, ConnectorFilesSaved, KnownSystemEvent};
use serde_json::json;

/// The success result lists the API message ids that began to stream.
#[test]
fn result_success_carries_begun_message_ids() {
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
        "begun_message_ids": ["msg_01A", "msg_01B"]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.begun_message_ids,
        Some(vec!["msg_01A".to_string(), "msg_01B".to_string()])
    );
}

/// An error result carries `begun_message_ids` too, and it may be empty.
#[test]
fn result_error_carries_empty_begun_message_ids() {
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
        "errors": [],
        "begun_message_ids": []
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.begun_message_ids, Some(vec![]));
}

/// A `connector_files` line says how many files a connector saved.
#[test]
fn informational_carries_saved_from() {
    let frame = json!({
        "type": "system",
        "subtype": "informational",
        "content": "Saved report.pdf from Drive",
        "level": "info",
        "tag": "connector_files",
        "saved_from": {"files": 1, "file": "report.pdf", "app": "Drive"},
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::Informational(info)) = sys.as_known_system_event() else {
        panic!("expected Informational");
    };
    assert_eq!(
        info.saved_from,
        Some(ConnectorFilesSaved {
            files: 1,
            file: Some("report.pdf".into()),
            app: Some("Drive".into()),
        })
    );
}

/// With several files, only the count is required.
#[test]
fn informational_saved_from_count_only() {
    let frame = json!({
        "type": "system",
        "subtype": "informational",
        "content": "Saved 3 files",
        "level": "info",
        "tag": "connector_files",
        "saved_from": {"files": 3},
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::Informational(info)) = sys.as_known_system_event() else {
        panic!("expected Informational");
    };
    let saved = info.saved_from.expect("saved_from");
    assert_eq!(saved.files, 3);
    assert_eq!(saved.file, None);
    assert_eq!(saved.app, None);
}

/// A `ui_invalidate` push with no instances carries the `unhooked` list.
#[test]
fn ui_invalidate_carries_unhooked() {
    let frame = json!({
        "type": "system",
        "subtype": "ui_invalidate",
        "event": "ui.render",
        "unhooked": ["clock", "notes"],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let inv = sys.as_ui_invalidate().expect("typed accessor");
    assert!(inv.instances.is_none());
    assert_eq!(
        inv.unhooked,
        Some(vec!["clock".to_string(), "notes".to_string()])
    );
}

/// An off-host background task entry names its tool_use id.
#[test]
fn background_tasks_changed_entry_tool_use_id() {
    let frame = json!({
        "type": "system",
        "subtype": "background_tasks_changed",
        "tasks": [
            {
                "task_id": "t1",
                "task_type": "remote_agent",
                "description": "cloud run",
                "off_host": true,
                "tool_use_id": "toolu_01X"
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
    assert_eq!(changed.tasks[0].tool_use_id.as_deref(), Some("toolu_01X"));
    assert_eq!(changed.tasks[1].tool_use_id, None);
}

/// The new `named_profile_unusable` kind passes through `api_error`.
#[test]
fn assistant_api_error_named_profile_unusable() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "m1",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "ANTHROPIC_PROFILE names a signed-out profile."}],
            "stop_reason": "stop_sequence",
            "stop_sequence": "",
            "usage": {"input_tokens": 0, "output_tokens": 0}
        },
        "parent_tool_use_id": null,
        "is_api_error_message": true,
        "api_error": "named_profile_unusable",
        "uuid": "u1",
        "session_id": "s1"
    });

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.api_error.as_deref(), Some("named_profile_unusable"));
}
