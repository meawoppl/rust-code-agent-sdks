//! Coverage for the CLI 2.1.300 stream-json drift.
//!
//! 2.1.296 → 2.1.300 added optional fields only, no new types and no
//! removals: `api_error_origin` on `assistant` and `result`; `server_id`,
//! `server_id_stamp` and `tool_name` on `assistant.tool_use_meta[]`;
//! `plugin` on `assistant.local_command_run`; `ran_ahead` on
//! `command_lifecycle`; `interrupted_by_shutdown`,
//! `handoff_intake_phases_ms`, nine `process_*` resource counters and
//! `input_attachments_detail.touch_compare` on `result`;
//! `prompt_submit_outcome` on `system/informational`; `shared_message_id` on
//! `system/turn_handoff_available`; `prompts_screened` on
//! `system/ui_invalidate`; and `text_not_typed` on `user`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, PromptSubmitOutcomeKind, SystemMessage,
};
use serde_json::{json, Value};

fn system(frame: Value) -> SystemMessage {
    assert_fully_wrapped(&frame);
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    sys
}

/// An API-error assistant frame names its origin, and its tool-use metadata
/// carries the connector ids and the server-side tool name.
#[test]
fn assistant_carries_api_error_origin_and_tool_use_meta_ids() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_01",
            "type": "message",
            "role": "assistant",
            "model": "claude-fable-5-1",
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "mcp__linear__create_issue",
                "input": {}
            }],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {"input_tokens": 1, "output_tokens": 1}
        },
        "parent_tool_use_id": null,
        "session_id": "s1",
        "uuid": "u1",
        "tool_use_meta": [{
            "id": "toolu_01",
            "display_name": "Create issue",
            "server_display_name": "Linear",
            "server_id": "mcpsrv_01abc",
            "server_id_stamp": "conn-42",
            "tool_name": "create_issue"
        }],
        "is_api_error_message": true,
        "api_error_status": 529,
        "api_error_origin": "first_party"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.api_error_origin.as_deref(), Some("first_party"));
    let meta = &msg.tool_use_meta[0];
    assert_eq!(meta.server_id.as_deref(), Some("mcpsrv_01abc"));
    assert_eq!(meta.server_id_stamp.as_deref(), Some("conn-42"));
    assert_eq!(meta.tool_name.as_deref(), Some("create_issue"));
}

/// A local-command row run by a plugin's command names the plugin.
#[test]
fn assistant_local_command_run_carries_plugin() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_02",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "done"}],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {"input_tokens": 0, "output_tokens": 0}
        },
        "parent_tool_use_id": null,
        "session_id": "s1",
        "uuid": "u2",
        "local_command_run": {"command": "deploy", "args": "", "plugin": "shipit"}
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    let run = msg.local_command_run.expect("local_command_run");
    assert_eq!(run.plugin.as_deref(), Some("shipit"));
}

/// A repeated `queued` lifecycle frame marks an `immediate` command whose
/// hooks ran ahead of its turn.
#[test]
fn command_lifecycle_carries_ran_ahead() {
    let frame = json!({
        "type": "command_lifecycle",
        "command_uuid": "c1",
        "state": "queued",
        "ran_ahead": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::CommandLifecycle(cmd) = serde_json::from_value(frame).unwrap() else {
        panic!("expected CommandLifecycle");
    };
    assert_eq!(cmd.ran_ahead, Some(true));
}

/// A success result carries the shutdown marker, the handoff intake split,
/// the per-turn process counters and the `touch_compare` counts.
#[test]
fn result_carries_process_counters_and_shutdown_marker() {
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
        "turn_start_resume_kind": "turn_handoff",
        "handoff_intake_phases_ms": {"startup_wait": 40, "before_read": 3},
        "process_turn_index": 2,
        "process_rss_mib_turn_start": 310,
        "process_anon_mib_turn_start": 180,
        "process_rss_mib_result": 342,
        "process_anon_mib_result": 201,
        "process_peak_rss_mib": 410,
        "process_cpu_ms_between_turns": 55,
        "process_wall_ms_between_turns": 60000,
        "process_cpu_ms_turn": 900,
        "process_wall_ms_turn": 12000,
        "input_attachments_detail": {
            "changed_files_reread": 1,
            "touch_compare": {"same": 3, "differs": 1, "via_outputs_link": 2, "enoent": 1}
        },
        "api_error_origin": "first_party",
        "interrupted_by_shutdown": true
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.interrupted_by_shutdown, Some(true));
    assert_eq!(res.api_error_origin.as_deref(), Some("first_party"));
    let phases = res.handoff_intake_phases_ms.expect("handoff phases");
    assert_eq!(phases.get("startup_wait"), Some(&40));
    assert_eq!(res.process_rss_mib_turn_start, Some(310));
    assert_eq!(res.process_peak_rss_mib, Some(410));
    assert_eq!(res.process_wall_ms_turn, Some(12000));
    let touch = res
        .input_attachments_detail
        .expect("detail")
        .touch_compare
        .expect("touch_compare");
    assert_eq!(touch.same, Some(3));
    assert_eq!(touch.via_outputs_link, Some(2));
    assert_eq!(touch.oversize, None);
}

/// The line that ends a prompt a hook blocked carries a typed outcome; an
/// unknown kind is kept as `Other`.
#[test]
fn informational_carries_prompt_submit_outcome() {
    let sys = system(json!({
        "type": "system",
        "subtype": "informational",
        "content": "Prompt blocked by hook",
        "level": "warning",
        "prompt_submit_outcome": {
            "kind": "blocked_by_hook",
            "prompt_kept": false,
            "reason": "no secrets in prompts"
        },
        "uuid": "u1",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::Informational(info)) = sys.as_known_system_event() else {
        panic!("expected Informational");
    };
    let outcome = info.prompt_submit_outcome.expect("outcome");
    assert_eq!(outcome.kind, Some(PromptSubmitOutcomeKind::BlockedByHook));
    assert!(!outcome.prompt_kept);
    assert_eq!(outcome.reason.as_deref(), Some("no secrets in prompts"));

    let sys = system(json!({
        "type": "system",
        "subtype": "informational",
        "content": "Prompt ended",
        "level": "warning",
        "prompt_submit_outcome": {
            "kind": "no_verdict",
            "no_verdict_kind": "remote_run_timed_out",
            "prompt_kept": true
        },
        "uuid": "u2",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::Informational(info)) = sys.as_known_system_event() else {
        panic!("expected Informational");
    };
    let outcome = info.prompt_submit_outcome.expect("outcome");
    assert_eq!(outcome.kind, Some(PromptSubmitOutcomeKind::NoVerdict));
    assert_eq!(
        outcome.no_verdict_kind.as_deref(),
        Some("remote_run_timed_out")
    );

    let kind: PromptSubmitOutcomeKind = serde_json::from_value(json!("vetoed_later")).unwrap();
    assert_eq!(kind, PromptSubmitOutcomeKind::Other("vetoed_later".into()));
    assert_eq!(serde_json::to_value(&kind).unwrap(), json!("vetoed_later"));
}

/// `shared_message_id` is a typed capability marker rather than landing in
/// `extra`.
#[test]
fn turn_handoff_available_carries_shared_message_id() {
    let sys = system(json!({
        "type": "system",
        "subtype": "turn_handoff_available",
        "v": 1,
        "tools": ["Write"],
        "worker_epoch": 8,
        "shared_message_id": true,
        "uuid": "u1",
        "session_id": "s1"
    }));
    let handoff = sys.as_turn_handoff_available().expect("typed accessor");
    assert_eq!(handoff.shared_message_id, Some(true));
    assert!(handoff.extra.is_empty());
}

/// `system/ui_invalidate` says when typed lines are screened by a mod.
#[test]
fn ui_invalidate_carries_prompts_screened() {
    let sys = system(json!({
        "type": "system",
        "subtype": "ui_invalidate",
        "event": "ui.render",
        "prompts_screened": true,
        "uuid": "u1",
        "session_id": "s1"
    }));
    let inv = sys.as_ui_invalidate().expect("typed accessor");
    assert_eq!(inv.prompts_screened, Some(true));
}

/// A user frame composed by a server or app is marked `text_not_typed`.
#[test]
fn user_carries_text_not_typed() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": "Summarize the release notes"}]
        },
        "parent_tool_use_id": null,
        "session_id": "6f1c9a52-0b5e-4a8e-9a43-2b1d7f0c8e11",
        "uuid": "0d9b2c3e-7a14-4f6b-8c2d-5e1f9a0b3c47",
        "text_not_typed": true
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(msg.text_not_typed, Some(true));
}
