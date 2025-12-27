//! Message types for conversations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::MessageId;
use serde_json::Value as JsonValue;

/// The role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User/human message.
    User,
    /// Assistant/AI message.
    Assistant,
    /// System message.
    System,
    /// Tool result message.
    Tool,
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message identifier.
    pub id: MessageId,
    /// Message role.
    pub role: MessageRole,
    /// Message content.
    pub content: String,
    /// When the message was created.
    pub timestamp: DateTime<Utc>,
    /// Optional attachments or structured data.
    pub attachments: Vec<MessageAttachment>,
    /// Tool call information (for assistant messages).
    pub tool_calls: Vec<ToolCall>,
    /// Tool result (for tool messages).
    pub tool_result: Option<ToolResult>,
}

impl Message {
    /// Creates a new message.
    #[must_use]
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id: MessageId::new(),
            role,
            content: content.into(),
            timestamp: Utc::now(),
            attachments: Vec::new(),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }

    /// Creates an assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    /// Creates a system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// Creates a tool result message.
    #[must_use]
    pub fn tool(tool_call_id: impl Into<String>, result: JsonValue) -> Self {
        let mut msg = Self::new(MessageRole::Tool, "");
        msg.tool_result = Some(ToolResult {
            tool_call_id: tool_call_id.into(),
            result,
            error: None,
        });
        msg
    }

    /// Adds an attachment.
    #[must_use]
    pub fn with_attachment(mut self, attachment: MessageAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Adds a tool call.
    #[must_use]
    pub fn with_tool_call(mut self, tool_call: ToolCall) -> Self {
        self.tool_calls.push(tool_call);
        self
    }

    /// Returns true if this message has tool calls.
    #[must_use]
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// An attachment to a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    /// Attachment type.
    pub attachment_type: AttachmentType,
    /// Attachment content or reference.
    pub content: String,
    /// Optional metadata.
    pub metadata: Option<JsonValue>,
}

/// Types of attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentType {
    /// Text content.
    Text,
    /// Image (base64 or URL).
    Image,
    /// File reference.
    File,
    /// Structured data.
    Data,
}

/// A tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call.
    pub id: String,
    /// The tool name.
    pub name: String,
    /// Arguments for the tool.
    pub arguments: JsonValue,
}

impl ToolCall {
    /// Creates a new tool call.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: JsonValue) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }
}

/// Result of a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The tool call ID this result is for.
    pub tool_call_id: String,
    /// The result value.
    pub result: JsonValue,
    /// Error message if the tool failed.
    pub error: Option<String>,
}

impl ToolResult {
    /// Creates a successful tool result.
    #[must_use]
    pub fn success(tool_call_id: impl Into<String>, result: JsonValue) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            result,
            error: None,
        }
    }

    /// Creates a failed tool result.
    #[must_use]
    pub fn failure(tool_call_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            result: JsonValue::Null,
            error: Some(error.into()),
        }
    }

    /// Returns true if the tool call succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_creation() {
        let msg = Message::user("Hello!");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello!");
    }

    #[test]
    fn message_with_tool_calls() {
        let tool_call = ToolCall::new("call_1", "search", serde_json::json!({"query": "weather"}));
        let msg = Message::assistant("Let me search for that.")
            .with_tool_call(tool_call);

        assert!(msg.has_tool_calls());
        assert_eq!(msg.tool_calls.len(), 1);
        assert_eq!(msg.tool_calls[0].name, "search");
    }

    #[test]
    fn tool_result_success() {
        let result = ToolResult::success("call_1", serde_json::json!({"answer": 42}));
        assert!(result.is_success());
    }

    #[test]
    fn tool_result_failure() {
        let result = ToolResult::failure("call_1", "Connection timeout");
        assert!(!result.is_success());
        assert_eq!(result.error, Some("Connection timeout".to_string()));
    }

    #[test]
    fn message_serde_roundtrip() {
        let msg = Message::assistant("Here's the result:")
            .with_tool_call(ToolCall::new("call_1", "calc", serde_json::json!({})));

        let json = serde_json::to_string(&msg).expect("serialize");
        let parsed: Message = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(msg.id, parsed.id);
        assert_eq!(msg.content, parsed.content);
    }
}
