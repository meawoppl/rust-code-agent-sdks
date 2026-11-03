//! Coverage for the CLI 2.1.317 stream-json drift.
//!
//! 2.1.316 → 2.1.317 added the `system/session_writes_lost` subtype; the
//! `tool_schema_build_detail`, `message_normalization_detail`,
//! `request_build_detail` and `process_turn_states` timing breakdowns on
//! `result`; `client_thinking`, `flag_restate_noop`, `synced_skills_path`
//! and `ultrathink_typed_raises` on `system/init`; `model_restriction` on
//! `system/informational`; `api_refusal_cyber_verification` on
//! `system/model_refusal_fallback`; `plugin_prompt`, `ultrathink_typed` and
//! `tool_states` on `user`; `built_in_tool` on `tool_result_meta` entries;
//! the `initialize_not_first` startup failure reason; and the
//! `malformed_tool_use_exhausted` / `network_edge_blocked` `api_error`
//! kinds. No removals.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeInput, ClaudeOutput, KnownSystemEvent, LostSessionWrite,
    ProcessStateTime, SessionWriteErrorCode, StartupFailureReason, SystemSubtype,
};
use serde_json::json;

fn system(frame: serde_json::Value) -> claude_codes::SystemMessage {
    assert_fully_wrapped(&frame);
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    sys
}

/// `system/session_writes_lost` is a typed subtype with open enums.
#[test]
fn system_session_writes_lost_fully_wrapped() {
    let sys = system(json!({
        "type": "system",
        "subtype": "session_writes_lost",
        "error_code": "ENOSPC",
        "lost": ["transcript", "global_config"],
        "transcript_session_ids": ["s0", "s1"],
        "uuid": "u1",
        "session_id": "s1"
    }));
    assert_eq!(sys.subtype, SystemSubtype::SessionWritesLost);
    assert!(sys.is_session_writes_lost());
    let lost = sys.as_session_writes_lost().expect("typed accessor");
    assert_eq!(lost.error_code, SessionWriteErrorCode::Enospc);
    assert_eq!(
        lost.lost,
        vec![LostSessionWrite::Transcript, LostSessionWrite::GlobalConfig]
    );
    assert_eq!(
        lost.transcript_session_ids.as_deref(),
        Some(&["s0".to_string(), "s1".to_string()][..])
    );
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::SessionWritesLost(_))
    ));

    let sys = system(json!({
        "type": "system",
        "subtype": "session_writes_lost",
        "error_code": "EROFS",
        "lost": ["global_config", "settings"],
        "uuid": "u2",
        "session_id": "s1"
    }));
    let lost = sys.as_session_writes_lost().expect("typed accessor");
    assert_eq!(
        lost.error_code,
        SessionWriteErrorCode::Unknown("EROFS".into())
    );
    assert_eq!(lost.lost[1], LostSessionWrite::Unknown("settings".into()));
    let wire = serde_json::to_value(&lost).unwrap();
    assert_eq!(wire["error_code"], "EROFS");
    assert!(wire.get("transcript_session_ids").is_none());
}

/// A success result carries the four new timing breakdowns.
#[test]
fn success_result_carries_request_phase_details_and_turn_states() {
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
        "time_to_request_phases_ms": {
            "tool_schema_build": 40,
            "message_normalization": 12,
            "request_build": 30
        },
        "tool_schema_build_detail": {
            "betas_ms": 1,
            "waits_ms": 10,
            "placement_ms": 4,
            "render_ms": 22,
            "after_ms": 3,
            "other_ms": 0,
            "tools": 31,
            "new_schemas": 29
        },
        "message_normalization_detail": {
            "before_ms": 2,
            "normalize_ms": 6,
            "images_ms": 0,
            "post_process_ms": 3,
            "after_ms": 1,
            "other_ms": 0,
            "messages": 8,
            "wire_messages": 6
        },
        "request_build_detail": {
            "records_ms": 2,
            "system_ms": 5,
            "headers_ms": 4,
            "cache_break_record_ms": 6,
            "plans_ms": 1,
            "retry_setup_ms": 1,
            "sent_build_ms": 9,
            "record_attempt_ms": 1,
            "other_ms": 1,
            "sent_build_safety_context_ms": 0,
            "sent_build_cache_breakpoints_ms": 4,
            "sent_build_surrogate_scan_ms": 2
        },
        "process_turn_states": {
            "main_stream": {"cpu_ms": 120, "wall_ms": 3800, "entries": 2},
            "tool": {"cpu_ms": 40, "wall_ms": 600, "entries": 1},
            "first_event_wait": {"cpu_ms": 5, "wall_ms": 400, "entries": 2},
            "before_first_request": {"cpu_ms": 80, "wall_ms": 150, "entries": 1},
            "between_requests": {"cpu_ms": 10, "wall_ms": 30, "entries": 1},
            "after_last_request": {"cpu_ms": 3, "wall_ms": 20, "entries": 1}
        },
        "permission_denials": [],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    let tsb = res.tool_schema_build_detail.as_ref().expect("tsb");
    assert_eq!(tsb.render_ms, 22);
    assert_eq!(tsb.new_schemas, Some(29));
    let mn = res.message_normalization_detail.as_ref().expect("mn");
    assert_eq!(mn.normalize_ms, 6);
    assert_eq!(mn.wire_messages, Some(6));
    let rb = res.request_build_detail.as_ref().expect("rb");
    assert_eq!(rb.cache_break_record_ms, 6);
    assert_eq!(rb.sent_build_surrogate_scan_ms, Some(2));
    let states = res.process_turn_states.as_ref().expect("states");
    assert_eq!(
        states.main_stream,
        Some(ProcessStateTime {
            cpu_ms: 120,
            wall_ms: 3800,
            entries: 2
        })
    );
    assert_eq!(states.side_stream, None);
    assert_eq!(states.agent_tool, None);

    let wire = serde_json::to_value(&res).unwrap();
    assert!(wire["process_turn_states"].get("side_stream").is_none());
}

/// `initialize_not_first` is a typed startup failure reason.
#[test]
fn startup_failure_reason_initialize_not_first() {
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
        "errors": ["The first stdin line was not the initialize control request."],
        "startup_failure_reason": "initialize_not_first"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::InitializeNotFirst)
    );
    assert_eq!(
        StartupFailureReason::InitializeNotFirst.to_string(),
        "initialize_not_first"
    );
}

/// `system/init` carries the four new fields; `client_thinking` keeps its
/// nullable members as wire `null`.
#[test]
fn system_init_carries_new_fields() {
    let sys = system(json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "ultrathink_available": true,
        "ultrathink_typed_raises": true,
        "client_thinking": {"max_thinking_tokens": null, "thinking_display": "summarized"},
        "flag_restate_noop": true,
        "synced_skills_path": "/srv/skills"
    }));
    let init = sys.as_init().expect("typed init");
    assert_eq!(init.ultrathink_typed_raises, Some(true));
    assert_eq!(init.flag_restate_noop, Some(true));
    assert_eq!(init.synced_skills_path.as_deref(), Some("/srv/skills"));
    let thinking = init.client_thinking.as_ref().expect("client_thinking");
    assert_eq!(thinking.max_thinking_tokens, None);
    assert_eq!(thinking.thinking_display.as_deref(), Some("summarized"));

    let wire = serde_json::to_value(&init).unwrap();
    assert!(wire["client_thinking"]["max_thinking_tokens"].is_null());
}

/// `system/informational` carries `model_restriction`.
#[test]
fn informational_carries_model_restriction() {
    let sys = system(json!({
        "type": "system",
        "subtype": "informational",
        "content": "claude-opus-5-5 is not available; using claude-sonnet-5-5",
        "level": "warning",
        "model_restriction": {
            "requested_model": "claude-opus-5-5",
            "effective_model": "claude-sonnet-5-5",
            "at": "start"
        },
        "uuid": "u1",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::Informational(info)) = sys.as_known_system_event() else {
        panic!("expected Informational");
    };
    let restriction = info.model_restriction.expect("model_restriction");
    assert_eq!(restriction.requested_model, "claude-opus-5-5");
    assert_eq!(restriction.effective_model, "claude-sonnet-5-5");
    assert_eq!(restriction.at.as_deref(), Some("start"));
}

/// `system/model_refusal_fallback` carries the cyber verification status.
#[test]
fn model_refusal_fallback_carries_cyber_verification() {
    let sys = system(json!({
        "type": "system",
        "subtype": "model_refusal_fallback",
        "trigger": "refusal",
        "direction": "down",
        "scope": "session",
        "original_model": "claude-opus-5-5",
        "fallback_model": "claude-sonnet-5-5",
        "request_id": "req_1",
        "api_refusal_category": "cyber",
        "api_refusal_cyber_verification": {
            "type": "active",
            "tier": "standard",
            "reason": "verified"
        },
        "content": "Switched model",
        "uuid": "u1",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::ModelRefusalFallback(fallback)) = sys.as_known_system_event() else {
        panic!("expected ModelRefusalFallback");
    };
    let cvp = fallback
        .api_refusal_cyber_verification
        .expect("cyber verification");
    assert_eq!(cvp.verification_type, "active");
    assert_eq!(cvp.tier.as_deref(), Some("standard"));
    assert_eq!(cvp.reason.as_deref(), Some("verified"));
}

/// A plugin-sent `user` frame carries `plugin_prompt`, and its
/// `tool_result_meta` entries carry `built_in_tool`.
#[test]
fn user_carries_plugin_prompt_and_built_in_tool() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [
                {"type": "tool_result", "tool_use_id": "toolu_01", "content": "ok"}
            ]
        },
        "parent_tool_use_id": null,
        "tool_result_meta": [
            {"id": "toolu_01", "built_in_tool": "ExitPlanMode"}
        ],
        "plugin_prompt": {"name": "review", "text": "/review 12"},
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    let meta = user.tool_result_meta.as_ref().expect("tool_result_meta");
    assert_eq!(meta[0].built_in_tool.as_deref(), Some("ExitPlanMode"));
    let prompt = user.plugin_prompt.as_ref().expect("plugin_prompt");
    assert_eq!(prompt.name, "review");
    assert_eq!(prompt.text, "/review 12");
}

/// The input-side `ultrathink_typed` and `tool_states` round-trip, and stay
/// off the wire when unset.
#[test]
fn user_input_ultrathink_typed_and_tool_states_round_trip() {
    let mut input = ClaudeInput::user_message("ultrathink about this", uuid::Uuid::nil());
    let ClaudeInput::User(msg) = &mut input else {
        panic!("expected User");
    };
    let plain = serde_json::to_value(&*msg).unwrap();
    assert!(plain.get("ultrathink_typed").is_none());
    assert!(plain.get("tool_states").is_none());

    msg.ultrathink_typed = Some(true);
    msg.tool_states = Some(vec![json!({"view": "chart", "state": {"zoom": 2}})]);
    let wire = serde_json::to_value(&input).unwrap();
    assert_eq!(wire["ultrathink_typed"], true);
    assert_eq!(wire["tool_states"][0]["view"], "chart");

    let ClaudeInput::User(back) = serde_json::from_value(wire).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(back.ultrathink_typed, Some(true));
    assert_eq!(back.tool_states.as_ref().map(Vec::len), Some(1));
}

/// The new `api_error` kinds pass through on an assistant API-error frame.
#[test]
fn assistant_api_error_new_kinds() {
    for kind in ["malformed_tool_use_exhausted", "network_edge_blocked"] {
        let frame = json!({
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "type": "message",
                "role": "assistant",
                "model": "<synthetic>",
                "content": [{"type": "text", "text": "API Error"}],
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
            "api_error": kind
        });
        assert_fully_wrapped(&frame);

        let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
            panic!("expected Assistant");
        };
        assert_eq!(msg.api_error.as_deref(), Some(kind));
    }
}
