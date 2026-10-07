//! Coverage for the CLI 2.1.293 stream-json drift.
//!
//! 2.1.292 → 2.1.293 added `awaited` on `task_started` and
//! `background_tasks_changed` entries, `subagent_type` on
//! `background_tasks_changed` entries, `permission_decision` on the `user`
//! frame's `tool_result_meta` entries, and the `org_config_required_unavailable`
//! / `org_config_refused` startup failure reasons. It removed
//! `system_prompt_detail` from `result`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, PermissionDecisionOutcome,
    PermissionDecisionReasonType, StartupFailureReason,
};
use serde_json::json;

/// An awaited resumed-subagent run is flagged on `task_started`.
#[test]
fn task_started_carries_awaited() {
    let frame = json!({
        "type": "system",
        "subtype": "task_started",
        "task_id": "t1",
        "run_id": "r2",
        "tool_use_id": "toolu_01",
        "description": "resumed agent",
        "task_type": "local_agent",
        "subagent_type": "general-purpose",
        "is_backgrounded": true,
        "awaited": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let started = sys.as_task_started().expect("task_started");
    assert_eq!(started.awaited, Some(true));
    assert_eq!(started.tool_use_id.as_deref(), Some("toolu_01"));
}

/// `background_tasks_changed` entries carry the agent type and `awaited`.
#[test]
fn background_tasks_changed_entries_carry_subagent_type_and_awaited() {
    let frame = json!({
        "type": "system",
        "subtype": "background_tasks_changed",
        "tasks": [
            {
                "task_id": "t1",
                "run_id": "r2",
                "task_type": "local_agent",
                "subagent_type": "general-purpose",
                "description": "resumed agent",
                "awaited": true
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
    assert_eq!(
        changed.tasks[0].subagent_type.as_deref(),
        Some("general-purpose")
    );
    assert_eq!(changed.tasks[0].awaited, Some(true));
    assert_eq!(changed.tasks[1].subagent_type, None);
    assert_eq!(changed.tasks[1].awaited, None);
}

/// `tool_result_meta` entries carry how the permission check ended.
#[test]
fn user_tool_result_meta_carries_permission_decision() {
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
                    "source": "config",
                    "reason_type": "rule"
                }
            },
            {
                "id": "toolu_02",
                "non_execution_kind": "user-rejected",
                "permission_decision": {"decision": "reject", "source": "user_reject"}
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
    let accepted = meta[0].permission_decision.as_ref().expect("decision");
    assert_eq!(accepted.decision, PermissionDecisionOutcome::Accept);
    assert_eq!(accepted.source, "config");
    assert_eq!(
        accepted.reason_type,
        Some(PermissionDecisionReasonType::Rule)
    );
    let rejected = meta[1].permission_decision.as_ref().expect("decision");
    assert_eq!(rejected.decision, PermissionDecisionOutcome::Reject);
    assert_eq!(rejected.reason_type, None);
}

/// The decision enums round-trip, unknown values included.
#[test]
fn permission_decision_enums_round_trip() {
    for (wire, typed) in [
        ("accept", PermissionDecisionOutcome::Accept),
        ("reject", PermissionDecisionOutcome::Reject),
        ("cancelled", PermissionDecisionOutcome::Cancelled),
        ("later", PermissionDecisionOutcome::Unknown("later".into())),
    ] {
        let parsed: PermissionDecisionOutcome = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, typed);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
    for (wire, typed) in [
        ("rule", PermissionDecisionReasonType::Rule),
        ("mode", PermissionDecisionReasonType::Mode),
        (
            "subcommandResults",
            PermissionDecisionReasonType::SubcommandResults,
        ),
        (
            "permissionPromptTool",
            PermissionDecisionReasonType::PermissionPromptTool,
        ),
        ("hook", PermissionDecisionReasonType::Hook),
        ("asyncAgent", PermissionDecisionReasonType::AsyncAgent),
        (
            "sandboxOverride",
            PermissionDecisionReasonType::SandboxOverride,
        ),
        ("workingDir", PermissionDecisionReasonType::WorkingDir),
        ("safetyCheck", PermissionDecisionReasonType::SafetyCheck),
        ("classifier", PermissionDecisionReasonType::Classifier),
        ("other", PermissionDecisionReasonType::Other),
        (
            "someFuture",
            PermissionDecisionReasonType::Unknown("someFuture".into()),
        ),
    ] {
        let parsed: PermissionDecisionReasonType = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, typed);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// A startup refusal for organization config parses to its typed reason.
#[test]
fn result_carries_org_config_startup_failure() {
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
        "total_cost_usd": 0,
        "usage": {
            "input_tokens": 0,
            "output_tokens": 0,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0
        },
        "modelUsage": {},
        "permission_denials": [],
        "errors": ["Your organization's settings could not be loaded."],
        "startup_failure_reason": "org_config_required_unavailable"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::OrgConfigRequiredUnavailable)
    );
}
