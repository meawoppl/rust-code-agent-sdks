//! Coverage for the CLI 2.1.331 stream-json drift.
//!
//! 2.1.330 → 2.1.331 added `tool_hold_detail` on the success result,
//! `agent_note` on `system/informational`, `sandbox_enabled` on
//! `system/init`, `served_notice` on `system/model_refusal_fallback`,
//! `mcp_own_output` on `tool_result_meta` entries, `page_count` on a user
//! frame's untyped `file_attachments`, and the `sideload_flags_disabled`
//! startup failure reason.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, AgentNoteKind, ClaudeOutput, KnownSystemEvent, SandboxEnabled,
    StartupFailureReason, ToolHoldEnd,
};
use serde_json::json;

fn system(frame: serde_json::Value) -> claude_codes::SystemMessage {
    assert_fully_wrapped(&frame);
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    sys
}

/// The success result reports how long hooks held tool calls.
#[test]
fn result_success_carries_tool_hold_detail() {
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
        "time_to_request_ms": 40,
        "tool_hold_detail": {
            "first_result_ms": 820,
            "holds": [
                {"name": "repo-sync", "end": "well", "began_ms": -15, "held_ms": 300},
                {"name": "", "end": "open", "began_ms": 10, "held_ms": 90},
                {"name": "x", "end": "paused", "began_ms": 0, "held_ms": 1}
            ]
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame.clone()).unwrap() else {
        panic!("expected Result");
    };
    let detail = res.tool_hold_detail.as_ref().expect("tool_hold_detail");
    assert_eq!(detail.first_result_ms, Some(820));
    assert_eq!(detail.holds[0].end, ToolHoldEnd::Well);
    assert_eq!(detail.holds[0].began_ms, -15);
    assert_eq!(detail.holds[1].end, ToolHoldEnd::Open);
    assert_eq!(detail.holds[2].end, ToolHoldEnd::Unknown("paused".into()));

    let round = serde_json::to_value(&res).unwrap();
    assert_eq!(round["tool_hold_detail"], frame["tool_hold_detail"]);
}

/// A result with no hold since the last one carries an empty list.
#[test]
fn result_tool_hold_detail_empty_holds() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 1,
        "duration_api_ms": 1,
        "num_turns": 1,
        "result": "ok",
        "stop_reason": "end_turn",
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.0,
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "tool_hold_detail": {"holds": []}
    });
    assert_fully_wrapped(&frame);
    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    let detail = res.tool_hold_detail.expect("tool_hold_detail");
    assert_eq!(detail.first_result_ms, None);
    assert!(detail.holds.is_empty());
}

/// `system/informational` carries a plugin's note for an agent.
#[test]
fn informational_carries_agent_note() {
    let sys = system(json!({
        "type": "system",
        "subtype": "informational",
        "content": "Left a note for the main conversation.",
        "level": "info",
        "agent_note": {
            "to": {"kind": "main"},
            "about": {
                "kind": "agent",
                "description": "Refactor the parser",
                "tool_use_id": "toolu_9"
            },
            "text": "The parser change misses the tests.",
            "source": "reviewer@builtin"
        },
        "uuid": "u1",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::Informational(info)) = sys.as_known_system_event() else {
        panic!("expected Informational");
    };
    let note = info.agent_note.as_ref().expect("agent_note");
    assert_eq!(note.to.kind, AgentNoteKind::Main);
    assert_eq!(note.to.description, None);
    assert_eq!(note.about.kind, AgentNoteKind::Agent);
    assert_eq!(note.about.tool_use_id.as_deref(), Some("toolu_9"));
    assert_eq!(note.source, "reviewer@builtin");
    assert_eq!(AgentNoteKind::from("session"), AgentNoteKind::Session);
}

/// Upstream catches a malformed `agent_note` and reads it as absent; so does
/// the crate, instead of failing the frame.
#[test]
fn informational_malformed_agent_note_reads_as_absent() {
    let frame = json!({
        "type": "system",
        "subtype": "informational",
        "content": "note",
        "level": "info",
        "agent_note": {"to": "main"},
        "uuid": "u1",
        "session_id": "s1"
    });
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::Informational(info)) = sys.as_known_system_event() else {
        panic!("expected Informational");
    };
    assert_eq!(info.agent_note, None);
}

/// `system/init` says whether shell commands run in the sandbox: `true`,
/// `false`, or `"unknown"`.
#[test]
fn init_carries_sandbox_enabled() {
    for (wire, expected) in [
        (json!(true), SandboxEnabled::On),
        (json!(false), SandboxEnabled::Off),
        (json!("unknown"), SandboxEnabled::Unknown),
    ] {
        let sys = system(json!({
            "type": "system",
            "subtype": "init",
            "session_id": "s1",
            "uuid": "u1",
            "sandbox_enabled": wire.clone()
        }));
        let init = sys.as_init().expect("typed init");
        assert_eq!(init.sandbox_enabled, Some(expected));
        assert_eq!(serde_json::to_value(expected).unwrap(), wire);
    }
}

/// `system/model_refusal_fallback` carries the model list's wording.
#[test]
fn model_refusal_fallback_carries_served_notice() {
    let sys = system(json!({
        "type": "system",
        "subtype": "model_refusal_fallback",
        "trigger": "refusal",
        "direction": "down",
        "scope": "session",
        "original_model": "claude-opus-5-5",
        "fallback_model": "claude-sonnet-5-5",
        "request_id": "req_1",
        "served_notice": {
            "text": "{model} declined this request.",
            "learn_more_url": "https://example.com/refusals"
        },
        "content": "Switched model",
        "uuid": "u1",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::ModelRefusalFallback(fallback)) = sys.as_known_system_event() else {
        panic!("expected ModelRefusalFallback");
    };
    let notice = fallback.served_notice.expect("served_notice");
    assert_eq!(notice.text, "{model} declined this request.");
    assert_eq!(
        notice.learn_more_url.as_deref(),
        Some("https://example.com/refusals")
    );
}

/// `tool_result_meta[]` entries say the writer ran the MCP call unchanged,
/// and a user frame's attachments may carry `page_count`.
#[test]
fn user_tool_result_meta_mcp_own_output() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "tool_result", "tool_use_id": "toolu_1", "content": "done"}]
        },
        "parent_tool_use_id": null,
        "tool_result_meta": [
            {"id": "toolu_1", "mcp_own_output": true}
        ],
        "file_attachments": [
            {"file_uuid": "f1", "file_name": "spec.pdf", "page_count": 12}
        ],
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    let meta = user.tool_result_meta.as_ref().unwrap();
    assert_eq!(meta[0].mcp_own_output, Some(true));
    assert_eq!(
        user.file_attachments.as_ref().unwrap()[0]["page_count"],
        json!(12)
    );
}

/// A malformed `mcp_own_output` reads as absent, as upstream catches it.
#[test]
fn user_malformed_mcp_own_output_reads_as_absent() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "tool_result", "tool_use_id": "toolu_1", "content": "done"}]
        },
        "parent_tool_use_id": null,
        "tool_result_meta": [{"id": "toolu_1", "mcp_own_output": "yes"}],
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(user.tool_result_meta.unwrap()[0].mcp_own_output, None);
}

/// `sideload_flags_disabled` is a typed startup failure reason.
#[test]
fn startup_failure_reason_sideload_flags_disabled() {
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
        "errors": ["Managed settings disable --plugin-dir."],
        "startup_failure_reason": "sideload_flags_disabled"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::SideloadFlagsDisabled)
    );
    assert_eq!(
        StartupFailureReason::SideloadFlagsDisabled.to_string(),
        "sideload_flags_disabled"
    );
}
