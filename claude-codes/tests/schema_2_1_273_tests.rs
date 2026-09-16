//! Coverage for the CLI 2.1.273 stream-json additive drift.
//!
//! 2.1.270 → 2.1.273 added three `system` subtypes (`turn_handoff_available`,
//! `turn_preempted`, `peer_message_hold`) and top-level fields to three wire
//! types, all additive (no removals): `usage_report` and `local_command_run`
//! on `assistant`, `reason` on `system/task_notification`, and `trigger` on
//! `system/dev_intent` (whose `kind` set also grew `android_app`).
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, DevIntentKind, DevIntentTrigger, KnownSystemEvent,
    PeerMessageHoldCause, PeerMessageHoldOutcome, PeerMessageHoldState, PeerMessageLane,
    SystemSubtype, TaskEndReason, TaskStatus,
};
use serde_json::json;

/// The new `system/turn_handoff_available` subtype is modeled and round-trips
/// fully wrapped through both typed accessors.
#[test]
fn system_turn_handoff_available_fully_wrapped() {
    let frame = json!({
        "type": "system",
        "subtype": "turn_handoff_available",
        "v": 1,
        "tools": ["Bash", "Edit"],
        "worker_epoch": 3,
        "relay_marker": true,
        "uuid": "u1",
        "session_id": "s1",
        "future_field": "preserved"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::TurnHandoffAvailable);
    assert!(sys.is_turn_handoff_available());

    let direct = sys.as_turn_handoff_available().expect("typed accessor");
    assert_eq!(direct.v, 1);
    assert_eq!(direct.tools, vec!["Bash", "Edit"]);
    assert_eq!(direct.worker_epoch, 3);
    assert_eq!(direct.relay_marker, Some(true));
    assert_eq!(direct.extra["future_field"], "preserved");

    let Some(KnownSystemEvent::TurnHandoffAvailable(known)) = sys.as_known_system_event() else {
        panic!("expected TurnHandoffAvailable event");
    };
    assert_eq!(known.session_id, "s1");
}

/// The new `system/turn_preempted` subtype round-trips fully wrapped, with an
/// empty `preempted_message_uuids` tolerated on both sides.
#[test]
fn system_turn_preempted_fully_wrapped() {
    let frame = json!({
        "type": "system",
        "subtype": "turn_preempted",
        "reason": "rapid_followup",
        "preempted_by_uuid": "um2",
        "preempted_message_uuids": ["um1"],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert!(sys.is_turn_preempted());
    let direct = sys.as_turn_preempted().expect("typed accessor");
    assert_eq!(direct.reason, "rapid_followup");
    assert_eq!(direct.preempted_by_uuid, "um2");
    assert_eq!(direct.preempted_message_uuids, vec!["um1"]);

    let empty = json!({
        "type": "system",
        "subtype": "turn_preempted",
        "reason": "rapid_followup",
        "preempted_by_uuid": "um2",
        "preempted_message_uuids": [],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&empty);
}

/// The new `system/peer_message_hold` subtype round-trips fully wrapped in
/// both its `held` (with `cause`) and `dropped` (with `outcome`) forms, and
/// each closed-looking set still tolerates an unknown value.
#[test]
fn system_peer_message_hold_fully_wrapped() {
    let held = json!({
        "type": "system",
        "subtype": "peer_message_hold",
        "state": "held",
        "message_uuid": "m1",
        "lane": "socket",
        "from": "agent-7",
        "from_name": "Reviewer",
        "cause": "mode-mismatch",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&held);

    let ClaudeOutput::System(sys) = serde_json::from_value(held).unwrap() else {
        panic!("expected System");
    };
    assert!(sys.is_peer_message_hold());
    let direct = sys.as_peer_message_hold().expect("typed accessor");
    assert_eq!(direct.state, PeerMessageHoldState::Held);
    assert_eq!(direct.lane, PeerMessageLane::Socket);
    assert_eq!(direct.from, "agent-7");
    assert_eq!(direct.from_name.as_deref(), Some("Reviewer"));
    assert_eq!(direct.cause, Some(PeerMessageHoldCause::ModeMismatch));
    assert_eq!(direct.outcome, None);

    let dropped = json!({
        "type": "system",
        "subtype": "peer_message_hold",
        "state": "dropped",
        "message_uuid": "m1",
        "lane": "bridge",
        "from": "",
        "outcome": "expired",
        "uuid": "u2",
        "session_id": "s1"
    });
    assert_fully_wrapped(&dropped);
    let ClaudeOutput::System(sys) = serde_json::from_value(dropped).unwrap() else {
        panic!("expected System");
    };
    let direct = sys.as_peer_message_hold().unwrap();
    assert_eq!(direct.state, PeerMessageHoldState::Dropped);
    assert_eq!(direct.lane, PeerMessageLane::Bridge);
    assert_eq!(direct.outcome, Some(PeerMessageHoldOutcome::Expired));

    let novel = json!({
        "type": "system",
        "subtype": "peer_message_hold",
        "state": "parked",
        "lane": "carrier-pigeon",
        "from": "x",
        "cause": "new-cause",
        "outcome": "new-outcome",
        "uuid": "u3",
        "session_id": "s1"
    });
    assert_fully_wrapped(&novel);
    let ClaudeOutput::System(sys) = serde_json::from_value(novel).unwrap() else {
        panic!("expected System");
    };
    let direct = sys.as_peer_message_hold().unwrap();
    assert_eq!(direct.state.as_str(), "parked");
    assert_eq!(direct.lane.as_str(), "carrier-pigeon");
    assert_eq!(direct.cause.unwrap().as_str(), "new-cause");
    assert_eq!(direct.outcome.unwrap().as_str(), "new-outcome");
}

/// `system/dev_intent` now carries `trigger` and can report `android_app`;
/// an unknown trigger falls back to `Unknown` rather than failing the frame.
#[test]
fn system_dev_intent_carries_trigger_and_android_kind() {
    let frame = json!({
        "type": "system",
        "subtype": "dev_intent",
        "kind": "android_app",
        "trigger": "project_scan",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let direct = sys.as_dev_intent().expect("typed accessor");
    assert_eq!(direct.kind, DevIntentKind::AndroidApp);
    assert_eq!(direct.trigger, Some(DevIntentTrigger::ProjectScan));

    let novel = json!({
        "type": "system",
        "subtype": "dev_intent",
        "kind": "ios_app",
        "trigger": "holographic_edit",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&novel);
    let ClaudeOutput::System(sys) = serde_json::from_value(novel).unwrap() else {
        panic!("expected System");
    };
    let direct = sys.as_dev_intent().unwrap();
    assert_eq!(
        direct.trigger,
        Some(DevIntentTrigger::Unknown("holographic_edit".to_string()))
    );

    let old = json!({
        "type": "system",
        "subtype": "dev_intent",
        "kind": "ios_app",
        "uuid": "u1",
        "session_id": "s1"
    });
    let ClaudeOutput::System(sys) = serde_json::from_value(old).unwrap() else {
        panic!("expected System");
    };
    let direct = sys.as_dev_intent().unwrap();
    assert_eq!(direct.trigger, None);
    let reserialized = serde_json::to_string(&direct).unwrap();
    assert!(!reserialized.contains("trigger"));
}

/// `system/task_notification` carries `reason: worker_restart` on an orphaned
/// task and leaves the key absent on an ordinary stop.
#[test]
fn system_task_notification_carries_reason() {
    let frame = json!({
        "type": "system",
        "subtype": "task_notification",
        "task_id": "t1",
        "status": "stopped",
        "reason": "worker_restart",
        "output_file": "/tmp/out.txt",
        "summary": "orphaned",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let direct = sys.as_task_notification().expect("typed accessor");
    assert_eq!(direct.status, TaskStatus::Stopped);
    assert_eq!(direct.reason, Some(TaskEndReason::WorkerRestart));

    let ordinary = json!({
        "type": "system",
        "subtype": "task_notification",
        "task_id": "t1",
        "status": "completed",
        "output_file": "/tmp/out.txt",
        "summary": "done",
        "uuid": "u1",
        "session_id": "s1"
    });
    let ClaudeOutput::System(sys) = serde_json::from_value(ordinary).unwrap() else {
        panic!("expected System");
    };
    let direct = sys.as_task_notification().unwrap();
    assert_eq!(direct.reason, None);
    let reserialized = serde_json::to_string(&direct).unwrap();
    assert!(!reserialized.contains("reason"));
}

/// An `assistant` frame carrying the `usage_report` twin of a `/usage` result
/// round-trips fully wrapped, including a scoped row and extra-usage spend.
#[test]
fn assistant_carries_usage_report() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "Session: $0.12"}],
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
        "usage_report": {
            "session": {
                "total_cost_usd": 0.12,
                "total_api_duration_ms": 4000,
                "total_duration_ms": 9000,
                "total_lines_added": 10,
                "total_lines_removed": 2,
                "model_usage": {
                    "claude-opus-4-8": {
                        "inputTokens": 100,
                        "outputTokens": 50,
                        "thinkingTokens": 20,
                        "cacheReadInputTokens": 0,
                        "cacheCreationInputTokens": 0,
                        "webSearchRequests": 0,
                        "costUSD": 0.12,
                        "contextWindow": 200000,
                        "maxOutputTokens": 32000,
                        "canonicalModel": "claude-opus-4-8",
                        "provider": "firstParty",
                        "costBasis": "list"
                    }
                }
            },
            "rate_limits": {
                "limits": [
                    {
                        "kind": "session",
                        "group": "session",
                        "percent": 12.5,
                        "resets_at": "2026-09-16T12:00:00Z",
                        "scope": null,
                        "severity": "normal",
                        "is_active": true
                    },
                    {
                        "kind": "weekly_scoped",
                        "group": "weekly",
                        "percent": 40,
                        "resets_at": null,
                        "scope": {"model": {"display_name": "Opus"}},
                        "severity": "warning"
                    }
                ],
                "extra_usage": {
                    "is_enabled": true,
                    "monthly_limit": 5000,
                    "used_credits": 120,
                    "utilization": 2.4,
                    "currency": "USD"
                }
            }
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected assistant");
    };
    let report = msg.usage_report.expect("usage_report");
    assert_eq!(report.session.total_lines_added, 10);
    assert_eq!(
        report.session.model_usage["claude-opus-4-8"].input_tokens,
        100
    );
    let limits = report.rate_limits.expect("rate_limits");
    let rows = limits.limits.expect("limits rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].kind, "session");
    assert_eq!(rows[0].is_active, Some(true));
    assert_eq!(
        rows[1]
            .scope
            .as_ref()
            .unwrap()
            .model
            .as_ref()
            .unwrap()
            .display_name,
        "Opus"
    );
    let extra = limits.extra_usage.expect("extra_usage");
    assert!(extra.is_enabled);
    assert_eq!(extra.currency.as_deref(), Some("USD"));

    let unavailable = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [],
            "stop_reason": null,
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            }
        },
        "session_id": "s1",
        "usage_report": {
            "session": {
                "total_cost_usd": 0.0,
                "total_api_duration_ms": 0,
                "total_duration_ms": 0,
                "total_lines_added": 0,
                "total_lines_removed": 0,
                "model_usage": {}
            },
            "rate_limits": null
        }
    });
    assert_fully_wrapped(&unavailable);
    let ClaudeOutput::Assistant(msg) = serde_json::from_value(unavailable).unwrap() else {
        panic!("expected assistant");
    };
    assert!(msg.usage_report.unwrap().rate_limits.is_none());
}

/// An `assistant` local-command twin carrying `local_command_run` round-trips
/// fully wrapped, and an older frame leaves both new keys absent.
#[test]
fn assistant_carries_local_command_run() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "Model set to opus"}],
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
        "is_virtual": true,
        "local_command_run": {"command": "model", "args": "opus"}
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected assistant");
    };
    let run = msg.local_command_run.expect("local_command_run");
    assert_eq!(run.command, "model");
    assert_eq!(run.args, "opus");

    let old = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-8",
            "content": [],
            "stop_reason": null,
            "usage": {
                "input_tokens": 1,
                "output_tokens": 2,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            }
        },
        "session_id": "s1"
    });
    let ClaudeOutput::Assistant(msg) = serde_json::from_value(old).unwrap() else {
        panic!("expected assistant");
    };
    assert!(msg.local_command_run.is_none());
    assert!(msg.usage_report.is_none());
    let reserialized = serde_json::to_string(&msg).unwrap();
    assert!(!reserialized.contains("local_command_run"));
    assert!(!reserialized.contains("usage_report"));
}
