//! Coverage for the CLI 2.1.288 stream-json drift.
//!
//! 2.1.287 → 2.1.288 added the `system/instruction_size_warning` subtype,
//! `input_attachments_detail` and `flag_fetch_kick` on `result`, `tag` on
//! `system/informational`, the `carried_writes` capability marker on
//! `system/turn_handoff_available`, and the `media_removed` API error with
//! its `media` / `media_reason` parameters.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ApiErrorMedia, ApiErrorMediaReason, ClaudeOutput, KnownSystemEvent,
    SystemSubtype,
};
use serde_json::json;

/// The new subtype parses through both the accessor and the typed event view.
#[test]
fn instruction_size_warning_is_typed() {
    let frame = json!({
        "type": "system",
        "subtype": "instruction_size_warning",
        "total_chars": 52000,
        "total_limit_chars": 40000,
        "file_count": 3,
        "largest_chars": 41000,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::InstructionSizeWarning);
    assert!(sys.is_instruction_size_warning());
    let warning = sys.as_instruction_size_warning().expect("typed accessor");
    assert_eq!(warning.total_chars, 52000);
    assert_eq!(warning.total_limit_chars, 40000);
    assert_eq!(warning.file_count, 3);
    assert_eq!(warning.largest_chars, Some(41000));
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::InstructionSizeWarning(_))
    ));
}

/// `largest_chars` is absent when the warning is about a single file.
#[test]
fn instruction_size_warning_without_largest_chars() {
    let frame = json!({
        "type": "system",
        "subtype": "instruction_size_warning",
        "total_chars": 45000,
        "total_limit_chars": 40000,
        "file_count": 1,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let warning = sys.as_instruction_size_warning().expect("typed accessor");
    assert_eq!(warning.largest_chars, None);
}

/// `result` carries what the `input_attachments` phase did and how start-up
/// handled the feature-flag fetch.
#[test]
fn result_carries_input_attachments_detail_and_flag_fetch_kick() {
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
        "time_to_request_phases_ms": {"input_attachments": 30, "other": 10},
        "time_to_request_cpu_ms": 12,
        "time_to_request_major_faults": 1,
        "input_attachments_detail": {
            "slowest_producer": "changed_files",
            "slowest_producer_ms": 27,
            "cpu_ms": 4,
            "major_faults": 0,
            "changed_files_reread": 2
        },
        "time_to_request_from_spawn_ms": 900,
        "flag_fetch_kick": "started_in_init_empty"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.flag_fetch_kick.as_deref(),
        Some("started_in_init_empty")
    );
    let detail = res.input_attachments_detail.expect("detail");
    assert_eq!(detail.slowest_producer.as_deref(), Some("changed_files"));
    assert_eq!(detail.slowest_producer_ms, Some(27));
    assert_eq!(detail.cpu_ms, Some(4));
    assert_eq!(detail.major_faults, Some(0));
    assert_eq!(detail.changed_files_reread, Some(2));
}

/// The feature tag on an informational line is typed.
#[test]
fn informational_carries_tag() {
    let frame = json!({
        "type": "system",
        "subtype": "informational",
        "content": "Heads up",
        "level": "notice",
        "tag": "some_feature",
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
    assert_eq!(info.tag.as_deref(), Some("some_feature"));
}

/// The `carried_writes` capability marker is typed rather than landing in
/// `extra`.
#[test]
fn turn_handoff_available_carries_carried_writes() {
    let frame = json!({
        "type": "system",
        "subtype": "turn_handoff_available",
        "v": 1,
        "tools": ["Write"],
        "worker_epoch": 5,
        "carried_writes": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let handoff = sys.as_turn_handoff_available().expect("typed accessor");
    assert_eq!(handoff.carried_writes, Some(true));
    assert!(handoff.extra.is_empty());
}

/// A `media_removed` API error says which kind of block was refused and why.
#[test]
fn assistant_media_removed_params_fully_wrapped() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "The PDF was removed"}],
            "stop_reason": null,
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            }
        },
        "session_id": "s1",
        "uuid": "u1",
        "parent_tool_use_id": null,
        "is_api_error_message": true,
        "api_error_status": 400,
        "api_error": "media_removed",
        "api_error_params": {
            "media": "document",
            "media_reason": "unsupported_by_model"
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.api_error.as_deref(), Some("media_removed"));
    let params = msg.api_error_params.expect("api_error_params");
    assert_eq!(params.media, Some(ApiErrorMedia::Document));
    assert_eq!(
        params.media_reason,
        Some(ApiErrorMediaReason::UnsupportedByModel)
    );
}

/// Unrecognized media kinds and reasons are kept, not rejected.
#[test]
fn api_error_media_enums_are_open_sets() {
    let media: ApiErrorMedia = serde_json::from_value(json!("video")).unwrap();
    assert_eq!(media, ApiErrorMedia::Unknown("video".into()));
    assert_eq!(media.to_string(), "video");
    assert_eq!(
        serde_json::to_value(ApiErrorMedia::Image).unwrap(),
        json!("image")
    );

    let reason: ApiErrorMediaReason = serde_json::from_value(json!("future_reason")).unwrap();
    assert_eq!(reason, ApiErrorMediaReason::Unknown("future_reason".into()));
    assert_eq!(
        serde_json::to_value(ApiErrorMediaReason::MediaBudget).unwrap(),
        json!("media_budget")
    );
}
