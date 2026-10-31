//! Coverage for the CLI 2.1.316 stream-json drift.
//!
//! 2.1.314 → 2.1.316 (2.1.315 shipped in between) added remote text-upload
//! pacing telemetry on `result` (`text_post_count`,
//! `text_post_gap_over_500ms_count`, `text_post_flush_timer_max_late_ms`),
//! per-pass timings and a `differs_files` size breakdown on
//! `input_attachments_detail`, four `time_to_request_phases_ms` phase labels,
//! and `ultrathink_available` on `system/init`. No new wire labels and no
//! removals.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput, DiffersFilesCounts};
use serde_json::json;

/// A success result carries the text-upload pacing figures, the new
/// attachment-pass timings and the four new request phases.
#[test]
fn success_result_carries_text_post_and_attachment_pass_telemetry() {
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
        "time_to_request_ms": 420,
        "time_to_request_phases_ms": {
            "conversation_append": 12,
            "init_frame_send": 3,
            "loop_entry": 7,
            "request_announcements": 25,
            "input_attachments": 60,
            "other": 4
        },
        "input_attachments_detail": {
            "slowest_producer": "changed_files",
            "slowest_producer_ms": 30,
            "touch_compare": {"same": 4, "differs": 3},
            "setup_ms": 2,
            "prompt_producers_ms": 5,
            "ambient_start_ms": 10,
            "ambient_wait_ms": 31,
            "render_ms": 8,
            "producers": 22,
            "token_counts": 1,
            "token_count_ms": 140,
            "changed_files_read_ms": 18,
            "differs_files": {
                "size_under_25_kb": 1,
                "size_25_to_50_kb": 1,
                "size_50_to_256_kb": 1,
                "hash_only": 1
            }
        },
        "first_text_post_ms": 900,
        "text_post_count": 14,
        "text_post_gap_over_500ms_count": 2,
        "text_post_flush_timer_max_late_ms": 37,
        "permission_denials": [],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.text_post_count, Some(14));
    assert_eq!(res.text_post_gap_over_500ms_count, Some(2));
    assert_eq!(res.text_post_flush_timer_max_late_ms, Some(37));

    let phases = res.time_to_request_phases_ms.as_ref().expect("phases");
    assert_eq!(phases.get("conversation_append"), Some(&12));
    assert_eq!(phases.get("request_announcements"), Some(&25));

    let detail = res.input_attachments_detail.as_ref().expect("detail");
    assert_eq!(detail.setup_ms, Some(2));
    assert_eq!(detail.ambient_wait_ms, Some(31));
    assert_eq!(detail.producers, Some(22));
    assert_eq!(detail.token_count_ms, Some(140));
    assert_eq!(detail.changed_files_read_ms, Some(18));
    assert_eq!(
        detail.differs_files,
        Some(DiffersFilesCounts {
            size_under_25_kb: Some(1),
            size_25_to_50_kb: Some(1),
            size_50_to_256_kb: Some(1),
            size_over_256_kb: None,
            hash_only: Some(1),
        })
    );

    let wire = serde_json::to_value(&res).unwrap();
    assert!(wire["input_attachments_detail"]["differs_files"]
        .get("size_over_256_kb")
        .is_none());
}

/// A pass that made no token counts reports `token_counts: 0` and omits
/// `token_count_ms` and `differs_files`; the text-post fields stay off the
/// wire when absent.
#[test]
fn attachment_pass_without_token_counts_omits_optional_members() {
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
        "input_attachments_detail": {"producers": 20, "token_counts": 0},
        "permission_denials": [],
        "uuid": "u2",
        "session_id": "s1"
    });
    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    let detail = res.input_attachments_detail.as_ref().expect("detail");
    assert_eq!(detail.token_counts, Some(0));
    assert_eq!(detail.token_count_ms, None);
    assert_eq!(detail.differs_files, None);
    assert_eq!(res.text_post_count, None);

    let wire = serde_json::to_value(&res).unwrap();
    assert!(wire.get("text_post_count").is_none());
    assert!(wire["input_attachments_detail"]
        .get("token_count_ms")
        .is_none());
    assert!(wire["input_attachments_detail"]
        .get("differs_files")
        .is_none());
}

/// `system/init` carries `ultrathink_available`.
#[test]
fn system_init_carries_ultrathink_available() {
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "per_turn_effort_active": true,
        "ultrathink_available": true
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    assert_eq!(init.ultrathink_available, Some(true));
}
