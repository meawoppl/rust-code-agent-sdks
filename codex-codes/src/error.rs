//! Error types for the codex-codes crate.
//!
//! All fallible operations return [`Result<T>`], which uses [`enum@Error`] as the
//! error type. The variants cover JSON serialization, I/O, protocol-level
//! issues, and JSON-RPC errors from the app-server.

use serde_json::Value;
use thiserror::Error;

/// Error type for parsing failures that preserves the raw frame data.
///
/// Returned inside [`Error::Deserialization`] when a message from the
/// app-server fails to deserialize. The structured fields let consumers
/// render the offending frame in bug reports without grepping logs.
///
/// Two failure modes are represented:
///
/// 1. **Bare JSON failure** — the line wasn't valid JSON, or the JSON didn't
///    match the [`JsonRpcMessage`](crate::JsonRpcMessage) envelope. `raw_line`
///    is the original line from stdout. `raw_json` is populated when the line
///    parsed as JSON but didn't fit the envelope; `None` when the line wasn't
///    even JSON. `method` is `None`.
///
/// 2. **Typed decode failure** — the envelope parsed fine (so the JSON-RPC
///    `method` is known), but the typed payload decode
///    (`Notification::from_envelope` / `ServerRequest::from_envelope`) failed
///    on the `params`. `method` carries the JSON-RPC method name. `raw_json`
///    carries the `params` value. `raw_line` is the re-serialized envelope —
///    wire-equivalent to what came in, suitable for pasting into a bug report.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Line from stdout (or re-serialized envelope, for typed-decode failures).
    pub raw_line: String,
    /// Parsed JSON value when available (the `params` for typed-decode
    /// failures; the parsed line for envelope-shape failures).
    pub raw_json: Option<Value>,
    /// The underlying serde error description.
    pub error_message: String,
    /// JSON-RPC `method` name when the failure happened at the typed-decode
    /// stage. `None` for bare-JSON / envelope-shape failures.
    pub method: Option<String>,
}

impl ParseError {
    /// Build a [`ParseError`] for a bare-JSON or envelope-shape failure.
    pub fn from_line(line: impl Into<String>, error: serde_json::Error) -> Self {
        let raw_line = line.into();
        let raw_json = serde_json::from_str::<Value>(&raw_line).ok();
        ParseError {
            raw_line,
            raw_json,
            error_message: error.to_string(),
            method: None,
        }
    }

    /// Build a [`ParseError`] for a typed-decode failure on a notification or
    /// request whose envelope parsed but whose `params` did not match.
    ///
    /// `raw_line` is reconstructed by re-serializing the envelope so consumers
    /// can render the full offending frame even though the original line was
    /// already consumed by the envelope decode.
    pub fn from_envelope(
        method: impl Into<String>,
        params: Option<Value>,
        error: serde_json::Error,
    ) -> Self {
        let method = method.into();
        let raw_line = match &params {
            Some(p) => format!(
                r#"{{"method":{},"params":{}}}"#,
                serde_json::to_string(&method).unwrap_or_else(|_| "\"<unserializable>\"".into()),
                serde_json::to_string(p).unwrap_or_else(|_| "null".into()),
            ),
            None => format!(
                r#"{{"method":{}}}"#,
                serde_json::to_string(&method).unwrap_or_else(|_| "\"<unserializable>\"".into()),
            ),
        };
        ParseError {
            raw_line,
            raw_json: params,
            error_message: error.to_string(),
            method: Some(method),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.method {
            Some(m) => write!(
                f,
                "Failed to decode params for method {:?}: {} (raw: {})",
                m, self.error_message, self.raw_line
            ),
            None => write!(
                f,
                "Failed to parse JSON-RPC message: {} (raw: {})",
                self.error_message, self.raw_line
            ),
        }
    }
}

impl std::error::Error for ParseError {}

/// All possible errors from codex-codes operations.
#[derive(Error, Debug)]
pub enum Error {
    /// JSON serialization or deserialization failed.
    ///
    /// Returned when request parameters can't be serialized or
    /// response payloads don't match expected types.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// An I/O error occurred communicating with the app-server process.
    ///
    /// Common causes: process not found, pipe broken, permission denied.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A protocol-level error (e.g., missing stdin/stdout pipes).
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// The app-server connection was closed unexpectedly.
    #[error("Connection closed")]
    ConnectionClosed,

    /// A message from the server could not be deserialized.
    ///
    /// Carries a [`ParseError`] with the offending `method` (when known),
    /// raw frame, and the underlying serde diagnostic. If you encounter this,
    /// please report it with the `raw_line` — it likely indicates a protocol
    /// change.
    #[error("Deserialization error: {0}")]
    Deserialization(#[from] ParseError),

    /// The app-server process exited with a non-zero status.
    #[error("Process exited with status {0}: {1}")]
    ProcessFailed(i32, String),

    /// The server returned a JSON-RPC error response.
    ///
    /// Contains the error code and message from the server, plus the
    /// optional structured `data` payload verbatim. Some errors carry a typed
    /// payload there; decode the known ones with helpers such as
    /// [`Error::model_provider_error`]. See the Codex CLI docs for error code
    /// meanings.
    #[error("JSON-RPC error ({code}): {message}")]
    JsonRpc {
        code: i64,
        message: String,
        data: Option<Value>,
    },

    /// The server closed the connection (EOF on stdout).
    ///
    /// Returned by `request()` if the server exits mid-conversation.
    #[error("Server closed connection")]
    ServerClosed,

    /// The CLI binary could not be found on PATH.
    #[error("Binary not found: '{name}' is not on PATH. Is it installed?")]
    BinaryNotFound { name: String },

    /// An unclassified error.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<crate::JsonRpcErrorData> for Error {
    fn from(err: crate::JsonRpcErrorData) -> Self {
        Error::JsonRpc {
            code: err.code,
            message: err.message,
            data: err.data,
        }
    }
}

impl Error {
    /// The structured `data` of a [`Error::JsonRpc`] error, if the server sent
    /// one. `None` for every other variant.
    pub fn jsonrpc_data(&self) -> Option<&Value> {
        match self {
            Error::JsonRpc { data, .. } => data.as_ref(),
            _ => None,
        }
    }

    /// Decode a [`Error::JsonRpc`] error's `data` as a
    /// [`ModelProviderError`](crate::protocol::ModelProviderError).
    ///
    /// The app-server attaches one when a thread's stored model provider
    /// can't be used, e.g. `thread/resume` of a thread created under a
    /// provider this machine doesn't configure or policy forbids. `None`
    /// when there is no `data` or it has a different shape; callers should
    /// then fall back to `code`/`message`.
    pub fn model_provider_error(&self) -> Option<crate::protocol::ModelProviderError> {
        serde_json::from_value(self.jsonrpc_data()?.clone()).ok()
    }
}

/// A `Result` type alias using [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn serde_err(s: &str) -> serde_json::Error {
        serde_json::from_str::<Value>(s).unwrap_err()
    }

    #[test]
    fn parse_error_from_line_valid_json_populates_raw_json() {
        let line = r#"{"foo":"bar"}"#;
        // Force a downstream typed-decode failure to produce a serde_json::Error;
        // we just need *some* error to attach.
        let err = serde_json::from_str::<i32>(line).unwrap_err();

        let pe = ParseError::from_line(line, err);
        assert_eq!(pe.raw_line, line);
        assert_eq!(pe.raw_json, Some(json!({"foo": "bar"})));
        assert!(pe.method.is_none());
        assert!(!pe.error_message.is_empty());
    }

    #[test]
    fn parse_error_from_line_invalid_json_has_none_raw_json() {
        let line = "not-json{";
        let err = serde_err(line);

        let pe = ParseError::from_line(line, err);
        assert_eq!(pe.raw_line, line);
        assert!(pe.raw_json.is_none());
        assert!(pe.method.is_none());
    }

    #[test]
    fn parse_error_from_envelope_carries_method_params_and_reconstructs_line() {
        let params = json!({"callId": null, "kind": "fileChange"});
        let err = serde_json::from_value::<i32>(params.clone()).unwrap_err();

        let pe =
            ParseError::from_envelope("item/fileChange/requestApproval", Some(params.clone()), err);

        assert_eq!(
            pe.method.as_deref(),
            Some("item/fileChange/requestApproval")
        );
        assert_eq!(pe.raw_json, Some(params.clone()));

        // raw_line round-trips to the same shape (re-parse what we built).
        let v: Value = serde_json::from_str(&pe.raw_line).unwrap();
        assert_eq!(v["method"], "item/fileChange/requestApproval");
        assert_eq!(v["params"], params);
    }

    #[test]
    fn parse_error_from_envelope_handles_missing_params() {
        let err = serde_err("not json");
        let pe = ParseError::from_envelope("turn/completed", None, err);
        let v: Value = serde_json::from_str(&pe.raw_line).unwrap();
        assert_eq!(v["method"], "turn/completed");
        assert!(v.get("params").is_none());
        assert!(pe.raw_json.is_none());
    }

    #[test]
    fn error_deserialization_display_includes_method_and_raw() {
        let params = json!({"foo": 1});
        let err = serde_err("not json");
        let pe = ParseError::from_envelope("item/bogus", Some(params), err);
        let e: Error = Error::Deserialization(pe);
        let rendered = format!("{}", e);
        assert!(rendered.contains("item/bogus"), "got: {}", rendered);
        assert!(rendered.contains("foo"), "got: {}", rendered);
    }

    fn jsonrpc(data: Option<Value>) -> Error {
        crate::JsonRpcErrorData {
            code: -32600,
            message: "bad".to_string(),
            data,
        }
        .into()
    }

    #[test]
    fn model_provider_error_decodes_policy_conflict() {
        let err = jsonrpc(Some(json!({
            "type": "modelProviderPolicyConflict",
            "modelProvider": "old",
            "requiredModelProvider": "corp"
        })));
        assert_eq!(
            err.model_provider_error(),
            Some(
                crate::protocol::ModelProviderError::ModelProviderPolicyConflict {
                    model_provider: "old".to_string(),
                    required_model_provider: "corp".to_string(),
                }
            )
        );
    }

    #[test]
    fn model_provider_error_none_for_other_data_and_variants() {
        let other = json!({"reason": "cloudConfigBundle", "detail": "x"});
        let err = jsonrpc(Some(other.clone()));
        assert_eq!(err.jsonrpc_data(), Some(&other));
        assert!(err.model_provider_error().is_none());

        let bare = jsonrpc(None);
        assert!(bare.jsonrpc_data().is_none());
        assert!(bare.model_provider_error().is_none());

        let not_rpc = Error::Protocol("x".to_string());
        assert!(not_rpc.jsonrpc_data().is_none());
        assert!(not_rpc.model_provider_error().is_none());
    }
}
