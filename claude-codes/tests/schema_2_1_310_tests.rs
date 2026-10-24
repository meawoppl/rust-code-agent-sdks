//! Coverage for the CLI 2.1.310 stream-json drift.
//!
//! 2.1.309 → 2.1.310 added two optional fields, no new message types and no
//! removals: each `rate_limit_info.unifiedWindows` entry now carries the
//! server's per-window `status` and `surpassedThreshold`.
//!
//! The frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::io::RateLimitStatus;
use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

/// A rate limit event reports a per-window status and threshold, which can
/// differ from the top-level status of the currently limiting window.
#[test]
fn unified_windows_carry_status_and_surpassed_threshold() {
    let frame = json!({
        "type": "rate_limit_event",
        "session_id": "s1",
        "uuid": "u1",
        "rate_limit_info": {
            "status": "allowed",
            "resetsAt": 1788441600u64,
            "rateLimitType": "five_hour",
            "isUsingOverage": false,
            "unifiedWindows": {
                "five_hour": {
                    "resetsAt": 1788441600u64,
                    "utilization": 0.3,
                    "status": "allowed"
                },
                "seven_day": {
                    "resetsAt": 1788872400u64,
                    "utilization": 0.82,
                    "status": "allowed_warning",
                    "surpassedThreshold": 0.75
                },
                "seven_day_overage_included": {
                    "resetsAt": 1788872400u64,
                    "utilization": 1.02,
                    "status": "rejected",
                    "surpassedThreshold": 1.0
                }
            }
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::RateLimitEvent(evt) = serde_json::from_value(frame).unwrap() else {
        panic!("expected rate_limit_event");
    };
    let windows = evt.rate_limit_info.unified_windows.unwrap();
    let five_hour = windows.five_hour.unwrap();
    assert_eq!(five_hour.status, Some(RateLimitStatus::Allowed));
    assert_eq!(five_hour.surpassed_threshold, None);
    let seven_day = windows.seven_day.unwrap();
    assert_eq!(seven_day.status, Some(RateLimitStatus::AllowedWarning));
    assert_eq!(seven_day.surpassed_threshold, Some(0.75));
    let overage = windows.seven_day_overage_included.unwrap();
    assert_eq!(overage.status, Some(RateLimitStatus::Rejected));
    assert_eq!(overage.surpassed_threshold, Some(1.0));
}

/// Windows from older CLIs, without the new fields, still deserialize.
#[test]
fn unified_window_without_status_still_parses() {
    let frame = json!({
        "type": "rate_limit_event",
        "session_id": "s1",
        "rate_limit_info": {
            "status": "allowed",
            "unifiedWindows": {
                "five_hour": {"resetsAt": 1788441600u64, "utilization": 0.01}
            }
        }
    });

    let ClaudeOutput::RateLimitEvent(evt) = serde_json::from_value(frame).unwrap() else {
        panic!("expected rate_limit_event");
    };
    let five_hour = evt
        .rate_limit_info
        .unified_windows
        .unwrap()
        .five_hour
        .unwrap();
    assert_eq!(five_hour.status, None);
    assert_eq!(five_hour.surpassed_threshold, None);
}
