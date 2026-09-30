//! Coverage for the CLI 2.1.285 stream-json drift.
//!
//! 2.1.284 → 2.1.285 added eight `system` subtypes — `session_title_changed`
//! and the plugin-UI pushes `ui_log`, `ui_toast`, `ui_status`,
//! `ui_invalidate`, `ui_panes`, `ui_scroll` and `ui_focus` — plus additive
//! optional fields: seven request-timing fields and `api_error` on
//! `result`, `fallback_credit` on usage, `pre_compact_artifact_read_versions`
//! on `compact_boundary`, and the `provider_not_allowed` startup failure
//! reason. No removals.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, StartupFailureReason, SystemMessage,
    SystemSubtype, UiSiteComponent,
};
use serde_json::{json, Value};

fn system(frame: Value) -> SystemMessage {
    assert_fully_wrapped(&frame);
    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    sys
}

/// `system/session_title_changed` is modeled and round-trips fully wrapped
/// through both typed accessors.
#[test]
fn system_session_title_changed_fully_wrapped() {
    let sys = system(json!({
        "type": "system",
        "subtype": "session_title_changed",
        "title": "Fix the flaky login test",
        "uuid": "u1",
        "session_id": "s1"
    }));
    assert_eq!(sys.subtype, SystemSubtype::SessionTitleChanged);
    assert!(sys.is_session_title_changed());

    let direct = sys.as_session_title_changed().expect("typed accessor");
    assert_eq!(direct.title, "Fix the flaky login test");

    let Some(KnownSystemEvent::SessionTitleChanged(known)) = sys.as_known_system_event() else {
        panic!("expected SessionTitleChanged event");
    };
    assert_eq!(known.session_id, "s1");
}

/// The three plugin text pushes (`ui_log`, `ui_toast`, `ui_status`) are
/// modeled; a `ui_status` with a `null` text clears the plugin's line.
#[test]
fn system_ui_text_pushes_fully_wrapped() {
    let sys = system(json!({
        "type": "system",
        "subtype": "ui_log",
        "plugin": "deploy-helper",
        "text": "deploy queued",
        "uuid": "u1",
        "session_id": "s1"
    }));
    assert!(sys.is_ui_log());
    let log = sys.as_ui_log().expect("typed accessor");
    assert_eq!(log.plugin, "deploy-helper");
    assert_eq!(log.text, "deploy queued");
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::UiLog(_))
    ));

    let sys = system(json!({
        "type": "system",
        "subtype": "ui_toast",
        "plugin": "deploy-helper",
        "text": "deployed",
        "timeout_ms": 4000,
        "uuid": "u2",
        "session_id": "s1"
    }));
    assert!(sys.is_ui_toast());
    let toast = sys.as_ui_toast().expect("typed accessor");
    assert_eq!(toast.timeout_ms, 4000);
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::UiToast(_))
    ));

    let sys = system(json!({
        "type": "system",
        "subtype": "ui_status",
        "plugin": "deploy-helper",
        "text": "deploying 2/5",
        "uuid": "u3",
        "session_id": "s1"
    }));
    assert!(sys.is_ui_status());
    let status = sys.as_ui_status().expect("typed accessor");
    assert_eq!(status.text.as_deref(), Some("deploying 2/5"));

    let sys = system(json!({
        "type": "system",
        "subtype": "ui_status",
        "plugin": "deploy-helper",
        "text": null,
        "uuid": "u4",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::UiStatus(cleared)) = sys.as_known_system_event() else {
        panic!("expected UiStatus event");
    };
    assert_eq!(cleared.text, None);
    assert_eq!(serde_json::to_value(&cleared).unwrap()["text"], Value::Null);
}

/// `system/ui_invalidate` names the stale instances when a state write made
/// only those stale, and omits the list when every instance may be.
#[test]
fn system_ui_invalidate_fully_wrapped() {
    let sys = system(json!({
        "type": "system",
        "subtype": "ui_invalidate",
        "event": "ui.render",
        "instances": [
            {"surface": "remote", "component": "Pane", "instance_id": "req-7"}
        ],
        "uuid": "u1",
        "session_id": "s1"
    }));
    assert!(sys.is_ui_invalidate());
    let scoped = sys.as_ui_invalidate().expect("typed accessor");
    assert_eq!(scoped.event, "ui.render");
    let instances = scoped.instances.expect("scoped instances");
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].component, "Pane");
    assert_eq!(instances[0].instance_id, "req-7");

    let sys = system(json!({
        "type": "system",
        "subtype": "ui_invalidate",
        "event": "ui.render",
        "uuid": "u2",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::UiInvalidate(all)) = sys.as_known_system_event() else {
        panic!("expected UiInvalidate event");
    };
    assert_eq!(all.instances, None);
}

/// `system/ui_panes` carries the placed-pane roster with its three nullable
/// pane pointers and each pane's optional manners and size.
#[test]
fn system_ui_panes_fully_wrapped() {
    let sys = system(json!({
        "type": "system",
        "subtype": "ui_panes",
        "panes": [
            {
                "id": "deploy-log",
                "title": "Deploy log",
                "plugin": "deploy-helper",
                "close_on_escape": true,
                "hold_toasts": true,
                "rows": 12,
                "columns": 80
            },
            {"id": "notes", "title": "notes", "plugin": "scratch"}
        ],
        "shown_id": "deploy-log",
        "focused_id": null,
        "focus_requested_id": "notes",
        "uuid": "u1",
        "session_id": "s1"
    }));
    assert!(sys.is_ui_panes());
    let roster = sys.as_ui_panes().expect("typed accessor");
    assert_eq!(roster.panes.len(), 2);
    assert_eq!(roster.panes[0].close_on_escape, Some(true));
    assert_eq!(roster.panes[0].hold_toasts, Some(true));
    assert_eq!(roster.panes[0].rows, Some(12));
    assert_eq!(roster.panes[0].columns, Some(80));
    assert_eq!(roster.panes[1].close_on_escape, None);
    assert_eq!(roster.panes[1].rows, None);
    assert_eq!(roster.shown_id.as_deref(), Some("deploy-log"));
    assert_eq!(roster.focused_id, None);
    assert_eq!(roster.focus_requested_id.as_deref(), Some("notes"));

    let sys = system(json!({
        "type": "system",
        "subtype": "ui_panes",
        "panes": [],
        "shown_id": null,
        "focused_id": null,
        "focus_requested_id": null,
        "uuid": "u2",
        "session_id": "s1"
    }));
    let Some(KnownSystemEvent::UiPanes(empty)) = sys.as_known_system_event() else {
        panic!("expected UiPanes event");
    };
    assert!(empty.panes.is_empty());
    assert_eq!(empty.shown_id, None);
}

/// `system/ui_scroll` and `system/ui_focus` address a site by client,
/// component and instance.
#[test]
fn system_ui_scroll_and_focus_fully_wrapped() {
    let sys = system(json!({
        "type": "system",
        "subtype": "ui_scroll",
        "client_id": "c1",
        "component": "Pane",
        "instance_id": "deploy-log",
        "offset": 40,
        "follow_end": true,
        "uuid": "u1",
        "session_id": "s1"
    }));
    assert!(sys.is_ui_scroll());
    let scroll = sys.as_ui_scroll().expect("typed accessor");
    assert_eq!(scroll.component, UiSiteComponent::Pane);
    assert_eq!(scroll.offset, 40);
    assert_eq!(scroll.follow_end, Some(true));
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::UiScroll(_))
    ));

    let sys = system(json!({
        "type": "system",
        "subtype": "ui_focus",
        "client_id": "c1",
        "component": "AbovePrompt",
        "instance_id": "above-prompt",
        "plugin": "deploy-helper",
        "key": "confirm",
        "uuid": "u2",
        "session_id": "s1"
    }));
    assert!(sys.is_ui_focus());
    let focus = sys.as_ui_focus().expect("typed accessor");
    assert_eq!(focus.component, UiSiteComponent::AbovePrompt);
    assert_eq!(focus.instance_id, "above-prompt");
    assert_eq!(focus.key, "confirm");
    assert!(matches!(
        sys.as_known_system_event(),
        Some(KnownSystemEvent::UiFocus(_))
    ));
}

/// The site component is an open set: an unknown value is preserved.
#[test]
fn ui_site_component_is_open_set() {
    for (wire, expected) in [
        ("Pane", UiSiteComponent::Pane),
        ("AbovePrompt", UiSiteComponent::AbovePrompt),
        ("Sidebar", UiSiteComponent::Unknown("Sidebar".into())),
    ] {
        let parsed: UiSiteComponent = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.to_string(), wire);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), json!(wire));
    }
}

fn result_frame() -> Value {
    json!({
        "type": "result",
        "subtype": "success",
        "is_error": true,
        "duration_ms": 1200,
        "duration_api_ms": 900,
        "num_turns": 1,
        "result": "API Error: overloaded",
        "stop_reason": null,
        "session_id": "s1",
        "uuid": "u1",
        "total_cost_usd": 0.01,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 38,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0,
            "fallback_credit": {"outcome": "applied"}
        },
        "modelUsage": {},
        "permission_denials": [],
        "time_to_request_ms": 140,
        "time_to_request_phases_ms": {
            "system_prompt": 20,
            "process_user_input": 35,
            "tool_schema_build": 60,
            "other": 25
        },
        "process_turn_index": 0,
        "time_to_request_cpu_ms": 95,
        "time_to_request_loop_lag_ms": 12,
        "time_to_request_major_faults": 3,
        "frame_received_wall_ms": 1753212345600.5,
        "frame_enqueued_wall_ms": 1753212345612.5,
        "turn_started_wall_ms": 1753212345650.5,
        "turn_start_phases_ms": {
            "idle": 4,
            "tool_pool": 21,
            "engine_setup": 9,
            "other": 4
        },
        "turn_start_control_requests_ms": 6,
        "api_error_status": 529,
        "api_error": "overloaded"
    })
}

/// `result` carries the request-timing breakdown, the turn-start breakdown
/// and the typed API error kind, fully wrapped.
#[test]
fn result_carries_request_timing_and_api_error() {
    let frame = result_frame();
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    let phases = res.time_to_request_phases_ms.as_ref().expect("phases");
    assert_eq!(
        phases.values().sum::<u64>(),
        res.time_to_request_ms.unwrap()
    );
    assert_eq!(phases.get("tool_schema_build"), Some(&60));
    assert_eq!(phases.get("autocompact"), None);
    assert_eq!(res.process_turn_index, Some(0));
    assert_eq!(res.time_to_request_cpu_ms, Some(95));
    assert_eq!(res.time_to_request_loop_lag_ms, Some(12));
    assert_eq!(res.time_to_request_major_faults, Some(3));

    let steps = res.turn_start_phases_ms.as_ref().expect("steps");
    assert_eq!(steps.len(), 4);
    assert_eq!(steps.get("tool_pool"), Some(&21));
    assert_eq!(res.turn_start_control_requests_ms, Some(6));

    assert_eq!(res.api_error.as_deref(), Some("overloaded"));
    let usage = res.usage.expect("usage");
    assert_eq!(usage.fallback_credit, Some(json!({"outcome": "applied"})));
}

/// A result without the new fields (older CLI, or a host that does not emit
/// startup timing) still parses, and a `null` fallback credit reads as none.
#[test]
fn result_new_fields_are_optional() {
    let frame = json!({
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
        "usage": {"input_tokens": 10, "output_tokens": 38, "fallback_credit": null},
        "modelUsage": {},
        "permission_denials": []
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(res.time_to_request_phases_ms, None);
    assert_eq!(res.process_turn_index, None);
    assert_eq!(res.time_to_request_cpu_ms, None);
    assert_eq!(res.turn_start_phases_ms, None);
    assert_eq!(res.turn_start_control_requests_ms, None);
    assert_eq!(res.api_error, None);
    assert_eq!(res.usage.expect("usage").fallback_credit, None);
}

/// The assistant message's usage carries the same fallback-credit outcome.
#[test]
fn assistant_usage_carries_fallback_credit() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-5-5",
            "content": [{"type": "text", "text": "ok"}],
            "stop_reason": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 6,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
                "fallback_credit": {"outcome": "applied"}
            }
        },
        "session_id": "s1",
        "uuid": "u1",
        "parent_tool_use_id": null
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Assistant");
    };
    let usage = msg.message.usage.expect("usage");
    assert_eq!(usage.fallback_credit, Some(json!({"outcome": "applied"})));
}

/// `compact_boundary` lists the artifact versions the conversation held
/// before the compaction.
#[test]
fn compact_boundary_carries_artifact_read_versions() {
    let sys = system(json!({
        "type": "system",
        "subtype": "compact_boundary",
        "session_id": "s1",
        "uuid": "u1",
        "compact_metadata": {
            "pre_tokens": 150000,
            "trigger": "auto",
            "pre_compact_artifact_read_versions": [
                {"slug": "design-doc", "ver": "7"},
                {"slug": "release-notes", "ver": "2"}
            ]
        }
    }));
    let boundary = sys.as_compact_boundary().expect("typed compact boundary");
    let versions = boundary
        .compact_metadata
        .pre_compact_artifact_read_versions
        .expect("typed versions");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].slug, "design-doc");
    assert_eq!(versions[0].ver, "7");
}

/// `provider_not_allowed` is a typed startup failure reason.
#[test]
fn startup_failure_reason_provider_not_allowed() {
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
        "errors": ["This machine's managed settings do not allow this API provider."],
        "startup_failure_reason": "provider_not_allowed"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected Result");
    };
    assert_eq!(
        res.startup_failure_reason,
        Some(StartupFailureReason::ProviderNotAllowed)
    );
    assert_eq!(
        StartupFailureReason::ProviderNotAllowed.to_string(),
        "provider_not_allowed"
    );
}
