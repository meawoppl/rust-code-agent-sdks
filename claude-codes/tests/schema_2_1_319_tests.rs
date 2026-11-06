//! Coverage for the CLI 2.1.319 stream-json drift.
//!
//! 2.1.318 → 2.1.319 added `stopped_by_permission_check` to both `result`
//! variants, `process_turn_threads` to the success result,
//! `resume_held_for_plan` to `system/status`, `auto_mode_rule` on a
//! `tool_result_meta` entry's `permission_decision`, `dailyCapPaused` on
//! `rate_limit_info`, four auto-mode `decision_reason_code` values on
//! `system/permission_denied`, and a `voice-call` message origin. No new
//! wire labels and no removals.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, AutoModeRuleList, ClaudeOutput, KnownSystemEvent,
    PermissionDecisionOutcome, PermissionDecisionReasonType, PermissionDeniedReasonCode,
};
use serde_json::json;

/// A success result that auto mode stopped carries the flag and the
/// per-thread CPU breakdown.
#[test]
fn result_success_stopped_by_permission_check_and_threads() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 1200,
        "duration_api_ms": 900,
        "num_turns": 1,
        "result": "ok",
        "stop_reason": null,
        "terminal_reason": "aborted_streaming",
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.01,
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "stopped_by_permission_check": true,
        "process_turn_threads": {
            "main_stream": {
                "one_in": 4096,
                "main_cpu_ms": 120,
                "main_system_ms": 10,
                "gc_cpu_ms": 8,
                "process_cpu_ms": 140,
                "process_system_ms": 12,
                "run_wait_ms": 3
            },
            "between_turns": {"one_in": 4096, "main_cpu_ms": 2, "process_cpu_ms": 2}
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.stopped_by_permission_check, Some(true));
    let threads = res.process_turn_threads.as_ref().expect("threads");
    assert_eq!(threads["main_stream"]["one_in"], 4096);
    assert_eq!(threads["main_stream"]["main_cpu_ms"], 120);
    assert_eq!(threads["between_turns"]["process_cpu_ms"], 2);

    let wire = serde_json::to_value(&res).unwrap();
    assert_eq!(wire["stopped_by_permission_check"], true);
    assert_eq!(wire["process_turn_threads"]["main_stream"]["gc_cpu_ms"], 8);
}

/// An error result can carry `stopped_by_permission_check` too, and both
/// fields stay absent when the wire omits them.
#[test]
fn result_error_stopped_by_permission_check() {
    let frame = json!({
        "type": "result",
        "subtype": "error_max_turns",
        "is_error": true,
        "duration_ms": 10,
        "duration_api_ms": 5,
        "num_turns": 3,
        "stop_reason": null,
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.0,
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "errors": [],
        "stopped_by_permission_check": true
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.stopped_by_permission_check, Some(true));
    assert!(res.process_turn_threads.is_none());
    let wire = serde_json::to_value(&res).unwrap();
    assert!(wire.get("process_turn_threads").is_none());
}

/// A resumed plan-mode turn that was not re-run is flagged on `status`.
#[test]
fn status_carries_resume_held_for_plan() {
    let frame = json!({
        "type": "system",
        "subtype": "status",
        "status": null,
        "permissionMode": "default",
        "resume_held_for_plan": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let status = sys.as_status().expect("status");
    assert_eq!(status.resume_held_for_plan, Some(true));
}

/// A classifier decision names the auto-mode settings entry it applied.
#[test]
fn permission_decision_carries_auto_mode_rule() {
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
                    "reason_type": "classifier",
                    "auto_mode_rule": {"list": "allow", "name": "Read project files"}
                }
            },
            {
                "id": "toolu_02",
                "permission_decision": {
                    "decision": "reject",
                    "source": "config",
                    "reason_type": "classifier",
                    "auto_mode_rule": {"list": "soft_deny", "name": "Push to main"}
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
    let accepted = meta[0].permission_decision.as_ref().expect("decision");
    assert_eq!(accepted.decision, PermissionDecisionOutcome::Accept);
    assert_eq!(
        accepted.reason_type,
        Some(PermissionDecisionReasonType::Classifier)
    );
    let rule = accepted.auto_mode_rule.as_ref().expect("rule");
    assert_eq!(rule.list, AutoModeRuleList::Allow);
    assert_eq!(rule.name, "Read project files");
    let rejected = meta[1].permission_decision.as_ref().expect("decision");
    assert_eq!(
        rejected.auto_mode_rule.as_ref().map(|r| &r.list),
        Some(&AutoModeRuleList::SoftDeny)
    );
}

/// The rule list is an open set.
#[test]
fn auto_mode_rule_list_round_trips() {
    for (wire, list) in [
        ("allow", AutoModeRuleList::Allow),
        ("soft_deny", AutoModeRuleList::SoftDeny),
        ("future", AutoModeRuleList::Unknown("future".into())),
    ] {
        let parsed: AutoModeRuleList = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, list);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// The four auto-mode codes on `system/permission_denied` are typed.
#[test]
fn permission_denied_auto_mode_codes() {
    for (wire, code) in [
        (
            "auto_mode_no_check_in_session",
            PermissionDeniedReasonCode::AutoModeNoCheckInSession,
        ),
        (
            "auto_mode_stop_no_check_in_session",
            PermissionDeniedReasonCode::AutoModeStopNoCheckInSession,
        ),
        (
            "auto_mode_stop_no_result_for_calls",
            PermissionDeniedReasonCode::AutoModeStopNoResultForCalls,
        ),
        (
            "auto_mode_stop_no_verdict",
            PermissionDeniedReasonCode::AutoModeStopNoVerdict,
        ),
    ] {
        let frame = json!({
            "type": "system",
            "subtype": "permission_denied",
            "tool_name": "Bash",
            "tool_use_id": "toolu_1",
            "decision_reason_type": "classifier",
            "decision_reason_code": wire,
            "message": "Bash was not run",
            "uuid": "u1",
            "session_id": "s1"
        });
        assert_fully_wrapped(&frame);

        let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
            panic!("expected System");
        };
        let Some(KnownSystemEvent::PermissionDenied(denied)) = sys.as_known_system_event() else {
            panic!("expected PermissionDenied view");
        };
        assert_eq!(denied.decision_reason_code, Some(code.clone()));
        assert_eq!(code.as_str(), wire);
    }
}

/// `dailyCapPaused` tells an organization-wide pause apart from the
/// organization's monthly cap.
#[test]
fn rate_limit_info_daily_cap_paused() {
    let frame = json!({
        "type": "rate_limit_event",
        "session_id": "s1",
        "uuid": "u1",
        "rate_limit_info": {
            "status": "rejected",
            "isUsingOverage": false,
            "overageStatus": "rejected",
            "overageDisabledReason": "org_service_level_disabled",
            "dailyCapPaused": true
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::RateLimitEvent(evt) = serde_json::from_value(frame).unwrap() else {
        panic!("expected rate_limit_event");
    };
    assert_eq!(evt.rate_limit_info.daily_cap_paused, Some(true));
    let wire = serde_json::to_value(&evt).unwrap();
    assert_eq!(wire["rate_limit_info"]["dailyCapPaused"], true);
}

/// The `voice-call` origin kind passes through the open `MessageOrigin`.
#[test]
fn user_origin_voice_call() {
    let frame = json!({
        "type": "user",
        "message": {"role": "user", "content": [{"type": "text", "text": "we said to ship on Friday"}]},
        "parent_tool_use_id": null,
        "origin": {"kind": "voice-call"},
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(user.origin.expect("origin").kind, "voice-call");
}
