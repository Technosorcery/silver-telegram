//! Feedback tracking for AI outputs.
//!
//! Per PRD 8.7, all explicit feedback levels are available but none are required:
//! - Per-output: Correcting specific AI decisions
//! - Per-interaction: Rating conversational responses
//! - Per-workflow-run: Evaluating overall workflow behavior

use crate::llm_call::LlmInvocationId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use silver_telegram_core::{UserId, WorkflowRunId};
use ulid::Ulid;

/// Unique identifier for a feedback record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FeedbackId(Ulid);

impl FeedbackId {
    /// Creates a new feedback ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for FeedbackId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FeedbackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fb_{}", self.0)
    }
}

/// The level/granularity of feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackLevel {
    /// Feedback on a specific AI output (e.g., classification was wrong).
    PerOutput,
    /// Feedback on a conversational interaction.
    PerInteraction,
    /// Feedback on an entire workflow run.
    PerWorkflowRun,
}

/// The type of feedback signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSignal {
    /// The output was correct/helpful.
    Positive,
    /// The output was incorrect/unhelpful.
    Negative,
    /// The output was modified by the user.
    Modified,
}

/// A feedback record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    /// Unique identifier.
    pub id: FeedbackId,
    /// The user who provided the feedback.
    pub user_id: UserId,
    /// Feedback level.
    pub level: FeedbackLevel,
    /// Feedback signal.
    pub signal: FeedbackSignal,
    /// Reference to the target of feedback.
    pub target: FeedbackTarget,
    /// Optional correction (what the output should have been).
    pub correction: Option<JsonValue>,
    /// Optional comment from the user.
    pub comment: Option<String>,
    /// When the feedback was provided.
    pub created_at: DateTime<Utc>,
}

/// The target of feedback (what the feedback is about).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FeedbackTarget {
    /// Feedback on an LLM invocation output.
    LlmOutput {
        invocation_id: LlmInvocationId,
        /// The original output that was evaluated.
        original_output: JsonValue,
    },
    /// Feedback on a workflow run.
    WorkflowRun {
        run_id: WorkflowRunId,
    },
    /// Feedback on a conversation message.
    ConversationMessage {
        session_id: String,
        message_id: String,
    },
}

impl Feedback {
    /// Creates positive feedback on an LLM output.
    #[must_use]
    pub fn positive_llm_output(user_id: UserId, invocation_id: LlmInvocationId, output: JsonValue) -> Self {
        Self {
            id: FeedbackId::new(),
            user_id,
            level: FeedbackLevel::PerOutput,
            signal: FeedbackSignal::Positive,
            target: FeedbackTarget::LlmOutput {
                invocation_id,
                original_output: output,
            },
            correction: None,
            comment: None,
            created_at: Utc::now(),
        }
    }

    /// Creates negative feedback on an LLM output with a correction.
    #[must_use]
    pub fn negative_llm_output(
        user_id: UserId,
        invocation_id: LlmInvocationId,
        output: JsonValue,
        correction: JsonValue,
    ) -> Self {
        Self {
            id: FeedbackId::new(),
            user_id,
            level: FeedbackLevel::PerOutput,
            signal: FeedbackSignal::Negative,
            target: FeedbackTarget::LlmOutput {
                invocation_id,
                original_output: output,
            },
            correction: Some(correction),
            comment: None,
            created_at: Utc::now(),
        }
    }

    /// Creates feedback on a workflow run.
    #[must_use]
    pub fn workflow_run(user_id: UserId, run_id: WorkflowRunId, signal: FeedbackSignal) -> Self {
        Self {
            id: FeedbackId::new(),
            user_id,
            level: FeedbackLevel::PerWorkflowRun,
            signal,
            target: FeedbackTarget::WorkflowRun { run_id },
            correction: None,
            comment: None,
            created_at: Utc::now(),
        }
    }

    /// Adds a comment to the feedback.
    #[must_use]
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }
}

/// Trait for storing and retrieving feedback.
pub trait FeedbackStore: Send + Sync {
    /// Stores a feedback record.
    fn store(
        &self,
        feedback: Feedback,
    ) -> impl std::future::Future<Output = Result<(), silver_telegram_core::AiError>> + Send;

    /// Retrieves feedback for an LLM invocation.
    fn get_for_invocation(
        &self,
        invocation_id: LlmInvocationId,
    ) -> impl std::future::Future<Output = Result<Vec<Feedback>, silver_telegram_core::AiError>> + Send;

    /// Retrieves feedback for a workflow run.
    fn get_for_workflow_run(
        &self,
        run_id: WorkflowRunId,
    ) -> impl std::future::Future<Output = Result<Vec<Feedback>, silver_telegram_core::AiError>> + Send;

    /// Gets aggregate statistics for a user's feedback.
    fn get_user_stats(
        &self,
        user_id: UserId,
    ) -> impl std::future::Future<Output = Result<FeedbackStats, silver_telegram_core::AiError>> + Send;
}

/// Aggregate statistics about feedback.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedbackStats {
    /// Total feedback records.
    pub total_count: u64,
    /// Positive feedback count.
    pub positive_count: u64,
    /// Negative feedback count.
    pub negative_count: u64,
    /// Modified feedback count.
    pub modified_count: u64,
    /// Breakdown by level.
    pub by_level: LevelBreakdown,
}

/// Breakdown of feedback by level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelBreakdown {
    /// Per-output feedback count.
    pub per_output: u64,
    /// Per-interaction feedback count.
    pub per_interaction: u64,
    /// Per-workflow-run feedback count.
    pub per_workflow_run: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_feedback_creation() {
        let user_id = UserId::new();
        let invocation_id = LlmInvocationId::new();
        let output = serde_json::json!({"category": "spam"});

        let feedback = Feedback::positive_llm_output(user_id, invocation_id, output);

        assert_eq!(feedback.signal, FeedbackSignal::Positive);
        assert_eq!(feedback.level, FeedbackLevel::PerOutput);
        assert!(feedback.correction.is_none());
    }

    #[test]
    fn negative_feedback_with_correction() {
        let user_id = UserId::new();
        let invocation_id = LlmInvocationId::new();
        let output = serde_json::json!({"category": "spam"});
        let correction = serde_json::json!({"category": "important"});

        let feedback = Feedback::negative_llm_output(user_id, invocation_id, output, correction.clone());

        assert_eq!(feedback.signal, FeedbackSignal::Negative);
        assert_eq!(feedback.correction, Some(correction));
    }

    #[test]
    fn feedback_with_comment() {
        let user_id = UserId::new();
        let run_id = WorkflowRunId::new();

        let feedback = Feedback::workflow_run(user_id, run_id, FeedbackSignal::Positive)
            .with_comment("Great results!");

        assert_eq!(feedback.comment, Some("Great results!".to_string()));
    }

    #[test]
    fn feedback_serde_roundtrip() {
        let user_id = UserId::new();
        let invocation_id = LlmInvocationId::new();
        let output = serde_json::json!({"result": "test"});

        let feedback = Feedback::positive_llm_output(user_id, invocation_id, output);

        let json = serde_json::to_string(&feedback).expect("serialize");
        let parsed: Feedback = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(feedback.id, parsed.id);
        assert_eq!(feedback.signal, parsed.signal);
    }
}
