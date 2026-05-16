//! Account-level rate limit notifications.
//!
//! Mirrors upstream's `codex-rs/app-server-protocol/src/protocol/v2/account.rs`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A rate-limit window descriptor used inside [`RateLimitSnapshot`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct RateLimitWindow {
    /// Unix timestamp (seconds) at which this rate-limit window resets.
    pub resets_at: i64,
    /// Percentage of the window already consumed (0-100).
    pub used_percent: i32,
    /// Length of the rate-limit window, in minutes.
    pub window_duration_mins: i64,
}

/// Rate-limit envelope sent in [`AccountRateLimitsUpdatedNotification`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct RateLimitSnapshot {
    /// Credit balance, if applicable for this plan. Shape is plan-dependent
    /// so the payload is preserved as raw JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits: Option<Value>,
    /// Stable machine identifier for the limit (e.g. `"codex"`).
    pub limit_id: String,
    /// Human-readable label, if the server provides one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_name: Option<String>,
    /// Plan tier (e.g. `"free"`, `"plus"`, `"team"`).
    pub plan_type: String,
    /// Primary (short-term) rate-limit window, if active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<RateLimitWindow>,
    /// Secondary (longer-term) rate-limit window, if active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary: Option<RateLimitWindow>,
    /// Which window (if any) the account has already hit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_reached_type: Option<String>,
}

/// `account/rateLimits/updated` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "integration-tests", serde(deny_unknown_fields))]
pub struct AccountRateLimitsUpdatedNotification {
    pub rate_limits: RateLimitSnapshot,
}
