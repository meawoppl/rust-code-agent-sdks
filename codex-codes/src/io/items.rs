//! Thread item types shared between the exec and app-server protocols.
//!
//! A [`ThreadItem`] represents a single unit of work within a turn — an agent
//! message, a command execution, a file change, etc. Items are emitted via
//! `item/started` and `item/completed` notifications and included in the
//! final [`Turn`](crate::Turn) when a turn completes.
//!
//! Both snake_case (exec protocol) and camelCase (app-server protocol) type
//! tags are accepted via serde aliases.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Status of a command execution within a [`CommandExecutionItem`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandExecutionStatus {
    /// The command is currently running.
    #[serde(alias = "inProgress")]
    InProgress,
    /// The command finished successfully.
    #[serde(alias = "completed")]
    Completed,
    /// The command failed (non-zero exit code or error).
    #[serde(alias = "failed")]
    Failed,
    /// The user declined the approval request for this command.
    #[serde(alias = "declined")]
    Declined,
}

/// A command execution item — a shell command the agent ran.
///
/// The exec JSONL protocol uses snake_case (`aggregated_output`, `exit_code`)
/// while the app-server protocol uses camelCase (`aggregatedOutput`, `exitCode`)
/// and may emit `null` for missing output. Fields below carry serde aliases so
/// both formats deserialize cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionItem {
    pub id: String,
    /// The shell command that was executed.
    pub command: String,
    /// Combined stdout/stderr output from the command. `None` while still in
    /// progress on the app-server protocol; the exec protocol uses an empty
    /// string for the same state.
    #[serde(alias = "aggregated_output", default)]
    pub aggregated_output: Option<String>,
    /// Exit code, if the command has finished.
    #[serde(alias = "exit_code", default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub status: CommandExecutionStatus,
}

/// Kind of patch change applied to a file.
///
/// Internally-tagged on the wire under the `type` discriminator
/// (`{"type":"add"}`, `{"type":"delete"}`, `{"type":"update","move_path":...}`).
/// Older codex versions emitted bare strings (`"update"`); those are no longer
/// accepted — regenerate test fixtures against a current codex CLI if needed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatchChangeKind {
    Add,
    Delete,
    Update {
        /// Set when the patch renames the file to this path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        move_path: Option<String>,
    },
}

/// A single file update within a file change item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUpdateChange {
    pub path: String,
    pub kind: PatchChangeKind,
    /// Unified-diff snippet describing the change.
    #[serde(default)]
    pub diff: String,
}

/// Status of a patch apply operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PatchApplyStatus {
    /// Patch is currently being applied. Surfaced on `item/started`.
    #[serde(alias = "in_progress")]
    InProgress,
    #[serde(alias = "completed")]
    Completed,
    #[serde(alias = "failed")]
    Failed,
    /// The user declined the file-change approval. Surfaced on
    /// `item/completed` after a rejected approval.
    #[serde(alias = "declined")]
    Declined,
}

/// A file change item representing one or more file modifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeItem {
    pub id: String,
    pub changes: Vec<FileUpdateChange>,
    pub status: PatchApplyStatus,
}

/// Status of an MCP tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolCallStatus {
    #[serde(alias = "inProgress")]
    InProgress,
    #[serde(alias = "completed")]
    Completed,
    #[serde(alias = "failed")]
    Failed,
}

/// Result of an MCP tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallResult {
    pub content: Vec<Value>,
    pub structured_content: Value,
}

/// Error from an MCP tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallError {
    pub message: String,
}

/// An MCP tool call item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallItem {
    pub id: String,
    pub server: String,
    pub tool: String,
    pub arguments: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<McpToolCallResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpToolCallError>,
    pub status: McpToolCallStatus,
}

/// An agent message item containing text output.
///
/// `text` may be empty (or absent on the wire) for `item/started` events on
/// the app-server protocol — codex emits the message envelope before any
/// tokens have been generated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageItem {
    pub id: String,
    #[serde(default)]
    pub text: String,
}

/// A single content block within a [`UserMessageItem`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageContent {
    /// Block kind tag (e.g. `"text"`).
    #[serde(rename = "type")]
    pub kind: String,
    /// The text content.
    #[serde(default)]
    pub text: String,
    /// Tokenized/structured representation of the text. Shape varies by
    /// codex version, so it's preserved as raw JSON.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_elements: Vec<Value>,
}

/// A user message item — the prompt the user sent for the current turn.
///
/// Emitted by the app-server as the first item in a turn. The exec JSONL
/// protocol does not typically emit this item kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageItem {
    pub id: String,
    pub content: Vec<UserMessageContent>,
}

/// A reasoning item containing the model's chain-of-thought.
///
/// `text` may be empty on `item/started` events; populated by the time
/// `item/completed` arrives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningItem {
    pub id: String,
    #[serde(default)]
    pub text: String,
}

/// A web search item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchItem {
    pub id: String,
    pub query: String,
}

/// An error item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorItem {
    pub id: String,
    pub message: String,
}

/// A single todo entry within a todo list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub text: String,
    pub completed: bool,
}

/// A todo list item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoListItem {
    pub id: String,
    pub items: Vec<TodoItem>,
}

/// All possible thread item types emitted by the Codex CLI.
///
/// Items are the core building blocks of a turn. Each variant represents
/// a different kind of work the agent performed. Items arrive via
/// `item/started` and `item/completed` notifications and are collected
/// in the final [`Turn`](crate::Turn).
///
/// Accepts both snake_case (`agent_message`) and camelCase (`agentMessage`)
/// type tags for compatibility with the exec and app-server protocols.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThreadItem {
    /// The user's prompt for the turn (app-server protocol only).
    #[serde(alias = "userMessage")]
    UserMessage(UserMessageItem),
    /// Text output from the agent.
    #[serde(alias = "agentMessage")]
    AgentMessage(AgentMessageItem),
    /// Chain-of-thought reasoning from the model.
    #[serde(alias = "reasoning")]
    Reasoning(ReasoningItem),
    /// A shell command the agent executed.
    #[serde(alias = "commandExecution")]
    CommandExecution(CommandExecutionItem),
    /// File modifications the agent applied.
    #[serde(alias = "fileChange")]
    FileChange(FileChangeItem),
    /// An MCP tool call to an external server.
    #[serde(alias = "mcpToolCall")]
    McpToolCall(McpToolCallItem),
    /// A web search the agent performed.
    #[serde(alias = "webSearch")]
    WebSearch(WebSearchItem),
    /// A todo list the agent maintains.
    #[serde(alias = "todoList")]
    TodoList(TodoListItem),
    /// An error that occurred during processing.
    #[serde(alias = "error")]
    Error(ErrorItem),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_agent_message() {
        let json = r#"{"type":"agent_message","id":"msg_1","text":"Hello world"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::AgentMessage(ref m) if m.text == "Hello world"));
    }

    #[test]
    fn test_deserialize_command_execution() {
        let json = r#"{"type":"command_execution","id":"cmd_1","command":"ls -la","aggregated_output":"total 0","exit_code":0,"status":"completed"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::CommandExecution(ref c) if c.exit_code == Some(0)));
    }

    #[test]
    fn test_deserialize_file_change() {
        let json = r#"{"type":"file_change","id":"fc_1","changes":[{"path":"src/main.rs","kind":{"type":"update"},"diff":"@@ -1 +1 @@\n-a\n+b\n"}],"status":"completed"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(
            item,
            ThreadItem::FileChange(ref f) if matches!(f.changes[0].kind, PatchChangeKind::Update { .. })
        ));
    }

    #[test]
    fn test_deserialize_todo_list() {
        let json = r#"{"type":"todo_list","id":"td_1","items":[{"text":"Fix bug","completed":false},{"text":"Write tests","completed":true}]}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::TodoList(ref t) if t.items.len() == 2));
    }

    #[test]
    fn test_deserialize_error() {
        let json = r#"{"type":"error","id":"err_1","message":"something went wrong"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::Error(ref e) if e.message == "something went wrong"));
    }

    #[test]
    fn test_deserialize_reasoning() {
        let json = r#"{"type":"reasoning","id":"r_1","text":"Let me think about this..."}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::Reasoning(ref r) if r.text.contains("think")));
    }

    #[test]
    fn test_deserialize_web_search() {
        let json = r#"{"type":"web_search","id":"ws_1","query":"rust serde tutorial"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::WebSearch(ref w) if w.query == "rust serde tutorial"));
    }

    #[test]
    fn test_deserialize_mcp_tool_call() {
        let json = r#"{"type":"mcp_tool_call","id":"mcp_1","server":"my-server","tool":"my-tool","arguments":{"key":"value"},"status":"completed","result":{"content":[],"structured_content":null}}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::McpToolCall(ref m) if m.tool == "my-tool"));
    }

    #[test]
    fn test_deserialize_camel_case_agent_message() {
        let json = r#"{"type":"agentMessage","id":"msg_1","text":"Hello"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::AgentMessage(ref m) if m.text == "Hello"));
    }

    #[test]
    fn test_deserialize_camel_case_command_execution() {
        let json = r#"{"type":"commandExecution","id":"cmd_1","command":"ls","aggregated_output":"","status":"completed"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::CommandExecution(_)));
    }

    #[test]
    fn test_deserialize_camel_case_file_change() {
        let json = r#"{"type":"fileChange","id":"fc_1","changes":[],"status":"completed"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::FileChange(_)));
    }

    #[test]
    fn test_command_execution_status_declined() {
        let json = r#"{"type":"command_execution","id":"cmd_1","command":"rm -rf /","aggregated_output":"","status":"declined"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(
            matches!(item, ThreadItem::CommandExecution(ref c) if c.status == CommandExecutionStatus::Declined)
        );
    }
}
