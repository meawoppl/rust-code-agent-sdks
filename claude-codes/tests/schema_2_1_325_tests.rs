//! Coverage for the CLI 2.1.325 stream-json drift.
//!
//! 2.1.323 → 2.1.325 added `api_error_type` on `assistant` and the result,
//! `restored_rewind` on the success result, `invalid_config_file` on the
//! result, `hook_no_verdict` on `system/informational`, `managed_lock` on
//! `system/init` `plugin_errors[]` entries, and `run_input` on `user`
//! `tool_result_meta[]` entries. The crate also picks up the
//! `plugin_errors[].path` key, which it had not modeled.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, StartupFailureReason};
use serde_json::json;

/// An assistant API-error message carries the body's `error.type`.
#[test]
fn assistant_carries_api_error_type() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "6f1d2c3e-4b5a-4c6d-8e7f-9a0b1c2d3e4f",
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": "API Error: rate limited"}],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {"input_tokens": 0, "output_tokens": 0}
        },
        "parent_tool_use_id": null,
        "session_id": "s1",
        "uuid": "u1",
        "error": "rate_limit",
        "api_error_status": 429,
        "api_error_type": "rate_limit_error"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    assert_eq!(msg.api_error_type.as_deref(), Some("rate_limit_error"));

    let wire = serde_json::to_value(&msg).unwrap();
    assert_eq!(wire["api_error_type"], "rate_limit_error");
}

/// A usage-limited turn whose rewind was restored reports the restore and
/// the error type that ended it.
#[test]
fn result_success_carries_restored_rewind_and_api_error_type() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": true,
        "duration_ms": 10,
        "duration_api_ms": 5,
        "num_turns": 1,
        "result": "You've hit your usage limit.",
        "stop_reason": null,
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.0,
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "api_error_status": 429,
        "api_error_type": "rate_limit_error",
        "restored_rewind": {
            "request_id": "req-7",
            "last_message_uuid": "m-41"
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.api_error_type.as_deref(), Some("rate_limit_error"));
    let rewind = res.restored_rewind.as_ref().expect("restored_rewind");
    assert_eq!(rewind.request_id, "req-7");
    assert_eq!(rewind.last_message_uuid, "m-41");

    let wire = serde_json::to_value(&res).unwrap();
    assert_eq!(wire["restored_rewind"]["last_message_uuid"], "m-41");
    assert!(wire.get("invalid_config_file").is_none());
}

/// The `config_file_invalid` startup result names the file and its backup
/// as separate fields.
#[test]
fn result_error_carries_invalid_config_file() {
    let frame = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "duration_ms": 0,
        "duration_api_ms": 0,
        "num_turns": 0,
        "stop_reason": null,
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.0,
        "usage": {},
        "modelUsage": {},
        "permission_denials": [],
        "errors": ["Configuration error in /home/u/.claude.json"],
        "startup_failure_reason": "config_file_invalid",
        "invalid_config_file": {
            "path": "/home/u/.claude.json",
            "is_global": true,
            "backup_searched": true,
            "backup_path": "/home/u/.claude/backups/.claude.json.backup.1"
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::ConfigFileInvalid)
    );
    let file = res
        .invalid_config_file
        .as_ref()
        .expect("invalid_config_file");
    assert_eq!(file.path, "/home/u/.claude.json");
    assert!(file.is_global);
    assert!(file.backup_searched);
    assert_eq!(
        file.backup_path.as_deref(),
        Some("/home/u/.claude/backups/.claude.json.backup.1")
    );

    let local = json!({
        "path": "/w/.claude/settings.json",
        "is_global": false,
        "backup_searched": false
    });
    let file: claude_codes::InvalidConfigFile = serde_json::from_value(local.clone()).unwrap();
    assert_eq!(file.backup_path, None);
    assert_eq!(serde_json::to_value(&file).unwrap(), local);
}

/// The warning that a PreToolUse hook gave no verdict is typed.
#[test]
fn informational_carries_hook_no_verdict() {
    let frame = json!({
        "type": "system",
        "subtype": "informational",
        "content": "PreToolUse hook timed out; the call went ahead.",
        "level": "warning",
        "tag": "hook_no_verdict",
        "tool_use_id": "toolu_1",
        "hook_no_verdict": {
            "hook_event": "PreToolUse",
            "tool_use_id": "toolu_1",
            "timed_out": true,
            "plugin_id": "guard@acme",
            "plugin_root": "/home/u/.claude/plugins/guard"
        },
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let Some(KnownSystemEvent::Informational(info)) = sys.as_known_system_event() else {
        panic!("expected Informational");
    };
    let nv = info.hook_no_verdict.expect("hook_no_verdict");
    assert_eq!(nv.hook_event, "PreToolUse");
    assert_eq!(nv.tool_use_id, "toolu_1");
    assert!(nv.timed_out);
    assert_eq!(nv.plugin_id.as_deref(), Some("guard@acme"));
    assert_eq!(
        nv.plugin_root.as_deref(),
        Some("/home/u/.claude/plugins/guard")
    );
}

/// `plugin_errors[]` entries carry the failed directory's path and the
/// managed-settings entry that dropped a directory copy.
#[test]
fn system_init_plugin_error_managed_lock() {
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "plugin_errors": [
            {
                "plugin": "inline[0]",
                "type": "managed-lock",
                "message": "Managed settings turn this plugin on from its marketplace.",
                "path": "/w/plugins/guard",
                "managed_lock": {
                    "plugin_enabled": true,
                    "name": "guard",
                    "marketplace": "acme"
                }
            },
            {
                "plugin": "inline[1]",
                "type": "managed-lock",
                "message": "Managed settings turn this plugin off.",
                "managed_lock": {"plugin_enabled": false, "name": "noisy"}
            }
        ]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    let errs = &init.plugin_errors;
    assert_eq!(errs[0].path.as_deref(), Some("/w/plugins/guard"));
    let lock = errs[0].managed_lock.as_ref().expect("managed_lock");
    assert!(lock.plugin_enabled);
    assert_eq!(lock.name, "guard");
    assert_eq!(lock.marketplace.as_deref(), Some("acme"));
    let lock = errs[1].managed_lock.as_ref().expect("managed_lock");
    assert!(!lock.plugin_enabled);
    assert_eq!(lock.marketplace, None);
    assert_eq!(errs[1].path, None);
}

/// `tool_result_meta[]` entries from a Remote worker say whether a person
/// typed the run's input.
#[test]
fn user_tool_result_meta_run_input() {
    let frame = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{"type": "tool_result", "tool_use_id": "toolu_1", "content": "done"}]
        },
        "parent_tool_use_id": null,
        "tool_result_meta": [
            {"id": "toolu_1", "built_in_tool": "Write", "run_input": "person_typed"}
        ],
        "uuid": "6f1c2e8a-3b4d-4e5f-8a9b-0c1d2e3f4a5b",
        "session_id": "7a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(user) = serde_json::from_value(frame).unwrap() else {
        panic!("expected User");
    };
    let meta = user.tool_result_meta.as_ref().unwrap();
    assert_eq!(meta[0].built_in_tool.as_deref(), Some("Write"));
    assert_eq!(meta[0].run_input.as_deref(), Some("person_typed"));
}
