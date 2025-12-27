//! Scheduler for workflow triggers.
//!
//! This crate provides:
//!
//! - **Trigger Manager**: Registration and lookup of triggers
//! - **Scheduler**: Cron-based scheduling with missed execution handling
//! - **Event Router**: Routing integration events to workflows

pub mod error;
pub mod manager;
pub mod schedule;

pub use error::{ScheduleError, SchedulerError, TriggerError};
pub use manager::{TriggerManager, TriggerRecord};
pub use schedule::{CronSchedule, ScheduleEvaluator, ScheduledExecution};
