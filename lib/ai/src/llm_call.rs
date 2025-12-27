//! LLM Call primitive.
//!
//! The fundamental AI operation: single-shot inference with optional
//! structured output. All higher-level AI operations (Classify, Generate,
//! Summarize, etc.) are built on this primitive.

use crate::backend::{LlmRequest, LlmResponse, TokenUsage};
use crate::error::LlmError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ulid::Ulid;

/// Unique identifier for an LLM invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LlmInvocationId(Ulid);

impl LlmInvocationId {
    /// Creates a new invocation ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for LlmInvocationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LlmInvocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "llm_{}", self.0)
    }
}

/// Configuration for an LLM Call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallConfig {
    /// The prompt template or inline prompt.
    pub prompt: String,
    /// Optional system prompt.
    pub system_prompt: Option<String>,
    /// Optional output schema for structured output.
    pub output_schema: Option<JsonValue>,
    /// Temperature for sampling.
    pub temperature: Option<f32>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
}

impl LlmCallConfig {
    /// Creates a new LLM call configuration.
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            system_prompt: None,
            output_schema: None,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Adds a system prompt.
    #[must_use]
    pub fn with_system_prompt(mut self, system: impl Into<String>) -> Self {
        self.system_prompt = Some(system.into());
        self
    }

    /// Adds an output schema.
    #[must_use]
    pub fn with_output_schema(mut self, schema: JsonValue) -> Self {
        self.output_schema = Some(schema);
        self
    }
}

/// The result of an LLM Call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallResult {
    /// Unique identifier for this invocation.
    pub id: LlmInvocationId,
    /// The raw text output.
    pub content: String,
    /// Structured output (if schema was provided).
    pub structured_output: Option<JsonValue>,
    /// Token usage statistics.
    pub usage: TokenUsage,
    /// Model that generated the response.
    pub model: String,
    /// When the call was made.
    pub timestamp: DateTime<Utc>,
    /// Latency in milliseconds.
    pub latency_ms: u64,
}

impl LlmCallResult {
    /// Creates a result from an LLM response.
    #[must_use]
    pub fn from_response(response: LlmResponse, latency_ms: u64) -> Self {
        Self {
            id: LlmInvocationId::new(),
            content: response.content,
            structured_output: response.structured_output,
            usage: response.usage,
            model: response.model,
            timestamp: Utc::now(),
            latency_ms,
        }
    }
}

/// An LLM Call executor.
///
/// This is a builder for executing LLM calls with various configurations.
#[derive(Debug, Clone)]
pub struct LlmCall {
    config: LlmCallConfig,
    context: Option<JsonValue>,
}

impl LlmCall {
    /// Creates a new LLM Call with the given prompt.
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            config: LlmCallConfig::new(prompt),
            context: None,
        }
    }

    /// Creates an LLM Call from a configuration.
    #[must_use]
    pub fn from_config(config: LlmCallConfig) -> Self {
        Self {
            config,
            context: None,
        }
    }

    /// Adds a system prompt.
    #[must_use]
    pub fn with_system_prompt(mut self, system: impl Into<String>) -> Self {
        self.config.system_prompt = Some(system.into());
        self
    }

    /// Adds an output schema for structured output.
    #[must_use]
    pub fn with_output_schema(mut self, schema: JsonValue) -> Self {
        self.config.output_schema = Some(schema);
        self
    }

    /// Sets the temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.config.temperature = Some(temperature);
        self
    }

    /// Adds context data to be included in the prompt.
    #[must_use]
    pub fn with_context(mut self, context: JsonValue) -> Self {
        self.context = Some(context);
        self
    }

    /// Builds an LLM request from this configuration.
    #[must_use]
    pub fn build_request(&self) -> LlmRequest {
        let mut prompt = self.config.prompt.clone();

        // If context is provided, include it in the prompt
        if let Some(ref context) = self.context {
            prompt = format!("Context:\n{}\n\n{}", context, prompt);
        }

        let mut request = LlmRequest::new(prompt);

        if let Some(ref system) = self.config.system_prompt {
            request = request.with_system(system.clone());
        }

        if let Some(ref schema) = self.config.output_schema {
            request = request.with_output_schema(schema.clone());
        }

        if let Some(temp) = self.config.temperature {
            request = request.with_temperature(temp);
        }

        if let Some(max_tokens) = self.config.max_tokens {
            request = request.with_max_tokens(max_tokens);
        }

        request
    }
}

/// Record of an LLM invocation for audit/debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInvocationRecord {
    /// Unique identifier.
    pub id: LlmInvocationId,
    /// The request that was sent.
    pub request: LlmRequest,
    /// The response received (if successful).
    pub response: Option<LlmCallResult>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// When the invocation was made.
    pub timestamp: DateTime<Utc>,
}

impl LlmInvocationRecord {
    /// Creates a successful invocation record.
    #[must_use]
    pub fn success(request: LlmRequest, result: LlmCallResult) -> Self {
        Self {
            id: result.id,
            request,
            response: Some(result),
            error: None,
            timestamp: Utc::now(),
        }
    }

    /// Creates a failed invocation record.
    #[must_use]
    pub fn failure(request: LlmRequest, error: &LlmError) -> Self {
        Self {
            id: LlmInvocationId::new(),
            request,
            response: None,
            error: Some(error.to_string()),
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_call_builder() {
        let call = LlmCall::new("Classify this email")
            .with_system_prompt("You are a helpful classifier.")
            .with_temperature(0.3)
            .with_context(serde_json::json!({"email": "Hello world"}));

        let request = call.build_request();
        assert!(request.prompt.contains("Classify this email"));
        assert!(request.prompt.contains("Hello world"));
        assert_eq!(request.system, Some("You are a helpful classifier.".to_string()));
        assert_eq!(request.temperature, Some(0.3));
    }

    #[test]
    fn llm_call_config_serde() {
        let config = LlmCallConfig::new("Generate a summary")
            .with_system_prompt("Be concise")
            .with_output_schema(serde_json::json!({"type": "string"}));

        let json = serde_json::to_string(&config).expect("serialize");
        let parsed: LlmCallConfig = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(config.prompt, parsed.prompt);
        assert_eq!(config.system_prompt, parsed.system_prompt);
    }

    #[test]
    fn invocation_id_display() {
        let id = LlmInvocationId::new();
        let display = id.to_string();
        assert!(display.starts_with("llm_"));
    }
}
