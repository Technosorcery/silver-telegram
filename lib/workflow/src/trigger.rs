//! Trigger types for workflow initiation.
//!
//! Triggers are nodes in the workflow graph that serve as entry points.
//! They are denormalized to a separate triggers table for efficient lookup
//! during execution.

use crate::node::NodeId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::{IntegrationAccountId, TriggerId, WorkflowId};

/// The type of trigger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// Time-based trigger with cron expression.
    Schedule,
    /// HTTP webhook trigger.
    Webhook,
    /// Integration event trigger (e.g., new email).
    IntegrationEvent,
    /// Manual trigger (user-initiated).
    Manual,
}

/// Configuration for a trigger, stored in the denormalized triggers table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerConfig {
    /// Cron-style scheduled trigger.
    Schedule {
        /// Cron expression (e.g., "0 7 * * *" for 7am daily).
        cron: String,
        /// Timezone for the schedule.
        timezone: Option<String>,
        /// Next scheduled execution time (computed).
        next_run: Option<DateTime<Utc>>,
        /// Behavior for missed executions.
        missed_execution: MissedExecutionBehavior,
    },
    /// HTTP webhook trigger.
    Webhook {
        /// The webhook path (e.g., "/hooks/my-workflow").
        path: String,
        /// Optional secret for webhook validation.
        secret: Option<String>,
    },
    /// Integration event trigger.
    IntegrationEvent {
        /// The integration account ID.
        integration_id: IntegrationAccountId,
        /// The event type to listen for.
        event_type: String,
        /// Optional filter for the event.
        filter: Option<String>,
    },
    /// Manual trigger (user-initiated).
    Manual,
}

/// Behavior when a scheduled execution is missed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissedExecutionBehavior {
    /// Skip the missed execution.
    #[default]
    Skip,
    /// Run immediately when detected.
    RunImmediately,
    /// Run at the next scheduled window.
    RunAtNextWindow,
}

/// A denormalized trigger record for efficient lookup.
///
/// This is stored separately from the workflow graph for indexing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trigger {
    /// Unique identifier for this trigger.
    pub id: TriggerId,
    /// The workflow this trigger belongs to.
    pub workflow_id: WorkflowId,
    /// The node ID within the workflow graph.
    pub node_id: NodeId,
    /// Whether this trigger is currently enabled.
    pub enabled: bool,
    /// Trigger configuration.
    pub config: TriggerConfig,
    /// When this trigger was created.
    pub created_at: DateTime<Utc>,
    /// When this trigger was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Trigger {
    /// Creates a new trigger.
    #[must_use]
    pub fn new(workflow_id: WorkflowId, node_id: NodeId, config: TriggerConfig) -> Self {
        let now = Utc::now();
        Self {
            id: TriggerId::new(),
            workflow_id,
            node_id,
            enabled: true,
            config,
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns the trigger type.
    #[must_use]
    pub fn trigger_type(&self) -> TriggerType {
        match &self.config {
            TriggerConfig::Schedule { .. } => TriggerType::Schedule,
            TriggerConfig::Webhook { .. } => TriggerType::Webhook,
            TriggerConfig::IntegrationEvent { .. } => TriggerType::IntegrationEvent,
            TriggerConfig::Manual => TriggerType::Manual,
        }
    }

    /// Enables this trigger.
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
    }

    /// Disables this trigger.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_trigger_creation() {
        let workflow_id = WorkflowId::new();
        let node_id = NodeId::new();
        let config = TriggerConfig::Schedule {
            cron: "0 7 * * *".to_string(),
            timezone: Some("America/New_York".to_string()),
            next_run: None,
            missed_execution: MissedExecutionBehavior::Skip,
        };

        let trigger = Trigger::new(workflow_id, node_id, config);
        assert_eq!(trigger.trigger_type(), TriggerType::Schedule);
        assert!(trigger.enabled);
    }

    #[test]
    fn webhook_trigger_creation() {
        let workflow_id = WorkflowId::new();
        let node_id = NodeId::new();
        let config = TriggerConfig::Webhook {
            path: "/hooks/my-workflow".to_string(),
            secret: Some("secret123".to_string()),
        };

        let trigger = Trigger::new(workflow_id, node_id, config);
        assert_eq!(trigger.trigger_type(), TriggerType::Webhook);
    }

    #[test]
    fn trigger_enable_disable() {
        let workflow_id = WorkflowId::new();
        let node_id = NodeId::new();
        let config = TriggerConfig::Manual;

        let mut trigger = Trigger::new(workflow_id, node_id, config);
        assert!(trigger.enabled);

        trigger.disable();
        assert!(!trigger.enabled);

        trigger.enable();
        assert!(trigger.enabled);
    }

    #[test]
    fn trigger_serde_roundtrip() {
        let workflow_id = WorkflowId::new();
        let node_id = NodeId::new();
        let config = TriggerConfig::IntegrationEvent {
            integration_id: IntegrationAccountId::new(),
            event_type: "email.received".to_string(),
            filter: Some("from:important@example.com".to_string()),
        };

        let trigger = Trigger::new(workflow_id, node_id, config);
        let json = serde_json::to_string(&trigger).expect("serialize");
        let parsed: Trigger = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(trigger.id, parsed.id);
        assert_eq!(trigger.workflow_id, parsed.workflow_id);
    }
}
