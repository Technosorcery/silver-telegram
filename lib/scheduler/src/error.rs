//! Error types for the scheduler crate.
//!
//! Errors are designed for layered context using rootcause:
//! - `TriggerError`: Errors from trigger operations
//! - `ScheduleError`: Errors from schedule operations
//! - `SchedulerError`: High-level wrapper for context

use silver_telegram_core::TriggerId;
use std::fmt;

/// Errors from trigger operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerError {
    /// Trigger not found.
    NotFound { id: TriggerId },
    /// Trigger already exists.
    AlreadyExists { id: TriggerId },
    /// Storage operation failed.
    StorageFailed { reason: String },
    /// Invalid trigger configuration.
    InvalidConfig { reason: String },
}

impl fmt::Display for TriggerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "trigger not found: {id}"),
            Self::AlreadyExists { id } => write!(f, "trigger already exists: {id}"),
            Self::StorageFailed { reason } => {
                write!(f, "trigger storage failed: {reason}")
            }
            Self::InvalidConfig { reason } => {
                write!(f, "invalid trigger config: {reason}")
            }
        }
    }
}

impl std::error::Error for TriggerError {}

/// Errors from schedule operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// Invalid cron expression.
    InvalidCronExpression { expression: String, reason: String },
    /// Schedule evaluation failed.
    EvaluationFailed { reason: String },
    /// Invalid timezone.
    InvalidTimezone { timezone: String },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCronExpression { expression, reason } => {
                write!(f, "invalid cron expression '{expression}': {reason}")
            }
            Self::EvaluationFailed { reason } => {
                write!(f, "schedule evaluation failed: {reason}")
            }
            Self::InvalidTimezone { timezone } => {
                write!(f, "invalid timezone: {timezone}")
            }
        }
    }
}

impl std::error::Error for ScheduleError {}

/// High-level scheduler errors.
///
/// Use these to add context when wrapping lower-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    /// Trigger operation context (use as context wrapper).
    TriggerOperation { trigger_id: TriggerId },
    /// Registration failed.
    RegistrationFailed { reason: String },
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TriggerOperation { trigger_id } => {
                write!(f, "trigger operation failed for {trigger_id}")
            }
            Self::RegistrationFailed { reason } => {
                write!(f, "trigger registration failed: {reason}")
            }
        }
    }
}

impl std::error::Error for SchedulerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_error_display() {
        let id = TriggerId::new();
        let err = TriggerError::NotFound { id };
        assert!(err.to_string().contains("trigger not found"));
    }

    #[test]
    fn schedule_error_display() {
        let err = ScheduleError::InvalidCronExpression {
            expression: "invalid".to_string(),
            reason: "expected 5 parts".to_string(),
        };
        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains("5 parts"));
    }

    #[test]
    fn scheduler_error_display() {
        let id = TriggerId::new();
        let err = SchedulerError::TriggerOperation { trigger_id: id };
        assert!(err.to_string().contains("trigger operation failed"));
    }
}
