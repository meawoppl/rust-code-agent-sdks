use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;

/// Deserialize content blocks that can be either a string or array
pub(crate) fn deserialize_content_blocks<'de, D>(
    deserializer: D,
) -> Result<Vec<ContentBlock>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(vec![ContentBlock::Text(TextBlock { text: s })]),
        Value::Array(_) => serde_json::from_value(value).map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom(
            "content must be a string or array",
        )),
    }
}

/// Content blocks for messages
///
/// Includes typed variants for known block types and an `Unknown` fallback
/// for forward compatibility with new block types added by the CLI.
#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text(TextBlock),
    Image(ImageBlock),
    Thinking(ThinkingBlock),
    ToolUse(ToolUseBlock),
    ToolResult(ToolResultBlock),
    /// A content block type not yet known to this version of the crate.
    /// Contains the raw JSON value for caller inspection.
    Unknown(Value),
}

impl ContentBlock {
    /// Returns the type tag string for this content block.
    pub fn block_type(&self) -> &str {
        match self {
            Self::Text(_) => "text",
            Self::Image(_) => "image",
            Self::Thinking(_) => "thinking",
            Self::ToolUse(_) => "tool_use",
            Self::ToolResult(_) => "tool_result",
            Self::Unknown(v) => v.get("type").and_then(|t| t.as_str()).unwrap_or("unknown"),
        }
    }

    /// Returns `true` if this is an unknown/unrecognized content block type.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown(_))
    }
}

impl Serialize for ContentBlock {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Text(v) => serialize_tagged("text", v, serializer),
            Self::Image(v) => serialize_tagged("image", v, serializer),
            Self::Thinking(v) => serialize_tagged("thinking", v, serializer),
            Self::ToolUse(v) => serialize_tagged("tool_use", v, serializer),
            Self::ToolResult(v) => serialize_tagged("tool_result", v, serializer),
            Self::Unknown(v) => v.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ContentBlock {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let type_str = value
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("type"))?;

        match type_str {
            "text" => serde_json::from_value(value)
                .map(ContentBlock::Text)
                .map_err(serde::de::Error::custom),
            "image" => serde_json::from_value(value)
                .map(ContentBlock::Image)
                .map_err(serde::de::Error::custom),
            "thinking" => serde_json::from_value(value)
                .map(ContentBlock::Thinking)
                .map_err(serde::de::Error::custom),
            "tool_use" => serde_json::from_value(value)
                .map(ContentBlock::ToolUse)
                .map_err(serde::de::Error::custom),
            "tool_result" => serde_json::from_value(value)
                .map(ContentBlock::ToolResult)
                .map_err(serde::de::Error::custom),
            _ => Ok(ContentBlock::Unknown(value)),
        }
    }
}

/// Serialize a value with an internally-tagged "type" field.
fn serialize_tagged<S: Serializer, T: Serialize>(
    tag: &str,
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut map = serde_json::to_value(value).map_err(serde::ser::Error::custom)?;
    if let Some(obj) = map.as_object_mut() {
        obj.insert("type".to_string(), Value::String(tag.to_string()));
    }
    map.serialize(serializer)
}

/// Text content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
}

/// Image content block (follows Anthropic API structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBlock {
    pub source: ImageSource,
}

/// Encoding type for image source data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageSourceType {
    /// Base64-encoded image data.
    Base64,
    /// A source type not yet known to this version of the crate.
    Unknown(String),
}

impl ImageSourceType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Base64 => "base64",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for ImageSourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ImageSourceType {
    fn from(s: &str) -> Self {
        match s {
            "base64" => Self::Base64,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for ImageSourceType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ImageSourceType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// MIME type for image content.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MediaType {
    /// JPEG image.
    Jpeg,
    /// PNG image.
    Png,
    /// GIF image.
    Gif,
    /// WebP image.
    Webp,
    /// A media type not yet known to this version of the crate.
    Unknown(String),
}

impl MediaType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for MediaType {
    fn from(s: &str) -> Self {
        match s {
            "image/jpeg" => Self::Jpeg,
            "image/png" => Self::Png,
            "image/gif" => Self::Gif,
            "image/webp" => Self::Webp,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for MediaType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Image source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: ImageSourceType,
    pub media_type: MediaType,
    pub data: String,
}

/// Thinking content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBlock {
    pub thinking: String,
    pub signature: String,
}

/// Tool use content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: Value,
}

impl ToolUseBlock {
    /// Try to parse the input as a typed ToolInput.
    ///
    /// This attempts to deserialize the raw JSON input into a strongly-typed
    /// `ToolInput` enum variant. Returns `None` if parsing fails.
    ///
    /// # Example
    ///
    /// ```
    /// use claude_codes::{ToolUseBlock, ToolInput};
    /// use serde_json::json;
    ///
    /// let block = ToolUseBlock {
    ///     id: "toolu_123".to_string(),
    ///     name: "Bash".to_string(),
    ///     input: json!({"command": "ls -la"}),
    /// };
    ///
    /// if let Some(ToolInput::Bash(bash)) = block.typed_input() {
    ///     assert_eq!(bash.command, "ls -la");
    /// }
    /// ```
    pub fn typed_input(&self) -> Option<crate::tool_inputs::ToolInput> {
        serde_json::from_value(self.input.clone()).ok()
    }

    /// Parse the input as a typed ToolInput, returning an error on failure.
    ///
    /// Unlike `typed_input()`, this method returns the parsing error for debugging.
    pub fn try_typed_input(&self) -> Result<crate::tool_inputs::ToolInput, serde_json::Error> {
        serde_json::from_value(self.input.clone())
    }
}

/// Tool result content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Tool result content type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(String),
    Structured(Vec<Value>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_unknown_content_block_deserializes() {
        let json = json!({
            "type": "server_tool_use",
            "id": "srvtu_1",
            "name": "web_search",
            "input": {"query": "rust serde"}
        });

        let block: ContentBlock = serde_json::from_value(json.clone()).unwrap();
        assert!(block.is_unknown());
        assert_eq!(block.block_type(), "server_tool_use");
        if let ContentBlock::Unknown(v) = &block {
            assert_eq!(v["id"], "srvtu_1");
            assert_eq!(v["name"], "web_search");
        } else {
            panic!("Expected Unknown variant");
        }
    }

    #[test]
    fn test_unknown_block_roundtrips() {
        let json = json!({
            "type": "web_search_tool_result",
            "tool_use_id": "srvtu_1",
            "content": [{"type": "web_search_result", "url": "https://example.com"}]
        });

        let block: ContentBlock = serde_json::from_value(json.clone()).unwrap();
        let reserialized = serde_json::to_value(&block).unwrap();
        assert_eq!(json, reserialized);
    }

    #[test]
    fn test_known_blocks_still_work() {
        let text_json = json!({"type": "text", "text": "hello"});
        let block: ContentBlock = serde_json::from_value(text_json).unwrap();
        assert!(!block.is_unknown());
        assert_eq!(block.block_type(), "text");
        assert!(matches!(block, ContentBlock::Text(TextBlock { text }) if text == "hello"));

        let tool_json =
            json!({"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {"command": "ls"}});
        let block: ContentBlock = serde_json::from_value(tool_json).unwrap();
        assert_eq!(block.block_type(), "tool_use");
        assert!(matches!(block, ContentBlock::ToolUse(_)));
    }

    #[test]
    fn test_known_blocks_roundtrip() {
        let text_json = json!({"type": "text", "text": "hello world"});
        let block: ContentBlock = serde_json::from_value(text_json.clone()).unwrap();
        let reserialized = serde_json::to_value(&block).unwrap();
        assert_eq!(text_json, reserialized);
    }

    #[test]
    fn test_assistant_message_with_unknown_block_survives() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [
                    {"type": "text", "text": "Let me search for that."},
                    {"type": "server_tool_use", "id": "srvtu_1", "name": "web_search", "input": {"query": "test"}},
                    {"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {"command": "ls"}}
                ]
            },
            "session_id": "abc"
        }"#;

        let output: crate::io::ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_assistant_message());
        let assistant = output.as_assistant().unwrap();
        assert_eq!(assistant.message.content.len(), 3);
        assert!(matches!(
            &assistant.message.content[0],
            ContentBlock::Text(_)
        ));
        assert!(matches!(
            &assistant.message.content[1],
            ContentBlock::Unknown(_)
        ));
        assert!(matches!(
            &assistant.message.content[2],
            ContentBlock::ToolUse(_)
        ));

        // text_content() still works, skipping unknown blocks
        assert_eq!(
            output.text_content(),
            Some("Let me search for that.".to_string())
        );
        // tool_uses() still works
        assert_eq!(output.tool_uses().count(), 1);
    }

    #[test]
    fn test_missing_type_field_errors() {
        let json = json!({"text": "no type field"});
        let result = serde_json::from_value::<ContentBlock>(json);
        assert!(result.is_err());
    }
}
