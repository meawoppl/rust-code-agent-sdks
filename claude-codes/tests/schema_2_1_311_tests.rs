//! Coverage for the CLI 2.1.311 stream-json drift.
//!
//! 2.1.310 → 2.1.311 added one system subtype (`inherited_asks_settled`),
//! streaming and process-start telemetry on `result`,
//! `requires_rewind_request_id` on `system/turn_handoff_available`,
//! `background_color` on each `system/ui_panes` pane, and two `api_error`
//! kinds. No removals.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, GarbageCollectionCounts, KnownSystemEvent, SystemMessage,
};
use serde_json::{json, Value};

fn system(frame: Value) -> SystemMessage {
    assert_fully_wrapped(&frame);
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    sys
}

/// A cloud worker names the inherited asks it keeps answerable.
#[test]
fn inherited_asks_settled_lists_kept_request_ids() {
    let sys = system(json!({
        "type": "system",
        "subtype": "inherited_asks_settled",
        "kept_request_ids": ["req-1", "req-7"],
        "uuid": "u1",
        "session_id": "s1"
    }));
    assert!(sys.is_inherited_asks_settled());
    let settled = sys.as_inherited_asks_settled().expect("typed accessor");
    assert_eq!(settled.kept_request_ids, vec!["req-1", "req-7"]);
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::InheritedAsksSettled(_))
    ));
}

/// A success result carries the streaming windows, start-up tuning bits and
/// garbage-collection counts.
#[test]
fn success_result_carries_streaming_and_process_start_telemetry() {
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
        "process_wall_ms_streaming": 3100,
        "process_cpu_ms_streaming": 420,
        "main_stream_count": 2,
        "main_stream_event_count": 180,
        "main_stream_chunk_count": 95,
        "process_start_tuning": 9,
        "process_start_purge_delay_ms": 250,
        "process_garbage_collections": {
            "to_first_message": {"eden": 3, "full": 1, "wall_ms": 12, "heap_after_mib": 48},
            "to_first_result": {"eden": 7, "full": 2, "wall_ms": 30}
        },
        "permission_denials": [],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.process_wall_ms_streaming, Some(3100));
    assert_eq!(res.process_cpu_ms_streaming, Some(420));
    assert_eq!(res.main_stream_count, Some(2));
    assert_eq!(res.main_stream_event_count, Some(180));
    assert_eq!(res.main_stream_chunk_count, Some(95));
    assert_eq!(res.process_start_tuning, Some(9));
    assert_eq!(res.process_start_purge_delay_ms, Some(250));
    let gc = res.process_garbage_collections.unwrap();
    assert_eq!(gc.to_first_message.unwrap().heap_after_mib, Some(48));
    assert_eq!(
        gc.to_first_result,
        GarbageCollectionCounts {
            eden: 7,
            full: 2,
            wall_ms: 30,
            heap_after_mib: None,
        }
    );
}

/// An error result carries the process-start fields too: the CLI stamps them
/// on results of any subtype.
#[test]
fn error_result_carries_process_start_tuning() {
    let frame = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "duration_ms": 10,
        "duration_api_ms": 0,
        "num_turns": 0,
        "stop_reason": null,
        "total_cost_usd": 0.0,
        "process_start_tuning": 1,
        "process_garbage_collections": {
            "to_first_result": {"eden": 1, "full": 0, "wall_ms": 2}
        },
        "permission_denials": [],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.process_start_tuning, Some(1));
    assert_eq!(res.process_start_purge_delay_ms, None);
    assert!(res
        .process_garbage_collections
        .unwrap()
        .to_first_message
        .is_none());
}

/// A handoff-capable worker says it honours `requires_rewind_request_id`.
#[test]
fn turn_handoff_available_carries_requires_rewind_request_id() {
    let sys = system(json!({
        "type": "system",
        "subtype": "turn_handoff_available",
        "v": 1,
        "tools": ["Write"],
        "worker_epoch": 9,
        "requires_rewind_request_id": true,
        "uuid": "u1",
        "session_id": "s1"
    }));
    let handoff = sys.as_turn_handoff_available().expect("typed accessor");
    assert_eq!(handoff.requires_rewind_request_id, Some(true));
    assert!(handoff.extra.is_empty());
}

/// Each `system/ui_panes` pane may name the background painted under it.
#[test]
fn ui_panes_carry_background_color() {
    let sys = system(json!({
        "type": "system",
        "subtype": "ui_panes",
        "panes": [
            {
                "id": "deploy-log",
                "title": "Deploy log",
                "plugin": "deploy-helper",
                "background_color": "transparent"
            },
            {"id": "notes", "title": "notes", "plugin": "scratch"}
        ],
        "shown_id": "deploy-log",
        "focused_id": null,
        "focus_requested_id": null,
        "uuid": "u1",
        "session_id": "s1"
    }));
    let roster = sys.as_ui_panes().expect("typed accessor");
    assert_eq!(
        roster.panes[0].background_color.as_deref(),
        Some("transparent")
    );
    assert_eq!(roster.panes[1].background_color, None);
}

/// The two new `api_error` kinds arrive as plain strings on the assistant
/// frame.
#[test]
fn assistant_carries_unattended_start_api_error_kinds() {
    for kind in ["unattended_start_limit", "unattended_start_unconfirmed"] {
        let frame = json!({
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "type": "message",
                "role": "assistant",
                "model": "<synthetic>",
                "content": [{"type": "text", "text": "Not started."}],
                "stop_reason": "stop_sequence",
                "stop_sequence": "",
                "usage": {"input_tokens": 0, "output_tokens": 0}
            },
            "parent_tool_use_id": null,
            "is_api_error_message": true,
            "api_error": kind,
            "uuid": "u1",
            "session_id": "s1"
        });
        assert_fully_wrapped(&frame);
        let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
            panic!("expected Assistant");
        };
        assert_eq!(msg.api_error.as_deref(), Some(kind));
    }
}
