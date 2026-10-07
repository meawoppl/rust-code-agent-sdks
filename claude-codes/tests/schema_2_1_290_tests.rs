//! Coverage for the CLI 2.1.290 stream-json drift.
//!
//! 2.1.289 → 2.1.290 added the `system/file_attachments_missing` subtype,
//! `resume_store_confirm_detail` on `result`, and `refused_message_id` on
//! `user`. (Its `system_prompt_detail` on `result` was removed again in
//! 2.1.293.)
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, SystemSubtype};
use serde_json::json;

/// The new subtype parses through both the accessor and the typed event view.
#[test]
fn file_attachments_missing_is_typed() {
    let frame = json!({
        "type": "system",
        "subtype": "file_attachments_missing",
        "message_uuid": "m1",
        "sent_count": 3,
        "missing": [
            {"file_uuid": "f1", "reason": "too_large"},
            {"file_uuid": "f2", "reason": "some_future_code"}
        ],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::FileAttachmentsMissing);
    assert!(sys.is_file_attachments_missing());
    let missing = sys.as_file_attachments_missing().expect("typed accessor");
    assert_eq!(missing.message_uuid, "m1");
    assert_eq!(missing.sent_count, 3);
    assert_eq!(missing.missing.len(), 2);
    assert_eq!(missing.missing[0].file_uuid, "f1");
    assert_eq!(missing.missing[0].reason, "too_large");
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::FileAttachmentsMissing(_))
    ));
}

/// `result` carries how the `resume_store_confirm` phase was spent.
#[test]
fn result_carries_resume_store_confirm_detail() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 1200,
        "duration_api_ms": 900,
        "num_turns": 1,
        "result": "done",
        "stop_reason": "end_turn",
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.01,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 38,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0
        },
        "modelUsage": {},
        "permission_denials": [],
        "time_to_request_ms": 140,
        "time_to_request_phases_ms": {"resume_store_confirm": 90, "system_prompt": 50},
        "resume_store_confirm_detail": {
            "write_ms": 10,
            "ahead_ms": 30,
            "post_ms": 45,
            "other_ms": 5,
            "posts": 3,
            "failed_posts": 1,
            "rows_ahead": 12,
            "rows": 4
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    let store = res.resume_store_confirm_detail.expect("store detail");
    assert_eq!(
        store.write_ms + store.ahead_ms + store.post_ms + store.other_ms,
        90
    );
    assert_eq!(store.failed_posts, 1);
    assert_eq!(store.rows_ahead, 12);
}

/// The synthetic re-ask after a refusal names the refused response.
#[test]
fn user_carries_refused_message_id() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": "Please try again."}]
        },
        "parent_tool_use_id": null,
        "isSynthetic": true,
        "refused_message_id": "msg_01ABC",
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(user.refused_message_id.as_deref(), Some("msg_01ABC"));
}
