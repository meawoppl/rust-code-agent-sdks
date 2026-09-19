//! Coverage for the CLI 2.1.278 stream-json drift.
//!
//! 2.1.276 → 2.1.278 added `scratchpad_path` on `system/init`, `staged_files`
//! and `no_query_first` on `system/turn_handoff_available`, `initiator` and
//! `pasted_content` on `user` frames, `builtin` on each slash-command row,
//! and seven more timing fields on `result`.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, StreamPostQueuedBehind, SystemSubtype,
};
use serde_json::json;

/// `system/init` carries the session's scratchpad directory.
#[test]
fn system_init_carries_scratchpad_path() {
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "scratchpad_path": "/tmp/claude-1000/-home-u-proj/s1/scratchpad"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    assert_eq!(
        init.scratchpad_path.as_deref(),
        Some("/tmp/claude-1000/-home-u-proj/s1/scratchpad")
    );
}

/// The two new capability markers on `system/turn_handoff_available` are
/// typed rather than landing in `extra`.
#[test]
fn turn_handoff_available_carries_staged_files_and_no_query_first() {
    let frame = json!({
        "type": "system",
        "subtype": "turn_handoff_available",
        "v": 1,
        "tools": ["Bash"],
        "worker_epoch": 4,
        "staged_files": true,
        "no_query_first": true,
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::TurnHandoffAvailable);
    let handoff = sys.as_turn_handoff_available().expect("typed accessor");
    assert_eq!(handoff.staged_files, Some(true));
    assert_eq!(handoff.no_query_first, Some(true));
    assert!(handoff.extra.is_empty());
}

/// A `user` frame carrying the host's `initiator` label and `pasted_content`
/// (a string entry and a content-block-array entry) round-trips fully wrapped.
#[test]
fn user_carries_initiator_and_pasted_content() {
    let frame = json!({
        "type": "user",
        "message": {"role": "user", "content": [{"type": "text", "text": "summarize this"}]},
        "parent_tool_use_id": null,
        "session_id": "1d1c4a2e-6f1b-4b7e-9c3d-2e5f6a7b8c9d",
        "uuid": "3f8b2c1a-9d4e-4f6a-8b7c-1a2b3c4d5e6f",
        "initiator": "scheduled-task",
        "pasted_content": [
            "fn main() {}",
            [{"type": "text", "text": "second paste"}]
        ]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    assert_eq!(user.initiator.as_deref(), Some("scheduled-task"));
    let pasted = user.pasted_content.expect("pasted_content");
    assert_eq!(pasted.len(), 2);
    assert_eq!(pasted[0], "fn main() {}");
    assert_eq!(pasted[1][0]["text"], "second paste");
}

/// A slash-command row marks Claude Code's own commands with `builtin`.
#[test]
fn commands_changed_carries_builtin_marker() {
    let frame = json!({
        "type": "system",
        "subtype": "commands_changed",
        "commands": [
            {"name": "usage", "description": "Show usage", "argumentHint": "", "aliases": ["cost"], "builtin": true},
            {"name": "deploy", "description": "Ship it", "argumentHint": "<env>"}
        ],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::CommandsChanged(changed)) = sys.as_known_system_event() else {
        panic!("expected CommandsChanged view");
    };
    assert_eq!(changed.commands[0].builtin, Some(true));
    assert_eq!(changed.commands[1].builtin, None);
}

fn result_frame(queued_behind: &str) -> serde_json::Value {
    json!({
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
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "request_sent_wall_ms": 1753212345678.25,
        "first_stream_post_queue_wait_ms": 7,
        "first_stream_post_queued_behind": queued_behind,
        "frame_received_wall_ms": 1753212345600.5,
        "frame_enqueued_wall_ms": 1753212345601.0,
        "turn_started_wall_ms": 1753212345602.75,
        "first_text_post_ms": 410,
        "first_text_post_wall_ms": 1753212346012.5
    })
}

/// The 2.1.278 timing instrumentation on `result` is typed and round-trips.
#[test]
fn result_carries_frame_and_text_post_timings() {
    let frame = result_frame("durable_post");
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.first_stream_post_queue_wait_ms, Some(7));
    assert_eq!(
        res.first_stream_post_queued_behind,
        Some(StreamPostQueuedBehind::DurablePost)
    );
    assert_eq!(res.frame_received_wall_ms, Some(1753212345600.5));
    assert_eq!(res.frame_enqueued_wall_ms, Some(1753212345601.0));
    assert_eq!(res.turn_started_wall_ms, Some(1753212345602.75));
    assert_eq!(res.first_text_post_ms, Some(410));
    assert_eq!(res.first_text_post_wall_ms, Some(1753212346012.5));
}

/// Every known `first_stream_post_queued_behind` value maps to its variant,
/// and a value this crate version does not know stays readable as `Unknown`.
#[test]
fn stream_post_queued_behind_is_open_set() {
    for (wire, expected) in [
        ("durable_post", StreamPostQueuedBehind::DurablePost),
        ("ephemeral_post", StreamPostQueuedBehind::EphemeralPost),
        ("retry_backoff", StreamPostQueuedBehind::RetryBackoff),
        ("hold", StreamPostQueuedBehind::Hold),
        ("none", StreamPostQueuedBehind::None),
        (
            "some_future_value",
            StreamPostQueuedBehind::Unknown("some_future_value".into()),
        ),
    ] {
        let frame = result_frame(wire);
        assert_fully_wrapped(&frame);
        let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
            panic!("expected Result");
        };
        assert_eq!(res.first_stream_post_queued_behind, Some(expected));
    }
}
