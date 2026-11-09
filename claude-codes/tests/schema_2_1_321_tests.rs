//! Coverage for the CLI 2.1.321 stream-json drift.
//!
//! 2.1.319 → 2.1.321 added the plugin notification pushes
//! `system/ui_notify` and `system/ui_notify_cancel`,
//! `tool_pool_build_count` and `first_request_cache_read_input_tokens` on the
//! success result, `artifact_publish` on an assistant `tool_use_meta` entry,
//! the `hook_denied` `api_error` kind and `api_error_params.cut`. The model
//! registry gained Fable 5.2 and Mythos 5.2. No removals.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ApiErrorCut, ClaudeModel, ClaudeOutput, KnownSystemEvent,
};
use serde_json::json;

/// A scheduled plugin notification carries its key, timing and the
/// `rewritten` flag.
#[test]
fn ui_notify_scheduled() {
    let frame = json!({
        "type": "system",
        "subtype": "ui_notify",
        "plugin": "reminders",
        "source": "reminders@community",
        "title": "Stand-up",
        "text": "Stand-up starts in five minutes",
        "key": "standup",
        "delay_ms": 300000,
        "until_back": true,
        "rewritten": false,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert!(sys.is_ui_notify());
    let n = sys.as_ui_notify().expect("typed accessor");
    assert_eq!(n.source, "reminders@community");
    assert_eq!(n.key.as_deref(), Some("standup"));
    assert_eq!(n.delay_ms, Some(300000));
    assert!(n.at.is_none());
    assert_eq!(n.until_back, Some(true));
    assert!(!n.rewritten);
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::UiNotify(_))
    ));
}

/// An immediate notification from an older engine omits `rewritten`, which
/// reads as false; one fixed on the clock uses `at`.
#[test]
fn ui_notify_minimal_and_at() {
    let frame = json!({
        "type": "system",
        "subtype": "ui_notify",
        "plugin": "reminders",
        "source": "reminders@inline",
        "title": "reminders",
        "text": "now",
        "uuid": "u1",
        "session_id": "s1"
    });
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let n = sys.as_ui_notify().expect("typed accessor");
    assert!(!n.rewritten);
    assert!(n.key.is_none() && n.delay_ms.is_none());

    let frame = json!({
        "type": "system",
        "subtype": "ui_notify",
        "plugin": "reminders",
        "source": "reminders@community",
        "title": "Lunch",
        "text": "Lunch",
        "at": 1_794_000_000_000u64,
        "rewritten": true,
        "uuid": "u2",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let n = sys.as_ui_notify().expect("typed accessor");
    assert_eq!(n.at, Some(1_794_000_000_000));
    assert!(n.rewritten);
}

/// A plugin's cancel names its key; the engine's removal of a plugin omits
/// it.
#[test]
fn ui_notify_cancel_with_and_without_key() {
    for key in [Some("standup"), None] {
        let mut frame = json!({
            "type": "system",
            "subtype": "ui_notify_cancel",
            "plugin": "reminders",
            "source": "reminders@community",
            "uuid": "u1",
            "session_id": "s1"
        });
        if let Some(k) = key {
            frame["key"] = json!(k);
        }
        assert_fully_wrapped(&frame);

        let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
            panic!("expected System");
        };
        assert!(sys.is_ui_notify_cancel());
        let c = sys.as_ui_notify_cancel().expect("typed accessor");
        assert_eq!(c.key.as_deref(), key);
        assert!(matches!(
            sys.as_known_system_event(),
            Some(KnownSystemEvent::UiNotifyCancel(_))
        ));
    }
}

/// The success result counts tool-list builds and splits the first
/// request's cache reads out of its input tokens.
#[test]
fn result_success_tool_pool_and_first_request_cache_read() {
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
        "process_cpu_ms_turn": 140,
        "tool_pool_build_count": 2,
        "request_sent_wall_ms": 1_794_000_000_000.5,
        "first_request_input_tokens": 52000,
        "first_request_cache_read_input_tokens": 0
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.tool_pool_build_count, Some(2));
    assert_eq!(res.first_request_input_tokens, Some(52000));
    assert_eq!(res.first_request_cache_read_input_tokens, Some(0));

    let wire = serde_json::to_value(&res).unwrap();
    assert_eq!(wire["tool_pool_build_count"], 2);
    assert_eq!(wire["first_request_cache_read_input_tokens"], 0);
}

/// An Artifacts publishing call is marked on its tool-use metadata.
#[test]
fn assistant_tool_use_meta_artifact_publish() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_01",
            "type": "message",
            "role": "assistant",
            "model": "claude-fable-5-2",
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "mcp__artifacts__publish",
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
            "display_name": "Publish artifact",
            "tool_name": "publish",
            "artifact_publish": true
        }]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.tool_use_meta[0].artifact_publish, Some(true));
}

/// A reply cut short by the context window says which stop reason cut it.
#[test]
fn assistant_api_error_cut() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_02",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "Response exceeded the context window"}],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {"input_tokens": 0, "output_tokens": 0}
        },
        "parent_tool_use_id": null,
        "session_id": "s1",
        "uuid": "u1",
        "is_api_error_message": true,
        "api_error": "max_output_tokens",
        "api_error_params": {"cut": "context_window"}
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    let params = msg.api_error_params.as_ref().expect("params");
    assert_eq!(params.cut, Some(ApiErrorCut::ContextWindow));
    assert_eq!(ApiErrorCut::from("output_limit"), ApiErrorCut::OutputLimit);
    assert_eq!(
        ApiErrorCut::from("future_cut"),
        ApiErrorCut::Unknown("future_cut".to_string())
    );
}

/// A request a mod's `http.fetch` hook stopped is a `hook_denied` API error.
#[test]
fn assistant_api_error_hook_denied() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_03",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "A mod's http.fetch hook stopped this request: blocked"}],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {"input_tokens": 0, "output_tokens": 0}
        },
        "parent_tool_use_id": null,
        "session_id": "s1",
        "uuid": "u1",
        "error": "invalid_request",
        "is_api_error_message": true,
        "api_error": "hook_denied"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.api_error.as_deref(), Some("hook_denied"));
}

/// Fable 5.2 and Mythos 5.2 round-trip through `ClaudeModel`.
#[test]
fn models_fable_and_mythos_5_2() {
    assert_eq!(ClaudeModel::from("claude-fable-5-2"), ClaudeModel::Fable52);
    assert_eq!(ClaudeModel::Fable52.display_name(), "Fable 5.2");
    assert_eq!(
        ClaudeModel::from("claude-mythos-5-2"),
        ClaudeModel::Mythos52
    );
    assert_eq!(ClaudeModel::Mythos52.display_name(), "Mythos 5.2");
    assert!(!ClaudeModel::Fable52.is_alias());
    assert!(ClaudeModel::known().contains(&ClaudeModel::Mythos52));
}
