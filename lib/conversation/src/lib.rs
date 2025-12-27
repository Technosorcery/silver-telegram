//! Conversation service for the silver-telegram platform.
//!
//! This crate provides:
//!
//! - **Session Manager**: Active conversation session lifecycle
//! - **Context Store**: Conversation history and extracted facts
//! - **Tool Registry**: Available tools during conversation

pub mod context;
pub mod message;
pub mod session;
pub mod tool;

pub use context::{ContextFact, ContextStore, FactSource};
pub use message::{Message, MessageRole};
pub use session::{Session, SessionManager, SessionState};
pub use tool::{Tool, ToolDefinition, ToolRegistry, ToolResult};
