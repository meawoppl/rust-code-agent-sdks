use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Result message for completed queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMessage {
    pub subtype: ResultSubtype,
    pub is_error: bool,
    pub duration_ms: u64,
    pub duration_api_ms: u64,
    pub num_turns: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,

    pub session_id: String,
    pub total_cost_usd: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,

    /// Tools that were blocked due to permission denials during the session
    #[serde(default)]
    pub permission_denials: Vec<PermissionDenial>,

    /// Error messages when `is_error` is true.
    ///
    /// Contains human-readable error strings (e.g., "No conversation found with session ID: ...").
    /// This allows typed access to error conditions without needing to serialize to JSON and search.
    #[serde(default)]
    pub errors: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,

    /// HTTP status code when the result is an API error (e.g., 429, 500, 529)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_error_status: Option<u16>,

    /// Why generation stopped (e.g., end_turn, max_tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// Why the session ended (e.g., "completed")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_reason: Option<String>,

    /// Fast mode toggle state (e.g., "off")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode_state: Option<String>,

    /// Per-model cost breakdown, keyed by model name
    #[serde(skip_serializing_if = "Option::is_none", rename = "modelUsage")]
    pub model_usage: Option<Value>,
}

/// A record of a tool permission that was denied during the session.
///
/// This is included in `ResultMessage.permission_denials` to provide a summary
/// of all permission denials that occurred.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionDenial {
    /// The name of the tool that was blocked (e.g., "Bash", "Write")
    pub tool_name: String,

    /// The input that was passed to the tool
    pub tool_input: Value,

    /// The unique identifier for this tool use request
    pub tool_use_id: String,
}

/// Result subtypes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultSubtype {
    Success,
    ErrorMaxTurns,
    ErrorDuringExecution,
}

/// Usage information for the request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    #[serde(default)]
    pub server_tool_use: ServerToolUse,
    #[serde(default)]
    pub service_tier: String,

    /// Cache creation breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<super::message_types::CacheCreationDetails>,

    /// Inference geography (e.g., "not_available")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,

    /// Per-turn usage breakdown
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub iterations: Vec<Value>,

    /// Speed tier (e.g., "standard")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
}

/// Server tool usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerToolUse {
    #[serde(default)]
    pub web_search_requests: u32,
    /// Number of web fetch requests made
    #[serde(default)]
    pub web_fetch_requests: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::ClaudeOutput;

    #[test]
    fn test_deserialize_result_message() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "result": "Done",
            "session_id": "123",
            "total_cost_usd": 0.01,
            "permission_denials": []
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(!output.is_error());
    }

    #[test]
    fn test_deserialize_result_with_permission_denials() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 2,
            "result": "Done",
            "session_id": "123",
            "total_cost_usd": 0.01,
            "permission_denials": [
                {
                    "tool_name": "Bash",
                    "tool_input": {"command": "rm -rf /", "description": "Delete everything"},
                    "tool_use_id": "toolu_123"
                }
            ]
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(result) = output {
            assert_eq!(result.permission_denials.len(), 1);
            assert_eq!(result.permission_denials[0].tool_name, "Bash");
            assert_eq!(result.permission_denials[0].tool_use_id, "toolu_123");
            assert_eq!(
                result.permission_denials[0]
                    .tool_input
                    .get("command")
                    .unwrap(),
                "rm -rf /"
            );
        } else {
            panic!("Expected Result");
        }
    }

    #[test]
    fn test_permission_denial_roundtrip() {
        let denial = PermissionDenial {
            tool_name: "Write".to_string(),
            tool_input: serde_json::json!({"file_path": "/etc/passwd", "content": "bad"}),
            tool_use_id: "toolu_456".to_string(),
        };

        let json = serde_json::to_string(&denial).unwrap();
        assert!(json.contains("\"tool_name\":\"Write\""));
        assert!(json.contains("\"tool_use_id\":\"toolu_456\""));
        assert!(json.contains("/etc/passwd"));

        let parsed: PermissionDenial = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, denial);
    }

    #[test]
    fn test_deserialize_result_message_with_errors() {
        let json = r#"{
            "type": "result",
            "subtype": "error_during_execution",
            "duration_ms": 0,
            "duration_api_ms": 0,
            "is_error": true,
            "num_turns": 0,
            "session_id": "27934753-425a-4182-892c-6b1c15050c3f",
            "total_cost_usd": 0,
            "errors": ["No conversation found with session ID: d56965c9-c855-4042-a8f5-f12bbb14d6f6"],
            "permission_denials": []
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_error());

        if let ClaudeOutput::Result(res) = output {
            assert!(res.is_error);
            assert_eq!(res.errors.len(), 1);
            assert!(res.errors[0].contains("No conversation found"));
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_deserialize_result_message_errors_defaults_empty() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "123",
            "total_cost_usd": 0.01
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(res) = output {
            assert!(res.errors.is_empty());
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_result_message_errors_roundtrip() {
        let json = r#"{
            "type": "result",
            "subtype": "error_during_execution",
            "is_error": true,
            "duration_ms": 0,
            "duration_api_ms": 0,
            "num_turns": 0,
            "session_id": "test-session",
            "total_cost_usd": 0.0,
            "errors": ["Error 1", "Error 2"]
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let reserialized = serde_json::to_string(&output).unwrap();

        assert!(reserialized.contains("Error 1"));
        assert!(reserialized.contains("Error 2"));
    }

    #[test]
    fn test_result_with_new_fields() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 5000,
            "duration_api_ms": 4500,
            "num_turns": 1,
            "result": "Done",
            "session_id": "abc",
            "total_cost_usd": 0.06,
            "api_error_status": null,
            "stop_reason": "end_turn",
            "terminal_reason": "completed",
            "fast_mode_state": "off",
            "modelUsage": {
                "claude-opus-4-7[1m]": {
                    "inputTokens": 3817,
                    "outputTokens": 14,
                    "costUSD": 0.06
                }
            },
            "usage": {
                "input_tokens": 3817,
                "output_tokens": 14,
                "cache_creation_input_tokens": 3540,
                "cache_read_input_tokens": 0,
                "server_tool_use": {
                    "web_search_requests": 0,
                    "web_fetch_requests": 2
                },
                "service_tier": "standard",
                "inference_geo": "not_available",
                "speed": "standard",
                "iterations": [
                    {"input_tokens": 3817, "output_tokens": 14, "type": "turn"}
                ]
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(res) = output {
            assert_eq!(res.stop_reason.as_deref(), Some("end_turn"));
            assert_eq!(res.terminal_reason.as_deref(), Some("completed"));
            assert_eq!(res.fast_mode_state.as_deref(), Some("off"));
            assert!(res.model_usage.is_some());
            assert!(res.api_error_status.is_none());

            let usage = res.usage.unwrap();
            assert_eq!(usage.server_tool_use.web_fetch_requests, 2);
            assert_eq!(usage.inference_geo.as_deref(), Some("not_available"));
            assert_eq!(usage.speed.as_deref(), Some("standard"));
            assert_eq!(usage.iterations.len(), 1);
        } else {
            panic!("Expected Result");
        }
    }

    #[test]
    fn test_result_backwards_compatible_without_new_fields() {
        // Verify old-format messages still parse fine
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(res) = output {
            assert!(res.api_error_status.is_none());
            assert!(res.stop_reason.is_none());
            assert!(res.terminal_reason.is_none());
            assert!(res.fast_mode_state.is_none());
            assert!(res.model_usage.is_none());
        } else {
            panic!("Expected Result");
        }
    }
}
