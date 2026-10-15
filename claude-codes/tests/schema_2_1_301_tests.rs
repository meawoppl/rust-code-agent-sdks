//! Coverage for the CLI 2.1.301 stream-json drift.
//!
//! 2.1.300 → 2.1.301 added optional fields only, no new types and no
//! removals: `refused_message_id` on `assistant`, and `original_cwd` plus
//! `additional_directories` on `system/init`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

/// The assistant message reporting a refusal names the refused response's
/// API id, distinct from its own local `message.id`.
#[test]
fn assistant_carries_refused_message_id() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "6f1d2c3e-4b5a-4c6d-8e7f-9a0b1c2d3e4f",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "I can't help with that."}],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {"input_tokens": 0, "output_tokens": 0}
        },
        "parent_tool_use_id": null,
        "session_id": "s1",
        "uuid": "u1",
        "refused_message_id": "msg_01refused"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.refused_message_id.as_deref(), Some("msg_01refused"));
}

/// Per-message usage carries the speed tier the request ran at (seen live
/// from the API alongside CLI 2.1.301).
#[test]
fn assistant_usage_carries_speed() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_02",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-5-5",
            "content": [{"type": "text", "text": "42"}],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {
                "input_tokens": 2,
                "output_tokens": 16,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
                "service_tier": "standard",
                "inference_geo": "global",
                "speed": "standard"
            }
        },
        "parent_tool_use_id": null,
        "session_id": "s1",
        "uuid": "u2"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    let usage = msg.message.usage.expect("usage");
    assert_eq!(usage.speed.as_deref(), Some("standard"));
}

/// `system/init` carries the permission-anchor folder and the added
/// directories.
#[test]
fn system_init_carries_original_cwd_and_additional_directories() {
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "cwd": "/repo/sub",
        "original_cwd": "/repo",
        "additional_directories": ["/data", "/shared/docs"]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    assert_eq!(init.original_cwd.as_deref(), Some("/repo"));
    assert_eq!(
        init.additional_directories,
        Some(vec!["/data".to_string(), "/shared/docs".to_string()])
    );
}

/// An empty `additional_directories` means "none", and stays distinct from an
/// absent field ("not told") through a round trip.
#[test]
fn system_init_additional_directories_empty_vs_absent() {
    let empty = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "additional_directories": []
    });
    assert_fully_wrapped(&empty);
    let ClaudeOutput::System(sys) = serde_json::from_value(empty).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    assert_eq!(init.additional_directories, Some(vec![]));
    assert_eq!(
        serde_json::to_value(&init).unwrap()["additional_directories"],
        json!([])
    );

    let absent = json!({"type": "system", "subtype": "init", "session_id": "s1"});
    let ClaudeOutput::System(sys) = serde_json::from_value(absent).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    assert_eq!(init.additional_directories, None);
    assert!(serde_json::to_value(&init)
        .unwrap()
        .get("additional_directories")
        .is_none());
}
