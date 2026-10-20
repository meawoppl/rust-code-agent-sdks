//! Coverage for the CLI 2.1.304 stream-json drift.
//!
//! 2.1.303 → 2.1.304 added one optional nested object, no new message types
//! and no removals: `usageCreditOffer` on `rate_limit_event.rate_limit_info`.
//!
//! Each frame below carries the new field and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput, UsageCreditOfferKind};
use serde_json::json;

/// A rejected rate-limit frame from a VS Code extension session carries the
/// account's limited-time usage-credit offer.
#[test]
fn rate_limit_info_carries_usage_credit_offer() {
    let frame = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": "rejected",
            "resetsAt": 1700003600,
            "rateLimitType": "five_hour",
            "errorCode": "credits_required",
            "canUserPurchaseCredits": true,
            "usageCreditOffer": {
                "kind": "extra_credit",
                "headlinePct": 25,
                "daysLeft": 6,
                "label": "25% extra usage credit, 6 days left"
            }
        },
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::RateLimitEvent(evt) = serde_json::from_value(frame).unwrap() else {
        panic!("expected RateLimitEvent");
    };
    let offer = evt.rate_limit_info.usage_credit_offer.unwrap();
    assert_eq!(offer.kind, UsageCreditOfferKind::ExtraCredit);
    assert_eq!(offer.headline_pct, 25);
    assert_eq!(offer.days_left, 6);
    assert_eq!(offer.label, "25% extra usage credit, 6 days left");
}
