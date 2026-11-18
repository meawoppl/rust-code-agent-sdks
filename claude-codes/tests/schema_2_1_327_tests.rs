//! Coverage for the CLI 2.1.327 stream-json drift.
//!
//! 2.1.326 → 2.1.327 added the `system/shell_cwd` subtype,
//! `host_dropped_env` on `system/init`, `comparison_fork` on
//! `system/turn_handoff_available`, `toolUseId` on a task notification's
//! `handback_report` (and `handbackToolUseId` on a hand-back message's
//! `origin`), `plugin_approval` on a tool result's `permission_decision`,
//! `boot_age_ms` and three `grace_*` waits on the result's
//! `memory_check_detail`, and the `initialize_fields_not_delivered` startup
//! failure reason.
//!
//! The drift script labels the `toolUseId` addition `ephemeral`: the
//! extractor's block for the hand-back report schema runs on into the
//! `cache_control` schema, whose literal names the block. The field belongs
//! to the report.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, MemoryCheckHold, MemoryCheckWait,
    PermissionDecisionOutcome, PluginApproval, StartupFailureReason, SystemSubtype,
};
use serde_json::json;

/// A main-thread shell call reports the folder it left its shell in.
#[test]
fn shell_cwd_parses() {
    let frame = json!({
        "type": "system",
        "subtype": "shell_cwd",
        "cwd": "/w/repo/sub",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::ShellCwd);
    assert!(sys.is_shell_cwd());
    assert_eq!(SystemSubtype::from("shell_cwd"), SystemSubtype::ShellCwd);
    assert_eq!(SystemSubtype::ShellCwd.as_str(), "shell_cwd");
    let Some(KnownSystemEvent::ShellCwd(msg)) = sys.as_known_system_event() else {
        panic!("expected ShellCwd");
    };
    assert_eq!(msg.cwd, "/w/repo/sub");
    assert_eq!(sys.as_shell_cwd(), Some(msg));
}

/// A Desktop-started init lists the managed-settings variables it dropped.
#[test]
fn init_carries_host_dropped_env() {
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "host_dropped_env": [
            {
                "source": "policySettings",
                "names": ["ANTHROPIC_BASE_URL", "CLAUDE_CODE_USE_BEDROCK"]
            }
        ]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    let dropped = init.host_dropped_env.as_ref().expect("host_dropped_env");
    assert_eq!(dropped.len(), 1);
    assert_eq!(dropped[0].source, "policySettings");
    assert_eq!(
        dropped[0].names,
        vec!["ANTHROPIC_BASE_URL", "CLAUDE_CODE_USE_BEDROCK"]
    );

    let wire = serde_json::to_value(&init).unwrap();
    assert_eq!(wire["host_dropped_env"][0]["source"], "policySettings");
}

/// A worker that answers `comparison_fork` says so.
#[test]
fn turn_handoff_available_carries_comparison_fork() {
    let frame = json!({
        "type": "system",
        "subtype": "turn_handoff_available",
        "v": 1,
        "tools": ["Bash"],
        "worker_epoch": 7,
        "comparison_fork": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::TurnHandoffAvailable(msg)) = sys.as_known_system_event() else {
        panic!("expected TurnHandoffAvailable");
    };
    assert_eq!(msg.comparison_fork, Some(true));
}

/// A hand-back report names the SubagentHandback call that sent it.
#[test]
fn handback_report_carries_tool_use_id() {
    let frame = json!({
        "type": "system",
        "subtype": "task_notification",
        "task_id": "t1",
        "status": "completed",
        "output_file": "",
        "summary": "harness notes",
        "handback": "delivered",
        "handback_report": {"text": "the report", "toolUseId": "toolu_hb"},
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let note = sys.as_task_notification().expect("task_notification");
    let report = note.handback_report.as_ref().expect("handback_report");
    assert_eq!(report.tool_use_id.as_deref(), Some("toolu_hb"));
    assert_eq!(report.warning, None);

    let wire = serde_json::to_value(report).unwrap();
    assert_eq!(wire, json!({"text": "the report", "toolUseId": "toolu_hb"}));
}

/// A hand-back message's origin carries the same id; `MessageOrigin` keeps
/// its variant fields in `extra`.
#[test]
fn origin_keeps_handback_tool_use_id() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": "report"}]
        },
        "parent_tool_use_id": null,
        "origin": {"kind": "task-notification", "handbackToolUseId": "toolu_hb"},
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    let origin = user.origin.as_ref().expect("origin");
    assert_eq!(origin.extra["handbackToolUseId"], "toolu_hb");
}

/// A `claude plugin eval` call reports how a plugin's hook figured.
#[test]
fn permission_decision_carries_plugin_approval() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [
                {"type": "tool_result", "tool_use_id": "toolu_01", "content": "ok"},
                {"type": "tool_result", "tool_use_id": "toolu_02", "content": "denied", "is_error": true}
            ]
        },
        "parent_tool_use_id": null,
        "tool_result_meta": [
            {
                "id": "toolu_01",
                "permission_decision": {
                    "decision": "accept",
                    "source": "hook",
                    "plugin_approval": "applied"
                }
            },
            {
                "id": "toolu_02",
                "permission_decision": {
                    "decision": "reject",
                    "source": "config",
                    "reason_type": "rule",
                    "plugin_approval": "refused"
                }
            }
        ],
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    let meta = user.tool_result_meta.expect("tool_result_meta");
    let applied = meta[0].permission_decision.as_ref().expect("decision");
    assert_eq!(applied.decision, PermissionDecisionOutcome::Accept);
    assert_eq!(applied.plugin_approval, Some(PluginApproval::Applied));
    let refused = meta[1].permission_decision.as_ref().expect("decision");
    assert_eq!(refused.plugin_approval, Some(PluginApproval::Refused));
}

/// `plugin_approval` is an open set.
#[test]
fn plugin_approval_round_trips() {
    for (wire, value) in [
        ("applied", PluginApproval::Applied),
        ("refused", PluginApproval::Refused),
        ("future", PluginApproval::Unknown("future".into())),
    ] {
        let parsed: PluginApproval = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, value);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// The memory check reports the start-up read's age and boot-grace waits.
#[test]
fn memory_check_detail_carries_boot_grace() {
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
            "hold": "nothing_to_send",
            "wait": "grace_answered",
            "requests": 1,
            "boot_age_ms": 4200
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
    assert_eq!(mc.hold, MemoryCheckHold::NothingToSend);
    assert_eq!(mc.wait, Some(MemoryCheckWait::GraceAnswered));
    assert_eq!(mc.boot_age_ms, Some(4200));
}

/// The three boot-grace waits are typed.
#[test]
fn memory_check_wait_grace_values_round_trip() {
    for (wire, value) in [
        ("grace_answered", MemoryCheckWait::GraceAnswered),
        ("grace_cut", MemoryCheckWait::GraceCut),
        ("grace_unanswered", MemoryCheckWait::GraceUnanswered),
    ] {
        let parsed: MemoryCheckWait = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, value);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// `initialize_fields_not_delivered` is a typed startup failure reason.
#[test]
fn startup_failure_reason_initialize_fields_not_delivered() {
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
        "errors": ["--initialize-fields names settings, but --settings was passed too."],
        "startup_failure_reason": "initialize_fields_not_delivered"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::InitializeFieldsNotDelivered)
    );
    assert_eq!(
        StartupFailureReason::InitializeFieldsNotDelivered.to_string(),
        "initialize_fields_not_delivered"
    );
}
