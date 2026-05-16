//! Typed dispatch for app-server notifications and server-to-client requests.
//!
//! The Codex app-server speaks JSON-RPC where every message carries a
//! `method` discriminant alongside a free-form `params` blob. This module
//! lifts that loose envelope into closed enums — [`Notification`] for
//! server-initiated notifications and [`ServerRequest`] for server-initiated
//! requests (the approval flow). Each variant wraps a typed param struct
//! from [`crate::protocol`].
//!
//! The pattern mirrors the [`ContentBlock`] dispatch in the sibling
//! `claude-codes` crate: hand-written [`Serialize`]/[`Deserialize`] impls
//! inspect the discriminant, route known cases through `serde_json::from_value`
//! into the typed struct, and route unknown methods into an `Unknown`
//! variant — preserving the raw payload for forward compatibility with
//! future codex versions.
//!
//! ## Typing contract
//!
//! - Unknown methods route to [`Notification::Unknown`] / [`ServerRequest::Unknown`]
//!   without error. Encountering one in production typically means the
//!   installed Codex CLI is newer than the bindings.
//! - Known methods whose payload fails to deserialize **do** cause an error.
//!   If you see one, the typed binding in [`crate::protocol`] is out of
//!   sync with the wire format and needs to be updated.

use crate::jsonrpc::RequestId;
use crate::protocol::{
    methods, AccountRateLimitsUpdatedNotification, AgentMessageDeltaNotification,
    CommandExecutionOutputDeltaNotification, CommandExecutionRequestApprovalParams,
    ErrorNotification, FileChangeOutputDeltaNotification, FileChangeRequestApprovalParams,
    ItemCompletedNotification, ItemStartedNotification, McpServerStatusUpdatedNotification,
    ReasoningSummaryTextDeltaNotification, RemoteControlStatusChangedNotification,
    ThreadStartedNotification, ThreadStatusChangedNotification,
    ThreadTokenUsageUpdatedNotification, TurnCompletedNotification, TurnStartedNotification,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// A server-to-client notification.
///
/// Each variant maps to a single `method` string on the wire. The `Unknown`
/// variant captures methods this crate version doesn't model yet, preserving
/// the raw payload for inspection.
#[derive(Debug, Clone)]
pub enum Notification {
    /// `thread/started`
    ThreadStarted(ThreadStartedNotification),
    /// `thread/status/changed`
    ThreadStatusChanged(ThreadStatusChangedNotification),
    /// `thread/tokenUsage/updated`
    ThreadTokenUsageUpdated(ThreadTokenUsageUpdatedNotification),
    /// `turn/started`
    TurnStarted(TurnStartedNotification),
    /// `turn/completed`
    TurnCompleted(TurnCompletedNotification),
    /// `item/started`
    ItemStarted(ItemStartedNotification),
    /// `item/completed`
    ItemCompleted(ItemCompletedNotification),
    /// `item/agentMessage/delta`
    AgentMessageDelta(AgentMessageDeltaNotification),
    /// `item/commandExecution/outputDelta`
    CmdOutputDelta(CommandExecutionOutputDeltaNotification),
    /// `item/fileChange/outputDelta`
    FileChangeOutputDelta(FileChangeOutputDeltaNotification),
    /// `item/reasoning/summaryTextDelta`
    ReasoningDelta(ReasoningSummaryTextDeltaNotification),
    /// `error`
    Error(ErrorNotification),
    /// `account/rateLimits/updated`
    AccountRateLimitsUpdated(AccountRateLimitsUpdatedNotification),
    /// `mcpServer/startupStatus/updated`
    McpServerStartupStatusUpdated(McpServerStatusUpdatedNotification),
    /// `remoteControl/status/changed`
    RemoteControlStatusChanged(RemoteControlStatusChangedNotification),
    /// A method this crate version does not yet model. The raw params are
    /// preserved for caller inspection. Encountering this typically means
    /// the installed codex CLI is newer than the bindings.
    Unknown {
        method: String,
        params: Option<Value>,
    },
}

impl Notification {
    /// Return the wire `method` string for this notification.
    pub fn method(&self) -> &str {
        match self {
            Self::ThreadStarted(_) => methods::THREAD_STARTED,
            Self::ThreadStatusChanged(_) => methods::THREAD_STATUS_CHANGED,
            Self::ThreadTokenUsageUpdated(_) => methods::THREAD_TOKEN_USAGE_UPDATED,
            Self::TurnStarted(_) => methods::TURN_STARTED,
            Self::TurnCompleted(_) => methods::TURN_COMPLETED,
            Self::ItemStarted(_) => methods::ITEM_STARTED,
            Self::ItemCompleted(_) => methods::ITEM_COMPLETED,
            Self::AgentMessageDelta(_) => methods::AGENT_MESSAGE_DELTA,
            Self::CmdOutputDelta(_) => methods::CMD_OUTPUT_DELTA,
            Self::FileChangeOutputDelta(_) => methods::FILE_CHANGE_OUTPUT_DELTA,
            Self::ReasoningDelta(_) => methods::REASONING_DELTA,
            Self::Error(_) => methods::ERROR,
            Self::AccountRateLimitsUpdated(_) => methods::ACCOUNT_RATE_LIMITS_UPDATED,
            Self::McpServerStartupStatusUpdated(_) => methods::MCP_SERVER_STARTUP_STATUS_UPDATED,
            Self::RemoteControlStatusChanged(_) => methods::REMOTE_CONTROL_STATUS_CHANGED,
            Self::Unknown { method, .. } => method,
        }
    }

    /// `true` if this notification's method isn't modeled by the crate.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Construct a [`Notification`] from a `method` + `params` envelope.
    ///
    /// Returns an error if `method` is recognized but `params` doesn't
    /// deserialize into the typed struct. Unknown methods route to
    /// [`Notification::Unknown`] without error.
    pub fn from_envelope(method: &str, params: Option<Value>) -> Result<Self, serde_json::Error> {
        let params_value = params.clone().unwrap_or(Value::Null);
        match method {
            methods::THREAD_STARTED => {
                serde_json::from_value(params_value).map(Self::ThreadStarted)
            }
            methods::THREAD_STATUS_CHANGED => {
                serde_json::from_value(params_value).map(Self::ThreadStatusChanged)
            }
            methods::THREAD_TOKEN_USAGE_UPDATED => {
                serde_json::from_value(params_value).map(Self::ThreadTokenUsageUpdated)
            }
            methods::TURN_STARTED => serde_json::from_value(params_value).map(Self::TurnStarted),
            methods::TURN_COMPLETED => {
                serde_json::from_value(params_value).map(Self::TurnCompleted)
            }
            methods::ITEM_STARTED => serde_json::from_value(params_value).map(Self::ItemStarted),
            methods::ITEM_COMPLETED => {
                serde_json::from_value(params_value).map(Self::ItemCompleted)
            }
            methods::AGENT_MESSAGE_DELTA => {
                serde_json::from_value(params_value).map(Self::AgentMessageDelta)
            }
            methods::CMD_OUTPUT_DELTA => {
                serde_json::from_value(params_value).map(Self::CmdOutputDelta)
            }
            methods::FILE_CHANGE_OUTPUT_DELTA => {
                serde_json::from_value(params_value).map(Self::FileChangeOutputDelta)
            }
            methods::REASONING_DELTA => {
                serde_json::from_value(params_value).map(Self::ReasoningDelta)
            }
            methods::ERROR => serde_json::from_value(params_value).map(Self::Error),
            methods::ACCOUNT_RATE_LIMITS_UPDATED => {
                serde_json::from_value(params_value).map(Self::AccountRateLimitsUpdated)
            }
            methods::MCP_SERVER_STARTUP_STATUS_UPDATED => {
                serde_json::from_value(params_value).map(Self::McpServerStartupStatusUpdated)
            }
            methods::REMOTE_CONTROL_STATUS_CHANGED => {
                serde_json::from_value(params_value).map(Self::RemoteControlStatusChanged)
            }
            _ => Ok(Self::Unknown {
                method: method.to_string(),
                params,
            }),
        }
    }

    /// Decompose this notification back into a `(method, params)` pair.
    pub fn into_envelope(self) -> Result<(String, Option<Value>), serde_json::Error> {
        fn pack<T: Serialize>(
            method: &str,
            v: &T,
        ) -> Result<(String, Option<Value>), serde_json::Error> {
            Ok((method.to_string(), Some(serde_json::to_value(v)?)))
        }
        match &self {
            Self::ThreadStarted(v) => pack(methods::THREAD_STARTED, v),
            Self::ThreadStatusChanged(v) => pack(methods::THREAD_STATUS_CHANGED, v),
            Self::ThreadTokenUsageUpdated(v) => pack(methods::THREAD_TOKEN_USAGE_UPDATED, v),
            Self::TurnStarted(v) => pack(methods::TURN_STARTED, v),
            Self::TurnCompleted(v) => pack(methods::TURN_COMPLETED, v),
            Self::ItemStarted(v) => pack(methods::ITEM_STARTED, v),
            Self::ItemCompleted(v) => pack(methods::ITEM_COMPLETED, v),
            Self::AgentMessageDelta(v) => pack(methods::AGENT_MESSAGE_DELTA, v),
            Self::CmdOutputDelta(v) => pack(methods::CMD_OUTPUT_DELTA, v),
            Self::FileChangeOutputDelta(v) => pack(methods::FILE_CHANGE_OUTPUT_DELTA, v),
            Self::ReasoningDelta(v) => pack(methods::REASONING_DELTA, v),
            Self::Error(v) => pack(methods::ERROR, v),
            Self::AccountRateLimitsUpdated(v) => pack(methods::ACCOUNT_RATE_LIMITS_UPDATED, v),
            Self::McpServerStartupStatusUpdated(v) => {
                pack(methods::MCP_SERVER_STARTUP_STATUS_UPDATED, v)
            }
            Self::RemoteControlStatusChanged(v) => pack(methods::REMOTE_CONTROL_STATUS_CHANGED, v),
            Self::Unknown { method, params } => Ok((method.clone(), params.clone())),
        }
    }
}

impl Serialize for Notification {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (method, params) = self
            .clone()
            .into_envelope()
            .map_err(serde::ser::Error::custom)?;
        let mut env = serde_json::Map::new();
        env.insert("method".to_string(), Value::String(method));
        if let Some(p) = params {
            env.insert("params".to_string(), p);
        }
        Value::Object(env).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Notification {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let method = value
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("method"))?
            .to_string();
        let params = value.get("params").cloned();
        Self::from_envelope(&method, params).map_err(serde::de::Error::custom)
    }
}

/// A server-to-client request that requires a response (approval flow).
///
/// The wire envelope carries an `id` for response correlation; that `id` is
/// held alongside this enum in [`ServerMessage::Request`] rather than embedded
/// inside the variant, since responding doesn't depend on which approval-type
/// was requested.
#[derive(Debug, Clone)]
pub enum ServerRequest {
    /// `item/commandExecution/requestApproval`
    CmdExecApproval(CommandExecutionRequestApprovalParams),
    /// `item/fileChange/requestApproval`
    FileChangeApproval(FileChangeRequestApprovalParams),
    /// A request method this crate version does not yet model.
    Unknown {
        method: String,
        params: Option<Value>,
    },
}

impl ServerRequest {
    /// Return the wire `method` string for this request.
    pub fn method(&self) -> &str {
        match self {
            Self::CmdExecApproval(_) => methods::CMD_EXEC_APPROVAL,
            Self::FileChangeApproval(_) => methods::FILE_CHANGE_APPROVAL,
            Self::Unknown { method, .. } => method,
        }
    }

    /// `true` if this request's method isn't modeled by the crate.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Construct a [`ServerRequest`] from a `method` + `params` envelope.
    pub fn from_envelope(method: &str, params: Option<Value>) -> Result<Self, serde_json::Error> {
        let params_value = params.clone().unwrap_or(Value::Null);
        match method {
            methods::CMD_EXEC_APPROVAL => {
                serde_json::from_value(params_value).map(Self::CmdExecApproval)
            }
            methods::FILE_CHANGE_APPROVAL => {
                serde_json::from_value(params_value).map(Self::FileChangeApproval)
            }
            _ => Ok(Self::Unknown {
                method: method.to_string(),
                params,
            }),
        }
    }
}

/// A message coming from the app-server.
///
/// Replaces the previous loose `{ method, params }` shape with typed enums.
/// Match on the outer variant first to distinguish notifications (no response)
/// from requests (need [`crate::AsyncClient::respond`] /
/// [`crate::SyncClient::respond`]).
#[derive(Debug, Clone)]
pub enum ServerMessage {
    /// A notification — no response required.
    Notification(Notification),
    /// A request — call `respond(id, ...)` on the client with the matching id.
    Request {
        id: RequestId,
        request: ServerRequest,
    },
}

impl ServerMessage {
    /// `true` if this message is an unmodeled method (notification or request).
    pub fn is_unknown(&self) -> bool {
        match self {
            Self::Notification(n) => n.is_unknown(),
            Self::Request { request, .. } => request.is_unknown(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_unknown_method_routes_to_unknown_variant() {
        let n = Notification::from_envelope("foo/bar", Some(serde_json::json!({"x": 1})))
            .expect("unknown methods do not error");
        match n {
            Notification::Unknown { method, params } => {
                assert_eq!(method, "foo/bar");
                assert_eq!(params, Some(serde_json::json!({"x": 1})));
            }
            other => panic!("expected Unknown, got {:?}", other),
        }
    }

    #[test]
    fn test_notification_known_method_with_bad_params_errors() {
        // thread/started expects a `thread` field — wrong shape should error.
        let err = Notification::from_envelope("thread/started", Some(serde_json::json!({})));
        assert!(err.is_err());
    }

    #[test]
    fn test_notification_round_trip_envelope() {
        let wire = serde_json::json!({
            "method": "item/agentMessage/delta",
            "params": {
                "threadId": "t1",
                "turnId": "tu1",
                "itemId": "i1",
                "delta": "hi"
            },
        });
        let n: Notification = serde_json::from_value(wire.clone()).unwrap();
        assert!(matches!(n, Notification::AgentMessageDelta(_)));
        let back = serde_json::to_value(&n).unwrap();
        assert_eq!(back, wire);
    }
}
