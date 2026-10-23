//! Coverage for the CLI 2.1.309 stream-json drift.
//!
//! 2.1.308 → 2.1.309 added one optional field, no new message types and no
//! removals: `model_usage_since_last_result` on `result` frames (both the
//! `success` and the error subtypes), keyed and shaped like `modelUsage`.
//!
//! Each output frame below carries the new field and is asserted **fully
//! wrapped** — the typed model captures every wire field with nothing left in
//! an untyped escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

/// A success result states the per-model usage counted since the previous
/// result alongside the cumulative `modelUsage`.
#[test]
fn success_result_carries_model_usage_since_last_result() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 5000,
        "duration_api_ms": 4500,
        "num_turns": 2,
        "result": "Done",
        "stop_reason": "end_turn",
        "total_cost_usd": 0.09,
        "modelUsage": {
            "claude-opus-4-8": {
                "inputTokens": 5000,
                "outputTokens": 40,
                "cacheReadInputTokens": 0,
                "cacheCreationInputTokens": 0,
                "costUSD": 0.09,
                "webSearchRequests": 0,
                "contextWindow": 200000,
                "maxOutputTokens": 32000
            }
        },
        "model_usage_since_last_result": {
            "claude-opus-4-8": {
                "inputTokens": 1200,
                "outputTokens": 26,
                "cacheReadInputTokens": 0,
                "cacheCreationInputTokens": 0,
                "costUSD": 0.03,
                "webSearchRequests": 0,
                "contextWindow": 200000,
                "maxOutputTokens": 32000
            }
        },
        "permission_denials": [],
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    let total = &res.model_usage.as_ref().unwrap()["claude-opus-4-8"];
    let delta = &res.model_usage_since_last_result.as_ref().unwrap()["claude-opus-4-8"];
    assert_eq!(total.input_tokens, 5000);
    assert_eq!(delta.input_tokens, 1200);
    assert_eq!(delta.output_tokens, 26);
    assert_eq!(delta.cost_usd, 0.03);
}

/// An empty map means nothing was counted, which is distinct from absent.
#[test]
fn empty_model_usage_since_last_result_is_not_absent() {
    let frame = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "duration_ms": 10,
        "duration_api_ms": 0,
        "num_turns": 0,
        "stop_reason": null,
        "total_cost_usd": 0.0,
        "modelUsage": {},
        "model_usage_since_last_result": {},
        "permission_denials": [],
        "errors": ["boom"],
        "uuid": "u2",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame.clone()).unwrap() else {
        panic!("expected Result");
    };
    assert!(res
        .model_usage_since_last_result
        .as_ref()
        .is_some_and(|m| m.is_empty()));
    let round_trip = serde_json::to_value(&res).unwrap();
    assert_eq!(round_trip["model_usage_since_last_result"], json!({}));

    let mut older = frame;
    older
        .as_object_mut()
        .unwrap()
        .remove("model_usage_since_last_result");
    let ClaudeOutput::Result(res) = serde_json::from_value(older).unwrap() else {
        panic!("expected Result");
    };
    assert!(res.model_usage_since_last_result.is_none());
}
