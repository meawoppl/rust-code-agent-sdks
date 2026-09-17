//! Coverage for the CLI 2.1.274 stream-json additive drift.
//!
//! 2.1.273 → 2.1.274 added `api_error_code` and `api_error_params` on
//! `assistant`, `api_error_code` on `result/success`, and
//! `startup_failure_reason` on the error `result` variants. All additive (no
//! removals or required/optional flips).
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::io::ResultSubtype;
use claude_codes::{
    assert_fully_wrapped, ApiErrorParams, ApiErrorProvider, ApiErrorRemedy, ClaudeOutput,
    StartupFailureReason,
};
use serde_json::json;

/// A synthetic API-error assistant message carrying the server gate code and
/// the credential-failure parameters round-trips fully wrapped.
#[test]
fn assistant_api_error_code_and_params_fully_wrapped() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "Bedrock credentials expired"}],
            "stop_reason": null,
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            }
        },
        "session_id": "s1",
        "uuid": "u1",
        "parent_tool_use_id": null,
        "is_api_error_message": true,
        "api_error_status": 401,
        "api_error": "provider_credentials",
        "api_error_code": "bedrock_token_expired",
        "api_error_params": {
            "provider": "bedrock",
            "remedy": "refresh_command"
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.is_api_error_message, Some(true));
    assert_eq!(msg.api_error.as_deref(), Some("provider_credentials"));
    assert_eq!(msg.api_error_code.as_deref(), Some("bedrock_token_expired"));

    let params = msg.api_error_params.expect("api_error_params");
    assert_eq!(params.provider, Some(ApiErrorProvider::Bedrock));
    assert_eq!(params.remedy, Some(ApiErrorRemedy::RefreshCommand));
    assert!(params.effort.is_none());
}

/// `effort_requires_thinking` carries only `effort`; providers and remedies
/// this crate does not know yet survive as `Unknown` and re-serialize
/// verbatim.
#[test]
fn api_error_params_effort_and_unknown_values_round_trip() {
    let effort_only: ApiErrorParams = serde_json::from_value(json!({"effort": "max"})).unwrap();
    assert_eq!(effort_only.effort.as_deref(), Some("max"));
    assert!(effort_only.provider.is_none());
    assert!(effort_only.remedy.is_none());
    assert_eq!(
        serde_json::to_value(&effort_only).unwrap(),
        json!({"effort": "max"})
    );

    let unknown: ApiErrorParams = serde_json::from_value(json!({
        "provider": "someNewProvider",
        "remedy": "some_new_remedy"
    }))
    .unwrap();
    assert_eq!(
        unknown.provider,
        Some(ApiErrorProvider::Unknown("someNewProvider".into()))
    );
    assert_eq!(
        unknown.remedy,
        Some(ApiErrorRemedy::Unknown("some_new_remedy".into()))
    );
    assert_eq!(
        serde_json::to_value(&unknown).unwrap(),
        json!({"provider": "someNewProvider", "remedy": "some_new_remedy"})
    );
}

/// Every known provider and remedy maps both ways and prints as its wire
/// spelling.
#[test]
fn api_error_provider_and_remedy_known_values() {
    let providers = [
        ("bedrock", ApiErrorProvider::Bedrock),
        ("anthropicAws", ApiErrorProvider::AnthropicAws),
        ("mantle", ApiErrorProvider::Mantle),
        (
            "anthropicGoogleCloud",
            ApiErrorProvider::AnthropicGoogleCloud,
        ),
        ("vertex", ApiErrorProvider::Vertex),
        ("foundry", ApiErrorProvider::Foundry),
        ("gateway", ApiErrorProvider::Gateway),
    ];
    for (wire, variant) in providers {
        assert_eq!(ApiErrorProvider::from(wire), variant);
        assert_eq!(variant.as_str(), wire);
        assert_eq!(variant.to_string(), wire);
    }

    let remedies = [
        ("refresh_command", ApiErrorRemedy::RefreshCommand),
        ("refresh_credentials", ApiErrorRemedy::RefreshCredentials),
        ("adc", ApiErrorRemedy::Adc),
        ("gateway_token", ApiErrorRemedy::GatewayToken),
        ("host_managed", ApiErrorRemedy::HostManaged),
        ("model_access", ApiErrorRemedy::ModelAccess),
    ];
    for (wire, variant) in remedies {
        assert_eq!(ApiErrorRemedy::from(wire), variant);
        assert_eq!(variant.as_str(), wire);
        assert_eq!(variant.to_string(), wire);
    }
}

/// A `success` result that ended on an API error carries the gate code
/// alongside the existing HTTP status.
#[test]
fn result_success_api_error_code_fully_wrapped() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": true,
        "duration_ms": 1200,
        "duration_api_ms": 900,
        "num_turns": 1,
        "result": "API Error: field not granted",
        "session_id": "s1",
        "total_cost_usd": 0.0,
        "usage": {"input_tokens": 1, "output_tokens": 2},
        "permission_denials": [],
        "uuid": "u1",
        "api_error_status": 403,
        "api_error_code": "field_not_granted"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.subtype, ResultSubtype::Success);
    assert!(res.is_error);
    assert_eq!(res.api_error_status, Some(403));
    assert_eq!(res.api_error_code.as_deref(), Some("field_not_granted"));
    assert!(res.startup_failure_reason.is_none());
}

/// The zeroed `error_during_execution` result a run writes on a known
/// startup failure names the cause and echoes the stderr text in `errors`.
#[test]
fn result_startup_failure_reason_fully_wrapped() {
    let frame = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "duration_ms": 0,
        "duration_api_ms": 0,
        "num_turns": 0,
        "session_id": "s1",
        "total_cost_usd": 0.0,
        "usage": {"input_tokens": 0, "output_tokens": 0},
        "permission_denials": [],
        "errors": ["Working directory is no longer available"],
        "uuid": "u1",
        "startup_failure_reason": "cwd_unavailable"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.subtype, ResultSubtype::ErrorDuringExecution);
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::CwdUnavailable)
    );
    assert_eq!(res.errors, vec!["Working directory is no longer available"]);
    assert!(res.api_error_code.is_none());
}

/// Every known startup failure cause maps both ways; a cause this crate does
/// not know yet survives as `Unknown` and re-serializes verbatim.
#[test]
fn startup_failure_reason_known_values_and_unknown() {
    let known = [
        (
            "org_pin_api_key_conflict",
            StartupFailureReason::OrgPinApiKeyConflict,
        ),
        ("org_verify_failed", StartupFailureReason::OrgVerifyFailed),
        ("org_pin_mismatch", StartupFailureReason::OrgPinMismatch),
        (
            "managed_settings_invalid",
            StartupFailureReason::ManagedSettingsInvalid,
        ),
        (
            "remote_settings_required_unavailable",
            StartupFailureReason::RemoteSettingsRequiredUnavailable,
        ),
        (
            "gateway_signin_required",
            StartupFailureReason::GatewaySigninRequired,
        ),
        (
            "gateway_access_denied",
            StartupFailureReason::GatewayAccessDenied,
        ),
        ("proxy_invalid", StartupFailureReason::ProxyInvalid),
        ("temp_dir_unusable", StartupFailureReason::TempDirUnusable),
        ("cwd_unavailable", StartupFailureReason::CwdUnavailable),
        ("shell_tool_missing", StartupFailureReason::ShellToolMissing),
        (
            "session_held_by_background",
            StartupFailureReason::SessionHeldByBackground,
        ),
        (
            "worktree_resume_refused",
            StartupFailureReason::WorktreeResumeRefused,
        ),
        (
            "worktree_unverified",
            StartupFailureReason::WorktreeUnverified,
        ),
        (
            "cli_version_too_old",
            StartupFailureReason::CliVersionTooOld,
        ),
        ("bypass_root", StartupFailureReason::BypassRoot),
    ];
    for (wire, variant) in known {
        assert_eq!(StartupFailureReason::from(wire), variant);
        assert_eq!(variant.as_str(), wire);
        assert_eq!(variant.to_string(), wire);
        assert_eq!(
            serde_json::to_string(&variant).unwrap(),
            format!("\"{wire}\"")
        );
    }

    let unknown: StartupFailureReason = serde_json::from_value(json!("new_reason")).unwrap();
    assert_eq!(unknown, StartupFailureReason::Unknown("new_reason".into()));
    assert_eq!(serde_json::to_value(&unknown).unwrap(), json!("new_reason"));
}
