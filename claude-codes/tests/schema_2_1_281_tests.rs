//! Coverage for the CLI 2.1.281 stream-json drift.
//!
//! 2.1.280 → 2.1.281 added one `system` subtype (`per_turn_effort_changed`)
//! and top-level fields to three wire types, all additive (no removals):
//! `per_turn_effort_active` and `view_mode` on `system/init`, `trigger`,
//! `user_message_uuid` and `timestamp` on `conversation_reset`, and
//! `local_command_outcome` on `assistant`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, ConversationResetTrigger, KnownSystemEvent,
    LocalCommandOutcomeKind, SystemSubtype, ViewMode,
};
use serde_json::json;

/// The new `system/per_turn_effort_changed` subtype is modeled and round-trips
/// fully wrapped through both typed accessors.
#[test]
fn system_per_turn_effort_changed_fully_wrapped() {
    let frame = json!({
        "type": "system",
        "subtype": "per_turn_effort_changed",
        "per_turn_effort_active": false,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::PerTurnEffortChanged);
    assert!(sys.is_per_turn_effort_changed());

    let direct = sys.as_per_turn_effort_changed().expect("typed accessor");
    assert!(!direct.per_turn_effort_active);
    assert_eq!(direct.uuid, "u1");

    let Some(KnownSystemEvent::PerTurnEffortChanged(known)) = sys.as_known_system_event() else {
        panic!("expected PerTurnEffortChanged event");
    };
    assert_eq!(known.session_id, "s1");
}

/// `system/init` carries the per-turn effort flag and the transcript view.
#[test]
fn system_init_carries_per_turn_effort_and_view_mode() {
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "per_turn_effort_active": true,
        "view_mode": "focus"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    assert_eq!(init.per_turn_effort_active, Some(true));
    assert_eq!(init.view_mode, Some(ViewMode::Focus));
}

/// `view_mode` is an open set: an unknown value is preserved, not dropped.
#[test]
fn view_mode_is_open_set() {
    for (wire, expected) in [
        ("focus", ViewMode::Focus),
        ("default", ViewMode::Default),
        ("split", ViewMode::Unknown("split".into())),
    ] {
        let parsed: ViewMode = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

/// `conversation_reset` carries what triggered it, the `/clear` message's
/// uuid, and a display timestamp.
#[test]
fn conversation_reset_carries_trigger_uuid_and_timestamp() {
    let frame = json!({
        "type": "conversation_reset",
        "new_conversation_id": "c2",
        "uuid": "u1",
        "session_id": "s1",
        "trigger": "clear",
        "user_message_uuid": "6f1d2c3e-4b5a-4c6d-8e7f-9a0b1c2d3e4f",
        "timestamp": "2026-09-24T08:00:00.000Z"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::ConversationReset(reset) = serde_json::from_value(frame).unwrap() else {
        panic!("expected ConversationReset");
    };
    assert_eq!(reset.trigger, Some(ConversationResetTrigger::Clear));
    assert_eq!(
        reset.user_message_uuid.as_deref(),
        Some("6f1d2c3e-4b5a-4c6d-8e7f-9a0b1c2d3e4f")
    );
    assert_eq!(reset.timestamp.as_deref(), Some("2026-09-24T08:00:00.000Z"));
}

/// A pre-2.1.281 `conversation_reset` (no trigger) still parses, and the
/// trigger is an open set.
#[test]
fn conversation_reset_trigger_is_optional_and_open_set() {
    let legacy = json!({
        "type": "conversation_reset",
        "new_conversation_id": "c2",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&legacy);
    let ClaudeOutput::ConversationReset(reset) = serde_json::from_value(legacy).unwrap() else {
        panic!("expected ConversationReset");
    };
    assert_eq!(reset.trigger, None);

    for (wire, expected) in [
        ("plan_mode_exit", ConversationResetTrigger::PlanModeExit),
        ("fresh_session", ConversationResetTrigger::FreshSession),
        ("onboarding", ConversationResetTrigger::Onboarding),
        ("rewind", ConversationResetTrigger::Unknown("rewind".into())),
    ] {
        let parsed: ConversationResetTrigger = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

fn local_command_frame(outcome: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "Unknown command: /foc. Did you mean /focus?"}],
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
        "local_command_run": {"command": "foc", "args": ""},
        "local_command_outcome": outcome
    })
}

/// The local-command twin carries a typed outcome, with the `unknown` kind's
/// closest-command suggestion.
#[test]
fn assistant_carries_local_command_outcome() {
    let frame = local_command_frame(json!({"kind": "unknown", "suggestion": "focus"}));
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    let outcome = msg.local_command_outcome.expect("typed outcome");
    assert_eq!(outcome.kind, LocalCommandOutcomeKind::Unknown);
    assert_eq!(outcome.suggestion.as_deref(), Some("focus"));

    let frame = local_command_frame(json!({"kind": "restart_required"}));
    assert_fully_wrapped(&frame);
    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    let outcome = msg.local_command_outcome.expect("typed outcome");
    assert_eq!(outcome.kind, LocalCommandOutcomeKind::RestartRequired);
    assert_eq!(outcome.suggestion, None);
}

/// The outcome kind is an open set: a kind this crate does not know is kept
/// as `Other` so a consumer can treat it as absent.
#[test]
fn local_command_outcome_kind_is_open_set() {
    for (wire, expected) in [
        (
            "unavailable_headless",
            LocalCommandOutcomeKind::UnavailableHeadless,
        ),
        ("unknown", LocalCommandOutcomeKind::Unknown),
        ("failed", LocalCommandOutcomeKind::Failed),
        ("restart_required", LocalCommandOutcomeKind::RestartRequired),
        (
            "deprecated",
            LocalCommandOutcomeKind::Other("deprecated".into()),
        ),
    ] {
        let parsed: LocalCommandOutcomeKind = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.to_string(), wire);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}
