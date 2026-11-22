//! Coverage for the typed control channel and tolerant required fields.
//!
//! The request tests compare whole JSON values, because the point of a typed
//! request is that the CLI sees the same bytes a host writes by hand. The ask
//! shapes are the CLI's own: an `ExitPlanMode` ask with null suggestions, and
//! the `Bash` suggestions recorded by Giskard's `tool-allowed` fixture.

use claude_codes::io::{AssistantMessage, ClaudeInput, ResultMessage};
use claude_codes::{
    ApplyFlagSettingsRequest, ClaudeOutput, ControlRequestPayload, ControlResponse,
    ControlResponsePayload, EffortLevel, Permission, PermissionDestination, PermissionModeName,
    PermissionSuggestion, PermissionType, ToolPermissionRequest,
};
use serde_json::{json, Value};

fn wire(input: &ClaudeInput) -> Value {
    serde_json::to_value(input).unwrap()
}

/// `set_permission_mode` serializes exactly as the protocol's hand-built form.
#[test]
fn set_permission_mode_serializes_to_the_wire_shape() {
    assert_eq!(
        wire(&ClaudeInput::set_permission_mode(
            "r1",
            PermissionModeName::Plan
        )),
        json!({"type": "control_request", "request_id": "r1",
               "request": {"subtype": "set_permission_mode", "mode": "plan"}})
    );
}

/// `set_model` names the model, and leaves the field out to reset to the default.
#[test]
fn set_model_serializes_with_and_without_a_model() {
    assert_eq!(
        wire(&ClaudeInput::set_model("r1", Some("haiku".into()))),
        json!({"type": "control_request", "request_id": "r1",
               "request": {"subtype": "set_model", "model": "haiku"}})
    );
    assert_eq!(
        wire(&ClaudeInput::set_model("r2", None)),
        json!({"type": "control_request", "request_id": "r2",
               "request": {"subtype": "set_model"}})
    );
}

/// `apply_flag_settings` still serializes as before beside the new requests.
#[test]
fn apply_flag_settings_writes_only_the_keys_set() {
    let settings = ApplyFlagSettingsRequest::new().effort_level(EffortLevel::High);
    assert_eq!(
        wire(&ClaudeInput::apply_flag_settings("r1", settings)),
        json!({"type": "control_request", "request_id": "r1",
               "request": {"subtype": "apply_flag_settings",
                           "settings": {"effortLevel": "high"}}})
    );
}

/// `get_settings` takes no parameters, and the new payloads parse back from the wire.
#[test]
fn get_settings_serializes_and_parses_as_a_bare_subtype() {
    let value = wire(&ClaudeInput::get_settings("r1"));
    assert_eq!(
        value,
        json!({"type": "control_request", "request_id": "r1",
               "request": {"subtype": "get_settings"}})
    );
    let payload: ControlRequestPayload = serde_json::from_value(value["request"].clone()).unwrap();
    assert!(matches!(payload, ControlRequestPayload::GetSettings));
}

/// The read-back exposes `applied`, the only proof that an effort change landed.
#[test]
fn get_settings_response_reads_applied() {
    let response = ControlResponse::success(
        "r1",
        json!({
            "effective": {},
            "sources": [{"source": "flagSettings", "settings": {"effortLevel": "max"}}],
            "applied": {"model": "claude-haiku-5-5", "effort": "max", "ultracodeAvailable": true}
        }),
    );
    let settings = response.get_settings_response().unwrap().unwrap();
    assert_eq!(settings.applied.model.as_deref(), Some("claude-haiku-5-5"));
    assert_eq!(settings.applied.effort.as_deref(), Some("max"));
    assert_eq!(settings.applied.ultracode_available, Some(true));
    assert_eq!(settings.sources[0].source, "flagSettings");
    // `effective` holds no effort here even though a valid `max` was applied.
    assert!(settings.effective.is_empty());
}

/// A model without effort reports null, and a read-back with only `applied` still parses.
#[test]
fn get_settings_response_accepts_null_effort_and_missing_sections() {
    let response = ControlResponse::success("r1", json!({"applied": {"effort": null}}));
    let settings = response.get_settings_response().unwrap().unwrap();
    assert_eq!(settings.applied.effort, None);
    assert_eq!(settings.applied.model, None);
}

/// `set_permission_mode`'s answer echoes the mode, or acknowledges with an empty object.
#[test]
fn set_permission_mode_response_reads_the_echo() {
    let echo = ControlResponse::success("r1", json!({"mode": "acceptEdits"}));
    assert_eq!(
        echo.set_permission_mode_response().unwrap().unwrap().mode,
        Some(PermissionModeName::AcceptEdits)
    );
    let empty = ControlResponse::success("r2", json!({}));
    assert_eq!(
        empty.set_permission_mode_response().unwrap().unwrap().mode,
        None
    );
}

/// A refusal keeps its `error_code`, which hosts branch on, and an error without one omits it.
#[test]
fn control_errors_keep_their_error_code() {
    let frame = json!({"type": "control_response", "response": {
        "subtype": "error", "request_id": "r1",
        "error": "Model 'x' not found", "error_code": "catalog_unknown"
    }});
    let ClaudeOutput::ControlResponse(response) = serde_json::from_value(frame).unwrap() else {
        panic!("not a control response");
    };
    let ControlResponsePayload::Error { error_code, .. } = &response.response else {
        panic!("not an error");
    };
    assert_eq!(error_code.as_deref(), Some("catalog_unknown"));

    let built = serde_json::to_value(ControlResponse::error("r2", "no")).unwrap();
    assert!(built["response"].get("error_code").is_none());
}

/// An `ExitPlanMode` ask sends null suggestions, which must not lose the ask.
#[test]
fn an_ask_with_null_suggestions_parses() {
    let ask: ToolPermissionRequest = serde_json::from_value(json!({
        "tool_name": "ExitPlanMode", "input": {}, "permission_suggestions": null
    }))
    .unwrap();
    assert!(ask.permission_suggestions.is_empty());
}

/// A sub-agent's ask exposes the fields hosts route and label it by.
#[test]
fn an_ask_exposes_its_routing_and_display_fields() {
    let ask: ToolPermissionRequest = serde_json::from_value(json!({
        "tool_name": "Bash", "input": {"command": "ls"}, "tool_use_id": "t1",
        "agent_id": "a1", "title": "Run ls", "display_name": "Bash",
        "description": "List files", "decision_reason_type": "rule",
        "requires_user_interaction": false
    }))
    .unwrap();
    assert_eq!(ask.agent_id.as_deref(), Some("a1"));
    assert_eq!(ask.title.as_deref(), Some("Run ls"));
    assert_eq!(ask.display_name.as_deref(), Some("Bash"));
    assert_eq!(ask.description.as_deref(), Some("List files"));
    assert_eq!(ask.decision_reason_type.as_deref(), Some("rule"));
    assert_eq!(ask.requires_user_interaction, Some(false));
}

/// All three recorded suggestion shapes survive a typed round trip unchanged.
#[test]
fn recorded_suggestions_round_trip_losslessly() {
    let recorded = json!([
        {"type": "addRules",
         "rules": [{"toolName": "Bash", "ruleContent": "touch probe.txt"}],
         "behavior": "allow", "destination": "localSettings"},
        {"type": "addDirectories", "directories": ["/work/project"], "destination": "session"},
        {"type": "setMode", "mode": "acceptEdits", "destination": "session"}
    ]);
    let suggestions: Vec<PermissionSuggestion> = serde_json::from_value(recorded.clone()).unwrap();
    assert_eq!(
        suggestions[0].destination,
        PermissionDestination::LocalSettings
    );
    assert_eq!(
        suggestions[1].suggestion_type,
        PermissionType::AddDirectories
    );
    assert_eq!(serde_json::to_value(&suggestions).unwrap(), recorded);
}

/// A suggested directory reaches the permission built from it.
#[test]
fn from_suggestion_keeps_directories() {
    let suggestion: PermissionSuggestion = serde_json::from_value(json!({
        "type": "addDirectories", "directories": ["/work"], "destination": "session"
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(Permission::from_suggestion(&suggestion)).unwrap(),
        json!({"type": "addDirectories", "directories": ["/work"], "destination": "session"})
    );
}

/// `ruleContent` is optional on the wire: a whole-tool rule must not be dropped or given content.
#[test]
fn from_suggestion_keeps_a_rule_without_content() {
    let suggestion: PermissionSuggestion = serde_json::from_value(json!({
        "type": "addRules", "rules": [{"toolName": "WebSearch"}],
        "behavior": "allow", "destination": "session"
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(Permission::from_suggestion(&suggestion)).unwrap(),
        json!({"type": "addRules", "rules": [{"toolName": "WebSearch"}],
               "behavior": "allow", "destination": "session"})
    );
}

/// A rule without content reads as `None`, the whole tool, while an empty pattern stays `""`.
#[test]
fn whole_tool_rules_are_explicit_and_empty_patterns_are_not_whole_tool() {
    let suggestion: PermissionSuggestion = serde_json::from_value(json!({
        "type": "addRules", "rules": [{"toolName": "WebSearch"}],
        "behavior": "allow", "destination": "session"
    }))
    .unwrap();
    let rules = Permission::from_suggestion(&suggestion).rules.unwrap();
    assert_eq!(rules[0].rule_content, None);

    let empty = serde_json::to_value(Permission::allow_tool("Bash", "")).unwrap();
    assert_eq!(empty["rules"][0]["ruleContent"], json!(""));
}

/// A read-back without `applied` still yields `effective` and `sources`.
#[test]
fn get_settings_response_without_applied_parses() {
    let response = ControlResponse::success(
        "r1",
        json!({"effective": {"effortLevel": "low"}, "sources": []}),
    );
    let settings = response.get_settings_response().unwrap().unwrap();
    assert_eq!(settings.applied.model, None);
    assert_eq!(settings.effective["effortLevel"], json!("low"));
}

/// A success with no payload acknowledges `set_permission_mode` without an echo; an error is `None`.
#[test]
fn set_permission_mode_response_tells_a_bare_ack_from_an_error() {
    let ack = ControlResponse::success_empty("r1");
    assert_eq!(
        ack.set_permission_mode_response().unwrap().unwrap().mode,
        None
    );
    assert!(ControlResponse::error("r2", "no")
        .set_permission_mode_response()
        .is_none());
}

/// The mode names the CLI uses are typed, and an unknown one still round-trips.
#[test]
fn permission_mode_names_cover_the_cli_and_stay_open() {
    for (wire_name, mode) in [
        ("default", PermissionModeName::Default),
        ("plan", PermissionModeName::Plan),
        ("dontAsk", PermissionModeName::DontAsk),
        ("auto", PermissionModeName::Auto),
    ] {
        assert_eq!(PermissionModeName::from(wire_name), mode);
        assert_eq!(mode.as_str(), wire_name);
    }
    assert_eq!(PermissionModeName::from("new").as_str(), "new");
}

/// A `result` without `total_cost_usd` still parses: losing it would leave a turn open.
#[test]
fn a_result_without_a_cost_parses() {
    let result: ResultMessage = serde_json::from_value(json!({
        "subtype": "success", "is_error": false, "duration_ms": 1, "duration_api_ms": 1,
        "num_turns": 1, "session_id": "s"
    }))
    .unwrap();
    assert_eq!(result.total_cost_usd, 0.0);
}

/// An `assistant` frame missing `id`, `model` and a thought's `signature` keeps its content.
#[test]
fn an_assistant_frame_without_id_model_or_signature_parses() {
    let frame: AssistantMessage = serde_json::from_value(json!({
        "session_id": "s", "uuid": "u", "parent_tool_use_id": null,
        "message": {
            "role": "assistant",
            "content": [{"type": "thinking", "thinking": "hm"}, {"type": "text", "text": "hi"}]
        }
    }))
    .unwrap();
    assert_eq!(frame.message.id, "");
    assert_eq!(frame.message.model, "");
    assert_eq!(frame.message.content.len(), 2);
}
