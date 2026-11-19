//! Coverage for the CLI 2.1.328 stream-json drift.
//!
//! 2.1.327 → 2.1.328 added the `system/task_list_statuses` subtype,
//! `message_undelivered` on the result, `saved_files` on
//! `system/informational`, `reason` and the `denied` outcome on
//! `system/peer_message_hold`, `memberSpendLimit` on the rate-limit info,
//! and the `container_limit` tool-result remedy kind.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, MemberSpendLimit, MessageUndelivered,
    PeerMessageDropReason, PeerMessageHoldOutcome, SpendLimitPeriod, SystemSubtype, TodoStatus,
    ToolResultRemedyKind,
};
use serde_json::json;

/// Claude Code reports the task-list statuses it set itself.
#[test]
fn task_list_statuses_parses() {
    let frame = json!({
        "type": "system",
        "subtype": "task_list_statuses",
        "statuses": [
            {"id": "1", "status": "completed"},
            {"id": "2", "status": "in_progress"},
            {"id": "3", "status": "pending"}
        ],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::TaskListStatuses);
    assert!(sys.is_task_list_statuses());
    assert_eq!(
        SystemSubtype::from("task_list_statuses"),
        SystemSubtype::TaskListStatuses
    );
    assert_eq!(
        SystemSubtype::TaskListStatuses.as_str(),
        "task_list_statuses"
    );
    let Some(KnownSystemEvent::TaskListStatuses(msg)) = sys.as_known_system_event() else {
        panic!("expected TaskListStatuses");
    };
    assert_eq!(msg.statuses.len(), 3);
    assert_eq!(msg.statuses[0].id, "1");
    assert_eq!(msg.statuses[0].status, TodoStatus::Completed);
    assert_eq!(msg.statuses[1].status, TodoStatus::InProgress);
    assert_eq!(msg.statuses[2].status, TodoStatus::Pending);
    assert_eq!(sys.as_task_list_statuses(), Some(msg));
}

/// The `--cloud` client's result for a message it could not deliver.
#[test]
fn result_carries_message_undelivered() {
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
        "message_undelivered": "session_not_opened"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.message_undelivered,
        Some(MessageUndelivered::SessionNotOpened)
    );
}

/// `message_undelivered` is an open set.
#[test]
fn message_undelivered_round_trips() {
    for (wire, value) in [
        ("session_not_opened", MessageUndelivered::SessionNotOpened),
        ("send_failed", MessageUndelivered::SendFailed),
        ("future", MessageUndelivered::Unknown("future".into())),
    ] {
        let parsed: MessageUndelivered = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, value);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// The `connector_files` line lists the files a connector saved.
#[test]
fn informational_carries_saved_files() {
    let frame = json!({
        "type": "system",
        "subtype": "informational",
        "content": "Saved 2 files from Drive:\n  ~/Downloads/a.pdf\n  ~/Downloads/b.csv",
        "level": "info",
        "tag": "connector_files",
        "saved_files": [
            {"line": 1, "path": "/home/u/Downloads/a.pdf", "shown": "~/Downloads/a.pdf"},
            {"line": 2, "path": "/home/u/Downloads/b.csv", "shown": "~/Downloads/b.csv"}
        ],
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
    let files = info.saved_files.expect("saved_files");
    assert_eq!(files.len(), 2);
    assert_eq!(files[1].line, 2);
    assert_eq!(files[1].path, "/home/u/Downloads/b.csv");
    assert_eq!(files[1].shown, "~/Downloads/b.csv");
}

/// A dropped peer message says which check discarded it.
#[test]
fn peer_message_hold_carries_reason() {
    let frame = json!({
        "type": "system",
        "subtype": "peer_message_hold",
        "state": "dropped",
        "message_uuid": "m1",
        "lane": "socket",
        "from": "peer@host",
        "outcome": "dropped",
        "reason": "hop-loop",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::PeerMessageHold(hold)) = sys.as_known_system_event() else {
        panic!("expected PeerMessageHold");
    };
    assert_eq!(hold.outcome, Some(PeerMessageHoldOutcome::Dropped));
    assert_eq!(hold.reason, Some(PeerMessageDropReason::HopLoop));
}

/// A peer message the person declined ends `denied`.
#[test]
fn peer_message_hold_denied_outcome() {
    let parsed: PeerMessageHoldOutcome = serde_json::from_value(json!("denied")).unwrap();
    assert_eq!(parsed, PeerMessageHoldOutcome::Denied);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), json!("denied"));
}

/// Every drop reason is typed, and the set is open.
#[test]
fn peer_message_drop_reason_round_trips() {
    for (wire, value) in [
        ("rate-limited", PeerMessageDropReason::RateLimited),
        ("duplicate", PeerMessageDropReason::Duplicate),
        ("hop-loop", PeerMessageDropReason::HopLoop),
        ("hop-runaway", PeerMessageDropReason::HopRunaway),
        ("queue-full", PeerMessageDropReason::QueueFull),
        ("unverified", PeerMessageDropReason::Unverified),
        ("future", PeerMessageDropReason::Unknown("future".into())),
    ] {
        let parsed: PeerMessageDropReason = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, value);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// A rejected rate-limit event marks the member's own spend limit.
#[test]
fn rate_limit_info_carries_member_spend_limit() {
    let frame = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": "rejected",
            "resetsAt": 1700003600,
            "rateLimitType": "five_hour",
            "memberSpendLimit": {"period": "weekly"}
        },
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::RateLimitEvent(evt) = serde_json::from_value(frame).unwrap() else {
        panic!("expected RateLimitEvent");
    };
    assert_eq!(
        evt.rate_limit_info.member_spend_limit,
        Some(MemberSpendLimit {
            period: Some(SpendLimitPeriod::Weekly)
        })
    );
}

/// No period means the monthly limit; the empty object still round-trips.
#[test]
fn member_spend_limit_without_period() {
    let parsed: MemberSpendLimit = serde_json::from_value(json!({})).unwrap();
    assert_eq!(parsed.period, None);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), json!({}));
    for (wire, value) in [
        ("daily", SpendLimitPeriod::Daily),
        ("weekly", SpendLimitPeriod::Weekly),
        ("monthly", SpendLimitPeriod::Monthly),
        ("future", SpendLimitPeriod::Unknown("future".into())),
    ] {
        let parsed: SpendLimitPeriod = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, value);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// A cloud session refused at the container limit carries the remedy kind.
#[test]
fn tool_result_remedy_container_limit() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "container limit", "is_error": true}
            ]
        },
        "parent_tool_use_id": null,
        "session_id": "1d1c4a2e-6f1b-4b7e-9c3d-2e5f6a7b8c9d",
        "uuid": "3f8b2c1a-9d4e-4f6a-8b7c-1a2b3c4d5e6f",
        "tool_result_meta": [
            {"id": "toolu_1", "remedy": {"kind": "container_limit"}}
        ]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    let meta = user.tool_result_meta.expect("tool_result_meta");
    let remedy = meta[0].remedy.as_ref().expect("remedy");
    assert_eq!(remedy.kind, ToolResultRemedyKind::ContainerLimit);
    assert_eq!(
        ToolResultRemedyKind::ContainerLimit.as_str(),
        "container_limit"
    );
}
