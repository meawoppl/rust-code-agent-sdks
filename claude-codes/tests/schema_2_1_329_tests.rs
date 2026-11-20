//! Coverage for the CLI 2.1.329 stream-json drift.
//!
//! 2.1.328 → 2.1.329 added `user_message_from_human` on
//! `conversation_reset`, `served_by_anthropic` on `system/init`,
//! `client_content_width` on `user`, and the `oauth_refresh_store_refused`
//! `api_error` kind.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

/// A `/clear` sent by a person from another client marks the reset as theirs.
#[test]
fn conversation_reset_carries_user_message_from_human() {
    let frame = json!({
        "type": "conversation_reset",
        "new_conversation_id": "c2",
        "uuid": "u1",
        "session_id": "s1",
        "trigger": "clear",
        "user_message_uuid": "6f1d2c3e-4b5a-4c6d-8e7f-9a0b1c2d3e4f",
        "user_message_from_human": true,
        "timestamp": "2026-11-19T08:00:00.000Z"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::ConversationReset(reset) = serde_json::from_value(frame).unwrap() else {
        panic!("expected ConversationReset");
    };
    assert_eq!(reset.user_message_from_human, Some(true));
}

/// `system/init` reports whether model requests go to Anthropic's own API.
#[test]
fn system_init_carries_served_by_anthropic() {
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "analytics_disabled": false,
        "served_by_anthropic": true
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    assert_eq!(init.served_by_anthropic, Some(true));
}

/// A user message carries the sending client's chat-column width.
#[test]
fn user_carries_client_content_width() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": "hello"}]
        },
        "parent_tool_use_id": null,
        "client_platform": "web_claude_ai",
        "client_content_width": 720.5,
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(user.client_content_width, Some(720.5));
}

/// The new `oauth_refresh_store_refused` kind passes through `api_error`.
#[test]
fn assistant_api_error_oauth_refresh_store_refused() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "m1",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "Keychain refused the write."}],
            "stop_reason": "stop_sequence",
            "stop_sequence": "",
            "usage": {"input_tokens": 0, "output_tokens": 0}
        },
        "parent_tool_use_id": null,
        "is_api_error_message": true,
        "api_error": "oauth_refresh_store_refused",
        "uuid": "u1",
        "session_id": "s1"
    });

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(
        msg.api_error.as_deref(),
        Some("oauth_refresh_store_refused")
    );
}
