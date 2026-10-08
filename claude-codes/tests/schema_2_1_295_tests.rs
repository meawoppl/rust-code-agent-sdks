//! Coverage for the CLI 2.1.295 stream-json drift.
//!
//! 2.1.293 → 2.1.295 added `api_message_id` and `text_runs_joined` on
//! `stream_event`, and `compact_output_chars` on `system/status`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

/// A `message_start` names its response and flags joined text runs.
#[test]
fn stream_event_carries_api_message_id_and_text_runs_joined() {
    let frame = json!({
        "type": "stream_event",
        "event": {"type": "message_start", "message": {"id": "msg_01"}},
        "parent_tool_use_id": null,
        "api_message_id": "msg_01",
        "text_runs_joined": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::StreamEvent(ev) = serde_json::from_value(frame).unwrap() else {
        panic!("expected StreamEvent");
    };
    assert_eq!(ev.api_message_id.as_deref(), Some("msg_01"));
    assert_eq!(ev.text_runs_joined, Some(true));
}

/// Later events of the same response carry only the id.
#[test]
fn stream_event_api_message_id_without_text_runs_joined() {
    let frame = json!({
        "type": "stream_event",
        "event": {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "hi"}},
        "parent_tool_use_id": null,
        "api_message_id": "msg_01",
        "uuid": "u2",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::StreamEvent(ev) = serde_json::from_value(frame).unwrap() else {
        panic!("expected StreamEvent");
    };
    assert_eq!(ev.api_message_id.as_deref(), Some("msg_01"));
    assert_eq!(ev.text_runs_joined, None);
}

/// An automatic compaction reports the summary's streamed length.
#[test]
fn status_carries_compact_output_chars() {
    let frame = json!({
        "type": "system",
        "subtype": "status",
        "status": "compacting",
        "compact_output_chars": 1234,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let status = sys.as_status().expect("status");
    assert_eq!(status.compact_output_chars, Some(1234));
}
