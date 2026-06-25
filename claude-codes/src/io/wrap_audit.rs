//! Wire-fidelity ("fully wrapped") auditing for [`ClaudeOutput`] frames.
//!
//! A frame is **fully wrapped** when the typed model captures every field the
//! CLI put on the wire — nothing is silently dropped, and nothing of substance
//! is left sitting in an untyped `serde_json::Value` escape hatch.
//!
//! [`audit_frame`] checks three things for a raw frame:
//!
//! 1. It deserializes into a concrete [`ClaudeOutput`] variant at all.
//! 2. Re-serializing that typed value reproduces every wire field (a lossless
//!    round-trip), so no field was quietly dropped by the typed struct.
//! 3. For `system` messages — whose non-`subtype` fields are otherwise absorbed
//!    by the catch-all [`SystemMessage::data`](crate::io::SystemMessage) —
//!    the `subtype` is one this crate models *and* its dedicated typed view
//!    round-trips losslessly against `data`.
//!
//! This is most useful for subagent sessions, where the CLI emits
//! `task_started` / `task_updated` / `task_notification` / `thinking_tokens`
//! system frames carrying token-accounting fields that downstream consumers
//! need without poking at raw JSON.

use serde_json::Value;

use super::message_types::SystemSubtype;
use super::ClaudeOutput;

/// The result of auditing a single raw frame for full typed coverage.
#[derive(Debug, Clone)]
pub struct FrameAudit {
    /// The frame's `type` (e.g. `system`, `assistant`, `result`), or the raw
    /// `type` string when the frame failed to deserialize.
    pub message_type: String,
    /// `true` when the typed model captures every wire field with no escape
    /// hatch absorbing data.
    pub fully_wrapped: bool,
    /// Human-readable description of every wrapping gap found. Empty iff
    /// [`FrameAudit::fully_wrapped`] is `true`.
    pub issues: Vec<String>,
}

/// Audit a single raw frame (one parsed JSONL line) for full typed coverage.
///
/// See the [module docs](self) for what "fully wrapped" means.
pub fn audit_frame(raw: &Value) -> FrameAudit {
    let mut issues = Vec::new();

    // 1. Must deserialize into a concrete typed variant.
    let parsed: ClaudeOutput = match serde_json::from_value(raw.clone()) {
        Ok(parsed) => parsed,
        Err(e) => {
            let message_type = raw
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>")
                .to_string();
            return FrameAudit {
                message_type,
                fully_wrapped: false,
                issues: vec![format!(
                    "does not deserialize into a typed ClaudeOutput: {e}"
                )],
            };
        }
    };
    let message_type = parsed.message_type();

    // 2. Top-level round-trip must not drop any wire field.
    match serde_json::to_value(&parsed) {
        Ok(reserialized) => diff_lost(raw, &reserialized, "", &mut issues),
        Err(e) => issues.push(format!("typed value failed to re-serialize: {e}")),
    }

    // 3. System frames hide their payload behind the `data` catch-all, so a
    //    top-level round-trip can't see field drops. Require a known subtype
    //    whose dedicated struct round-trips losslessly against `data`.
    if let ClaudeOutput::System(sys) = &parsed {
        match (&sys.subtype, sys.typed_value()) {
            (SystemSubtype::Unknown(s), _) => issues.push(format!(
                "system subtype '{s}' is not modeled — its fields stay in an untyped Value"
            )),
            (_, Some(typed)) => diff_lost(&sys.data, &typed, "", &mut issues),
            (subtype, None) => issues.push(format!(
                "system subtype '{subtype}' has no dedicated typed view"
            )),
        }
    }

    FrameAudit {
        message_type,
        fully_wrapped: issues.is_empty(),
        issues,
    }
}

/// Panic with a detailed report unless `raw` is fully wrapped.
pub fn assert_fully_wrapped(raw: &Value) {
    let audit = audit_frame(raw);
    assert!(
        audit.fully_wrapped,
        "frame (type={}) is not fully wrapped:\n  - {}\nraw frame: {}",
        audit.message_type,
        audit.issues.join("\n  - "),
        raw,
    );
}

/// `true` for wire values that hold no information — `null` or an empty
/// array/object — which a typed model may legitimately omit on serialize.
fn carries_no_data(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// Record every place where `wire` carries data that `typed` (a typed
/// re-serialization) lost.
///
/// Only *losses* are reported. Keys the typed model adds that the wire omitted
/// (serde defaults like `permission_denials: []`) and wire `null`s that
/// serialize away (`skip_serializing_if = "Option::is_none"`) are not data
/// loss and are ignored.
fn diff_lost(wire: &Value, typed: &Value, path: &str, out: &mut Vec<String>) {
    match (wire, typed) {
        (Value::Object(w), Value::Object(t)) => {
            for (key, wire_val) in w {
                let child = format!("{path}/{key}");
                match t.get(key) {
                    Some(typed_val) => diff_lost(wire_val, typed_val, &child, out),
                    // A wire `null` or empty collection carries no data, so a
                    // `skip_serializing_if` omission of it is not a loss.
                    None if carries_no_data(wire_val) => {}
                    None => out.push(format!(
                        "field `{child}` present on the wire but dropped by the typed model"
                    )),
                }
            }
        }
        (Value::Array(w), Value::Array(t)) => {
            if w.len() != t.len() {
                out.push(format!(
                    "array `{path}` has {} element(s) on the wire but {} after typed round-trip",
                    w.len(),
                    t.len()
                ));
            } else {
                for (i, (wv, tv)) in w.iter().zip(t.iter()).enumerate() {
                    diff_lost(wv, tv, &format!("{path}/{i}"), out);
                }
            }
        }
        _ => {
            if wire != typed {
                out.push(format!(
                    "value at `{path}` changed on typed round-trip (wire={wire}, typed={typed})"
                ));
            }
        }
    }
}
