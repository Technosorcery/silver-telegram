//! LLM backend abstraction.
//!
//! Provides a unified interface for different LLM providers (local Ollama, cloud APIs).

use crate::error::LlmError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Available LLM providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    /// Local Ollama instance.
    Ollama,
    /// Anthropic Claude API.
    Anthropic,
    /// OpenAI API.
    OpenAi,
    /// Generic OpenAI-compatible API.
    OpenAiCompatible,
}

/// Configuration for an LLM backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmBackendConfig {
    /// The provider type.
    pub provider: LlmProvider,
    /// Base URL for the API.
    pub base_url: String,
    /// Model identifier.
    pub model: String,
    /// API key (if required).
    pub api_key: Option<String>,
    /// Additional provider-specific options.
    pub options: HashMap<String, JsonValue>,
}

impl LlmBackendConfig {
    /// Creates a new Ollama backend configuration.
    #[must_use]
    pub fn ollama(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::Ollama,
            base_url: base_url.into(),
            model: model.into(),
            api_key: None,
            options: HashMap::new(),
        }
    }

    /// Creates a new Anthropic backend configuration.
    #[must_use]
    pub fn anthropic(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::Anthropic,
            base_url: "https://api.anthropic.com".to_string(),
            model: model.into(),
            api_key: Some(api_key.into()),
            options: HashMap::new(),
        }
    }
}

/// A request to an LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// The prompt/messages to send.
    pub prompt: String,
    /// System prompt, if any.
    pub system: Option<String>,
    /// Context from previous messages.
    pub context: Vec<LlmMessage>,
    /// Optional JSON schema for structured output.
    pub output_schema: Option<JsonValue>,
    /// Temperature for sampling (0.0 - 1.0).
    pub temperature: Option<f32>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
}

impl LlmRequest {
    /// Creates a new simple request with just a prompt.
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            system: None,
            context: Vec::new(),
            output_schema: None,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Adds a system prompt.
    #[must_use]
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Adds context messages.
    #[must_use]
    pub fn with_context(mut self, context: Vec<LlmMessage>) -> Self {
        self.context = context;
        self
    }

    /// Adds an output schema for structured output.
    #[must_use]
    pub fn with_output_schema(mut self, schema: JsonValue) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Sets the temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the max tokens.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    /// The role of the message sender.
    pub role: MessageRole,
    /// The content of the message.
    pub content: String,
}

impl LlmMessage {
    /// Creates a user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    /// Creates an assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

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
}

/// A response from an LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// The generated content.
    pub content: String,
    /// Structured output (if output_schema was provided).
    pub structured_output: Option<JsonValue>,
    /// Token usage statistics.
    pub usage: TokenUsage,
    /// Model that generated the response.
    pub model: String,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input tokens.
    pub input_tokens: u32,
    /// Number of output tokens.
    pub output_tokens: u32,
}

impl TokenUsage {
    /// Returns the total number of tokens.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Trait for LLM backends.
///
/// This trait defines the interface that all LLM providers must implement.
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Generates a response for the given request.
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM call fails.
    async fn generate(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Returns the provider type.
    fn provider(&self) -> LlmProvider;

    /// Returns the model name.
    fn model(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_request_builder() {
        let request = LlmRequest::new("Hello, world!")
            .with_system("You are a helpful assistant.")
            .with_temperature(0.7)
            .with_max_tokens(100);

        assert_eq!(request.prompt, "Hello, world!");
        assert_eq!(
            request.system,
            Some("You are a helpful assistant.".to_string())
        );
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
    }

    #[test]
    fn llm_message_creation() {
        let user_msg = LlmMessage::user("What is the weather?");
        assert_eq!(user_msg.role, MessageRole::User);

        let assistant_msg = LlmMessage::assistant("I don't have access to weather data.");
        assert_eq!(assistant_msg.role, MessageRole::Assistant);
    }

    #[test]
    fn token_usage_total() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
        };
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn backend_config_serde() {
        let config = LlmBackendConfig::ollama("http://localhost:11434", "llama2");
        let json = serde_json::to_string(&config).expect("serialize");
        let parsed: LlmBackendConfig = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(config.provider, parsed.provider);
        assert_eq!(config.model, parsed.model);
    }
}
