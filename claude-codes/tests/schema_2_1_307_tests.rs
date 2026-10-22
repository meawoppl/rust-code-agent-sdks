//! Coverage for the CLI 2.1.307 stream-json drift.
//!
//! 2.1.306 → 2.1.307 added optional fields only, no new message types and no
//! removals: `wakeup_source` and `scheduled_for` on `command_lifecycle`,
//! `requeue_if_interrupted_within_ms` on inbound `user` messages,
//! `adminRequestOffer` on `rate_limit_event.rate_limit_info`, the
//! `unsupported_runtime` feature-disabled cause, and the `empty_completion`
//! `api_error` kind.
//!
//! Each output frame below carries the new fields and is asserted **fully
//! wrapped** — the typed model captures every wire field with nothing left in
//! an untyped escape hatch.

use claude_codes::{
    assert_fully_wrapped, AdminRequestOfferState, ClaudeInput, ClaudeOutput, CommandWakeupSource,
    RemedyFeatureCause,
};
use serde_json::json;
use uuid::Uuid;

/// A `started` frame for a fired `/loop` wakeup carries its source and the
/// time it was set for.
#[test]
fn command_lifecycle_carries_wakeup_source() {
    let frame = json!({
        "type": "command_lifecycle",
        "command_uuid": "c1",
        "state": "started",
        "wakeup_source": "loop_wakeup",
        "scheduled_for": 1700000000000.0,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::CommandLifecycle(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected CommandLifecycle");
    };
    assert_eq!(msg.wakeup_source, Some(CommandWakeupSource::LoopWakeup));
    assert_eq!(msg.scheduled_for, Some(1700000000000.0));
}

/// Wakeup sources outside the known set pass through unchanged.
#[test]
fn command_wakeup_source_is_open() {
    assert_eq!(
        CommandWakeupSource::from("schedule_wakeup"),
        CommandWakeupSource::ScheduleWakeup
    );
    let other = CommandWakeupSource::from("webhook");
    assert_eq!(other, CommandWakeupSource::Unknown("webhook".to_string()));
    assert_eq!(other.as_str(), "webhook");
}

/// A rejected rate-limit frame for a Team member at their own spend limit
/// carries the admin-request offer.
#[test]
fn rate_limit_info_carries_admin_request_offer() {
    let frame = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": "rejected",
            "resetsAt": 1700003600,
            "rateLimitType": "five_hour",
            "adminRequestOffer": {
                "state": "can_ask",
                "label": "Ask your admin for more",
                "doneLabel": "Request sent",
                "confirm": true,
                "confirmPrompt": "Send a request to your admin?",
                "lastFileFailed": false,
                "revision": 2
            }
        },
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::RateLimitEvent(evt) = serde_json::from_value(frame).unwrap() else {
        panic!("expected RateLimitEvent");
    };
    let offer = evt.rate_limit_info.admin_request_offer.unwrap();
    assert_eq!(offer.state, AdminRequestOfferState::CanAsk);
    assert_eq!(offer.done_label, "Request sent");
    assert!(offer.confirm);
    assert_eq!(offer.last_file_failed, Some(false));
    assert_eq!(offer.revision, Some(2));
}

/// The requeue window is sent on inbound user messages only when set.
#[test]
fn user_input_serializes_requeue_window() {
    let mut input = ClaudeInput::user_message("hi", Uuid::nil());
    let value = serde_json::to_value(&input).unwrap();
    assert!(value.get("requeue_if_interrupted_within_ms").is_none());

    let ClaudeInput::User(msg) = &mut input else {
        panic!("expected User");
    };
    msg.requeue_if_interrupted_within_ms = Some(2000);
    let value = serde_json::to_value(&input).unwrap();
    assert_eq!(value["requeue_if_interrupted_within_ms"], 2000);
}

#[test]
fn remedy_feature_cause_unsupported_runtime() {
    let cause = RemedyFeatureCause::from("unsupported_runtime");
    assert_eq!(cause, RemedyFeatureCause::UnsupportedRuntime);
    assert_eq!(cause.as_str(), "unsupported_runtime");
}
