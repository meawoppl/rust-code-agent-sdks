//! Coverage for the CLI 2.1.276 stream-json drift.
//!
//! 2.1.274 → 2.1.276 added `decision_reason_code` on
//! `system/permission_denied` and `remedy` on each `tool_result_meta` entry
//! of a `user` frame, and made that entry's `non_execution_kind` optional
//! (an entry may now carry a remedy alone, on a result that did run).
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, PermissionDeniedReasonCode,
    RemedyFeature, RemedyFeatureCause, RemedyLoginProvider, RemedyPolicyKind, SystemSubtype,
    ToolResultRemedyKind,
};
use serde_json::json;

/// A permission denial carrying the new actionable code round-trips fully
/// wrapped and decodes to the typed variant.
#[test]
fn permission_denied_decision_reason_code_fully_wrapped() {
    let frame = json!({
        "type": "system",
        "subtype": "permission_denied",
        "tool_name": "Read",
        "tool_use_id": "toolu_1",
        "decision_reason_type": "other",
        "decision_reason_code": "outside_reads_blocked",
        "decision_reason": "Reads outside the working directories are blocked",
        "message": "Permission denied",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::PermissionDenied);
    let Some(KnownSystemEvent::PermissionDenied(denied)) = sys.as_known_system_event() else {
        panic!("expected PermissionDenied view");
    };
    assert_eq!(
        denied.decision_reason_code,
        Some(PermissionDeniedReasonCode::OutsideReadsBlocked)
    );
}

/// A code this crate version does not know stays readable as `Unknown`
/// rather than failing the frame — the CLI documents new values as additive.
#[test]
fn permission_denied_unknown_reason_code_is_open_set() {
    let frame = json!({
        "type": "system",
        "subtype": "permission_denied",
        "tool_name": "Bash",
        "tool_use_id": "toolu_2",
        "decision_reason_code": "some_future_code",
        "message": "Permission denied",
        "uuid": "u2",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::PermissionDenied(denied)) = sys.as_known_system_event() else {
        panic!("expected PermissionDenied view");
    };
    assert_eq!(
        denied.decision_reason_code,
        Some(PermissionDeniedReasonCode::Unknown(
            "some_future_code".into()
        ))
    );
    assert_eq!(
        denied.decision_reason_code.unwrap().as_str(),
        "some_future_code"
    );
}

/// A user frame whose tool_result_meta entries carry every remedy shape —
/// one alongside a non-execution kind, one alone on a result that ran —
/// round-trips fully wrapped.
#[test]
fn user_tool_result_meta_remedy_fully_wrapped() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "MCP server needs auth", "is_error": true},
                {"type": "tool_result", "tool_use_id": "toolu_2", "content": "written", "is_error": false},
                {"type": "tool_result", "tool_use_id": "toolu_3", "content": "feature off", "is_error": true},
                {"type": "tool_result", "tool_use_id": "toolu_4", "content": "denied", "is_error": true}
            ]
        },
        "parent_tool_use_id": null,
        "session_id": "1d1c4a2e-6f1b-4b7e-9c3d-2e5f6a7b8c9d",
        "uuid": "3f8b2c1a-9d4e-4f6a-8b7c-1a2b3c4d5e6f",
        "tool_result_meta": [
            {
                "id": "toolu_1",
                "non_execution_kind": "permission-rule",
                "remedy": {"kind": "mcp_needs_auth", "servers": ["github", "linear"]}
            },
            {
                "id": "toolu_2",
                "remedy": {"kind": "staged_for_review", "path": "/home/u/.claude/settings.json"}
            },
            {
                "id": "toolu_3",
                "remedy": {"kind": "feature_disabled", "feature": "workflows", "cause": "managed_settings"}
            },
            {
                "id": "toolu_4",
                "remedy": {
                    "kind": "policy_denied",
                    "feature": "artifacts",
                    "policy_kind": "latched",
                    "provider": "claude_design",
                    "managed": true
                }
            }
        ]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    let meta = user.tool_result_meta.expect("tool_result_meta");
    assert_eq!(meta.len(), 4);

    assert_eq!(
        meta[0].non_execution_kind.as_deref(),
        Some("permission-rule")
    );
    let remedy = meta[0].remedy.as_ref().unwrap();
    assert_eq!(remedy.kind, ToolResultRemedyKind::McpNeedsAuth);
    assert_eq!(
        remedy.servers.as_deref(),
        Some(&["github".to_string(), "linear".to_string()][..])
    );

    assert_eq!(meta[1].non_execution_kind, None);
    let remedy = meta[1].remedy.as_ref().unwrap();
    assert_eq!(remedy.kind, ToolResultRemedyKind::StagedForReview);
    assert_eq!(
        remedy.path.as_deref(),
        Some("/home/u/.claude/settings.json")
    );

    let remedy = meta[2].remedy.as_ref().unwrap();
    assert_eq!(remedy.kind, ToolResultRemedyKind::FeatureDisabled);
    assert_eq!(remedy.feature, Some(RemedyFeature::Workflows));
    assert_eq!(remedy.cause, Some(RemedyFeatureCause::ManagedSettings));

    let remedy = meta[3].remedy.as_ref().unwrap();
    assert_eq!(remedy.kind, ToolResultRemedyKind::PolicyDenied);
    assert_eq!(remedy.feature, Some(RemedyFeature::Artifacts));
    assert_eq!(remedy.policy_kind, Some(RemedyPolicyKind::Latched));
    assert_eq!(remedy.provider, Some(RemedyLoginProvider::ClaudeDesign));
    assert_eq!(remedy.managed, Some(true));
}

/// An unknown remedy kind is preserved verbatim, as the CLI asks hosts to
/// treat unknown kinds as "no remedy" rather than as a parse failure.
#[test]
fn unknown_remedy_kind_is_open_set() {
    let frame = json!({
        "type": "user",
        "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_9", "content": "x", "is_error": true}
        ]},
        "parent_tool_use_id": null,
        "session_id": "1d1c4a2e-6f1b-4b7e-9c3d-2e5f6a7b8c9d",
        "uuid": "7c6d5e4f-3a2b-4c1d-9e8f-0a1b2c3d4e5f",
        "tool_result_meta": [
            {"id": "toolu_9", "remedy": {"kind": "future_kind", "provider": "future_login"}}
        ]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    let remedy = user.tool_result_meta.unwrap().remove(0).remedy.unwrap();
    assert_eq!(
        remedy.kind,
        ToolResultRemedyKind::Unknown("future_kind".into())
    );
    assert_eq!(remedy.kind.as_str(), "future_kind");
    assert_eq!(
        remedy.provider,
        Some(RemedyLoginProvider::Unknown("future_login".into()))
    );
}
