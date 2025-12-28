//! AI primitives for the silver-telegram platform.
//!
//! This crate provides the two fundamental AI primitives:
//!
//! - **LLM Call**: Single-shot inference with optional structured output
//! - **Coordinate**: LLM-driven execution loop for complex tasks
//!
//! Higher-level operations (Classify, Generate, Summarize, etc.) are built
//! on top of LLM Call with specialized prompts and output schemas.

pub mod backend;
pub mod coordinate;
pub mod error;
pub mod feedback;
pub mod llm_call;

pub use backend::{LlmBackend, LlmProvider, LlmRequest, LlmResponse};
pub use coordinate::{CoordinateConfig, CoordinateResult, CoordinateStep, Coordinator};
pub use error::{AiError, CoordinateError, FeedbackError, LlmError};
pub use feedback::{Feedback, FeedbackLevel, FeedbackStore};
pub use llm_call::{LlmCall, LlmCallConfig, LlmCallResult};
