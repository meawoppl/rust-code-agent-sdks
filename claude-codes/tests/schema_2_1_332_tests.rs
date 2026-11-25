//! Coverage for the CLI 2.1.332 stream-json drift.
//!
//! 2.1.331 → 2.1.332 added the answerable-hold fields on
//! `system/peer_message_hold` (`hold_id`, `can_deliver`, `always_scope`,
//! `session_class`, `verified_pid`, `preview`), `mcp_task` on
//! `task_notification`, `refusal_retry` on user frames, `call_ended` and
//! `rewritten_by_plugin` on `tool_result_meta` entries, the `needs_person`
//! / `advice_to_claude` remedy kinds with `blocker`, and the
//! `verification_required` hold fields on `api_error_params`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ApiErrorHoldWording, ClaudeOutput, KnownSystemEvent, PeerAlwaysScope,
    PeerSessionClass, RemedyBlocker, RemedyFeature, ToolCallEnded, ToolResultRemedyKind,
};
use serde_json::json;

fn system(frame: serde_json::Value) -> claude_codes::SystemMessage {
    assert_fully_wrapped(&frame);
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    sys
}

fn user(frame: serde_json::Value) -> claude_codes::io::UserMessage {
    assert_fully_wrapped(&frame);
    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    user
}

/// A held peer message carries the handle and context a host needs to let
/// a person answer it.
#[test]
fn peer_message_hold_carries_answerable_fields() {
    let sys = system(json!({
        "type": "system",
        "subtype": "peer_message_hold",
        "state": "held",
        "message_uuid": "m1",
        "lane": "socket",
        "from": "peer@host",
        "cause": "mode-mismatch",
        "hold_id": "h1",
        "can_deliver": true,
        "always_scope": "mine",
        "session_class": "bypass",
        "verified_pid": 4242,
        "preview": {
            "text": "please run the tests",
            "total_chars": 20,
            "other_blocks": 1,
            "is_whole": false
        },
        "uuid": "u1",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::PeerMessageHold(hold)) = sys.as_known_system_event() else {
        panic!("expected PeerMessageHold");
    };
    assert_eq!(hold.hold_id.as_deref(), Some("h1"));
    assert_eq!(hold.can_deliver, Some(true));
    assert_eq!(hold.always_scope, Some(PeerAlwaysScope::Mine));
    assert_eq!(hold.session_class, Some(PeerSessionClass::Bypass));
    assert_eq!(hold.verified_pid, Some(4242));
    let preview = hold.preview.as_ref().expect("preview");
    assert_eq!(preview.text, "please run the tests");
    assert_eq!(preview.total_chars, 20);
    assert_eq!(preview.other_blocks, 1);
    assert!(!preview.is_whole);
}

/// A message that only waits for room in the queue is held with no cause.
#[test]
fn peer_message_hold_queue_wait_has_no_cause() {
    let sys = system(json!({
        "type": "system",
        "subtype": "peer_message_hold",
        "state": "held",
        "lane": "stdin",
        "from": "",
        "hold_id": "h2",
        "can_deliver": false,
        "always_scope": "anyone",
        "session_class": "prompting",
        "uuid": "u2",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::PeerMessageHold(hold)) = sys.as_known_system_event() else {
        panic!("expected PeerMessageHold");
    };
    assert_eq!(hold.cause, None);
    assert_eq!(hold.can_deliver, Some(false));
    assert_eq!(hold.always_scope, Some(PeerAlwaysScope::Anyone));
    assert_eq!(hold.session_class, Some(PeerSessionClass::Prompting));

    let future: PeerAlwaysScope = serde_json::from_value(json!("team")).unwrap();
    assert_eq!(future, PeerAlwaysScope::Unknown("team".into()));
    assert_eq!(serde_json::to_value(&future).unwrap(), json!("team"));
}

/// A finished MCP task reports its times on `task_notification`.
#[test]
fn task_notification_carries_mcp_task_times() {
    let sys = system(json!({
        "type": "system",
        "subtype": "task_notification",
        "task_id": "t1",
        "status": "completed",
        "output_file": "",
        "summary": "done",
        "tool_use_id": "toolu_1",
        "mcp_task": {
            "start_time": 1_764_000_000_000_i64,
            "end_time": 1_764_000_090_000_i64,
            "server_elapsed_ms": 85_000
        },
        "uuid": "u1",
        "session_id": "s1"
    }));
    let note = sys.as_task_notification().expect("task_notification");
    let times = note.mcp_task.expect("mcp_task");
    assert_eq!(times.start_time, 1_764_000_000_000);
    assert_eq!(times.end_time, 1_764_000_090_000);
    assert_eq!(times.server_elapsed_ms, Some(85_000));
}

/// `server_elapsed_ms` is absent unless the server reported the end.
#[test]
fn task_notification_mcp_task_without_server_elapsed() {
    let sys = system(json!({
        "type": "system",
        "subtype": "task_notification",
        "task_id": "t1",
        "status": "stopped",
        "output_file": "",
        "summary": "stopped",
        "mcp_task": {"start_time": 10, "end_time": 20},
        "uuid": "u1",
        "session_id": "s1"
    }));
    let note = sys.as_task_notification().expect("task_notification");
    assert_eq!(note.mcp_task.unwrap().server_elapsed_ms, None);
}

/// The CLI's synthetic re-ask after a refusal is marked `refusal_retry`, and
/// its tool results say how unfinished calls ended and whether a plugin
/// rewrote them.
#[test]
fn user_refusal_retry_and_tool_result_meta_fields() {
    let user = user(json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "not run", "is_error": true},
                {"type": "tool_result", "tool_use_id": "toolu_2", "content": "trimmed"}
            ]
        },
        "parent_tool_use_id": null,
        "refusal_retry": true,
        "tool_result_meta": [
            {"id": "toolu_1", "call_ended": "not_started"},
            {"id": "toolu_2", "rewritten_by_plugin": true}
        ],
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    }));
    assert_eq!(user.refusal_retry, Some(true));
    let meta = user.tool_result_meta.as_ref().unwrap();
    assert_eq!(meta[0].call_ended, Some(ToolCallEnded::NotStarted));
    assert_eq!(meta[0].rewritten_by_plugin, None);
    assert_eq!(meta[1].call_ended, None);
    assert_eq!(meta[1].rewritten_by_plugin, Some(true));
}

/// Upstream declares `call_ended` with `.catch(undefined)` and
/// `rewritten_by_plugin` with `.catch(true)`.
#[test]
fn tool_result_meta_malformed_values_follow_upstream_catch() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "tool_result", "tool_use_id": "toolu_1", "content": "x"}]
        },
        "parent_tool_use_id": null,
        "tool_result_meta": [
            {"id": "toolu_1", "call_ended": 7, "rewritten_by_plugin": "yes"}
        ],
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    let meta = &user.tool_result_meta.unwrap()[0];
    assert_eq!(meta.call_ended, None);
    assert_eq!(meta.rewritten_by_plugin, Some(true));

    let future: ToolCallEnded = serde_json::from_value(json!("abandoned")).unwrap();
    assert_eq!(future, ToolCallEnded::Unknown("abandoned".into()));
    assert_eq!(serde_json::to_value(&future).unwrap(), json!("abandoned"));
}

/// A `needs_person` remedy names the refused feature and what blocks it.
#[test]
fn tool_result_remedy_needs_person_with_blocker() {
    let user = user(json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "refused", "is_error": true},
                {"type": "tool_result", "tool_use_id": "toolu_2", "content": "retry later", "is_error": true}
            ]
        },
        "parent_tool_use_id": null,
        "tool_result_meta": [
            {"id": "toolu_1", "remedy": {"kind": "needs_person", "feature": "artifacts", "blocker": "publish_limit"}},
            {"id": "toolu_2", "remedy": {"kind": "advice_to_claude"}}
        ],
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    }));
    let meta = user.tool_result_meta.as_ref().unwrap();
    let remedy = meta[0].remedy.as_ref().unwrap();
    assert_eq!(remedy.kind, ToolResultRemedyKind::NeedsPerson);
    assert_eq!(remedy.feature, Some(RemedyFeature::Artifacts));
    assert_eq!(remedy.blocker, Some(RemedyBlocker::PublishLimit));
    assert_eq!(
        meta[1].remedy.as_ref().unwrap().kind,
        ToolResultRemedyKind::AdviceToClaude
    );

    for wire in [
        "switched_off",
        "publish_limit",
        "no_edit_access",
        "owner_only",
        "other_org",
        "network_off",
        "restricted_sandbox",
        "something_new",
    ] {
        let parsed: RemedyBlocker = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed.as_str(), wire);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

fn verification_frame(params: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "Verify your account to continue"}],
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
        "error": "verification_required",
        "api_error_params": params
    })
}

/// An account hold on `verification_required` carries its wording, whether
/// an admin must act, and where to go.
#[test]
fn assistant_verification_required_hold_params() {
    let frame = verification_frame(json!({
        "hold_wording": "owner_must_verify",
        "hold_admin_required": true,
        "hold_url": "https://example.com/verify"
    }));
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    let params = msg.api_error_params.expect("api_error_params");
    assert_eq!(
        params.hold_wording,
        Some(ApiErrorHoldWording::OwnerMustVerify)
    );
    assert_eq!(params.hold_admin_required, Some(true));
    assert_eq!(
        params.hold_url.as_deref(),
        Some("https://example.com/verify")
    );
}

/// The hold fields are declared `.catch(undefined)`: malformed reads as
/// absent.
#[test]
fn assistant_malformed_hold_params_read_as_absent() {
    let frame = verification_frame(json!({
        "hold_wording": 3,
        "hold_admin_required": "yes",
        "hold_url": false
    }));
    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    let params = msg.api_error_params.expect("api_error_params");
    assert_eq!(params.hold_wording, None);
    assert_eq!(params.hold_admin_required, None);
    assert_eq!(params.hold_url, None);

    for wire in ["verify_now", "owner_must_verify", "contact", "later"] {
        let parsed: ApiErrorHoldWording = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed.as_str(), wire);
    }
}

/// The live wrapping audit against 2.1.332 saw `overageEnabled`, which the
/// CLI's rate-limit schema already declared.
#[test]
fn rate_limit_info_carries_overage_enabled() {
    let frame = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": "allowed_warning",
            "resetsAt": 1795766400,
            "rateLimitType": "seven_day",
            "isUsingOverage": false,
            "overageEnabled": true,
            "overageStatus": "allowed",
            "overageResetsAt": 1796083200,
            "surpassedThreshold": 0.75,
            "utilization": 0.8
        },
        "uuid": "9cb44fbf-7736-41aa-8e1e-25357ed2c5d4",
        "session_id": "064026a0-aab1-4db1-8185-766d408123c0"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::RateLimitEvent(evt) = serde_json::from_value(frame).unwrap() else {
        panic!("expected RateLimitEvent");
    };
    assert_eq!(evt.rate_limit_info.overage_enabled, Some(true));
}
