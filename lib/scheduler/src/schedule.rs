//! Cron-based scheduling with missed execution handling.

use crate::error::ScheduleError;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::{TriggerId, WorkflowId};
use silver_telegram_workflow::trigger::MissedExecutionBehavior;
use ulid::Ulid;

/// A parsed cron schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSchedule {
    /// The cron expression.
    pub expression: String,
    /// Timezone for evaluation.
    pub timezone: Option<String>,
}

impl CronSchedule {
    /// Creates a new cron schedule.
    #[must_use]
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            timezone: None,
        }
    }

    /// Sets the timezone.
    #[must_use]
    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }

    /// Validates the cron expression.
    ///
    /// # Errors
    ///
    /// Returns an error if the expression is invalid.
    pub fn validate(&self) -> Result<(), ScheduleError> {
        // Basic validation - in production, use a proper cron parser
        let parts: Vec<&str> = self.expression.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(ScheduleError::InvalidCronExpression {
                expression: self.expression.clone(),
                reason: format!("expected 5 parts, got {}", parts.len()),
            });
        }
        Ok(())
    }

    /// Calculates the next execution time after the given time.
    ///
    /// Note: This is a placeholder. A production implementation would use
    /// a proper cron parsing library.
    #[must_use]
    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Placeholder: return 1 hour after the given time
        // In production, use a cron library like `cron` or `croner`
        Some(after + Duration::hours(1))
    }
}

/// A scheduled execution record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledExecution {
    /// Unique identifier.
    pub id: ScheduledExecutionId,
    /// The trigger that created this execution.
    pub trigger_id: TriggerId,
    /// The workflow to execute.
    pub workflow_id: WorkflowId,
    /// When this execution is scheduled for.
    pub scheduled_for: DateTime<Utc>,
    /// Current status.
    pub status: ExecutionStatus,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When execution started (if it has).
    pub started_at: Option<DateTime<Utc>>,
    /// When execution completed (if it has).
    pub completed_at: Option<DateTime<Utc>>,
}

/// Unique identifier for a scheduled execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScheduledExecutionId(Ulid);

impl ScheduledExecutionId {
    /// Creates a new execution ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for ScheduledExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ScheduledExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sched_{}", self.0)
    }
}

/// Status of a scheduled execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Waiting for scheduled time.
    Pending,
    /// Ready to execute (scheduled time has passed).
    Ready,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed.
    Failed,
    /// Skipped (e.g., missed execution with skip policy).
    Skipped,
}

impl ScheduledExecution {
    /// Creates a new scheduled execution.
    #[must_use]
    pub fn new(
        trigger_id: TriggerId,
        workflow_id: WorkflowId,
        scheduled_for: DateTime<Utc>,
    ) -> Self {
        Self {
            id: ScheduledExecutionId::new(),
            trigger_id,
            workflow_id,
            scheduled_for,
            status: ExecutionStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Checks if this execution is ready to run.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.status == ExecutionStatus::Pending && Utc::now() >= self.scheduled_for
    }

    /// Checks if this execution was missed.
    #[must_use]
    pub fn is_missed(&self, threshold: Duration) -> bool {
        self.status == ExecutionStatus::Pending && Utc::now() > self.scheduled_for + threshold
    }

    /// Marks the execution as started.
    pub fn start(&mut self) {
        self.status = ExecutionStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Marks the execution as completed.
    pub fn complete(&mut self) {
        self.status = ExecutionStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Marks the execution as failed.
    pub fn fail(&mut self) {
        self.status = ExecutionStatus::Failed;
        self.completed_at = Some(Utc::now());
    }

    /// Marks the execution as skipped.
    pub fn skip(&mut self) {
        self.status = ExecutionStatus::Skipped;
        self.completed_at = Some(Utc::now());
    }
}

/// Evaluates schedules and handles missed executions.
#[async_trait]
pub trait ScheduleEvaluator: Send + Sync {
    /// Gets executions that are ready to run.
    async fn get_ready_executions(&self) -> Result<Vec<ScheduledExecution>, ScheduleError>;

    /// Creates the next scheduled execution for a trigger.
    async fn schedule_next(
        &self,
        trigger_id: TriggerId,
        workflow_id: WorkflowId,
        schedule: &CronSchedule,
    ) -> Result<ScheduledExecution, ScheduleError>;

    /// Handles missed executions based on policy.
    async fn handle_missed_executions(
        &self,
        trigger_id: TriggerId,
        behavior: MissedExecutionBehavior,
    ) -> Result<Vec<ScheduledExecution>, ScheduleError>;

    /// Updates execution status.
    async fn update_execution(&self, execution: ScheduledExecution) -> Result<(), ScheduleError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_schedule_creation() {
        let schedule = CronSchedule::new("0 7 * * *").with_timezone("America/New_York");

        assert_eq!(schedule.expression, "0 7 * * *");
        assert_eq!(schedule.timezone, Some("America/New_York".to_string()));
    }

    #[test]
    fn cron_schedule_validation() {
        let valid = CronSchedule::new("0 7 * * *");
        assert!(valid.validate().is_ok());

        let invalid = CronSchedule::new("invalid");
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn scheduled_execution_lifecycle() {
        let trigger_id = TriggerId::new();
        let workflow_id = WorkflowId::new();
        let scheduled_for = Utc::now() - Duration::minutes(5);

        let mut execution = ScheduledExecution::new(trigger_id, workflow_id, scheduled_for);
        assert!(execution.is_ready());
        assert_eq!(execution.status, ExecutionStatus::Pending);

        execution.start();
        assert_eq!(execution.status, ExecutionStatus::Running);
        assert!(execution.started_at.is_some());

        execution.complete();
        assert_eq!(execution.status, ExecutionStatus::Completed);
        assert!(execution.completed_at.is_some());
    }

    #[test]
    fn scheduled_execution_missed() {
        let trigger_id = TriggerId::new();
        let workflow_id = WorkflowId::new();
        let scheduled_for = Utc::now() - Duration::hours(2);

        let execution = ScheduledExecution::new(trigger_id, workflow_id, scheduled_for);

        assert!(execution.is_missed(Duration::hours(1)));
        assert!(!execution.is_missed(Duration::hours(3)));
    }

    #[test]
    fn execution_id_display() {
        let id = ScheduledExecutionId::new();
        let display = id.to_string();
        assert!(display.starts_with("sched_"));
    }
}
