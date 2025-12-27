//! Trigger manager for workflow triggers.
//!
//! Manages the denormalized trigger table for efficient lookup.

use crate::error::TriggerError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::{IntegrationAccountId, TriggerId, WorkflowId};
use silver_telegram_workflow::trigger::{Trigger, TriggerConfig, TriggerType};
use silver_telegram_workflow::NodeId;

/// A denormalized trigger record optimized for lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRecord {
    /// Trigger ID.
    pub id: TriggerId,
    /// Workflow ID.
    pub workflow_id: WorkflowId,
    /// Node ID in the workflow.
    pub node_id: NodeId,
    /// Trigger type.
    pub trigger_type: TriggerType,
    /// Whether enabled.
    pub enabled: bool,
    /// Lookup key (type-specific).
    pub lookup_key: TriggerLookupKey,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// When updated.
    pub updated_at: DateTime<Utc>,
}

/// Type-specific lookup key for efficient trigger matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerLookupKey {
    /// Schedule trigger lookup key.
    Schedule {
        /// Cron expression.
        cron: String,
        /// Timezone.
        timezone: Option<String>,
    },
    /// Webhook trigger lookup key.
    Webhook {
        /// Webhook path.
        path: String,
    },
    /// Integration event trigger lookup key.
    IntegrationEvent {
        /// Integration account ID.
        integration_id: IntegrationAccountId,
        /// Event type.
        event_type: String,
    },
    /// Manual trigger (no lookup needed).
    Manual,
}

impl From<&TriggerConfig> for TriggerLookupKey {
    fn from(config: &TriggerConfig) -> Self {
        match config {
            TriggerConfig::Schedule { cron, timezone, .. } => Self::Schedule {
                cron: cron.clone(),
                timezone: timezone.clone(),
            },
            TriggerConfig::Webhook { path, .. } => Self::Webhook { path: path.clone() },
            TriggerConfig::IntegrationEvent {
                integration_id,
                event_type,
                ..
            } => Self::IntegrationEvent {
                integration_id: *integration_id,
                event_type: event_type.clone(),
            },
            TriggerConfig::Manual => Self::Manual,
        }
    }
}

impl TriggerRecord {
    /// Creates a trigger record from a trigger.
    #[must_use]
    pub fn from_trigger(trigger: &Trigger) -> Self {
        Self {
            id: trigger.id,
            workflow_id: trigger.workflow_id,
            node_id: trigger.node_id,
            trigger_type: trigger.trigger_type(),
            enabled: trigger.enabled,
            lookup_key: TriggerLookupKey::from(&trigger.config),
            created_at: trigger.created_at,
            updated_at: trigger.updated_at,
        }
    }
}

/// Trait for trigger storage and lookup.
#[async_trait]
pub trait TriggerManager: Send + Sync {
    /// Registers a trigger.
    async fn register(&self, trigger: Trigger) -> Result<TriggerId, TriggerError>;

    /// Gets a trigger by ID.
    async fn get(&self, id: TriggerId) -> Result<Trigger, TriggerError>;

    /// Updates a trigger.
    async fn update(&self, trigger: Trigger) -> Result<(), TriggerError>;

    /// Deletes a trigger.
    async fn delete(&self, id: TriggerId) -> Result<(), TriggerError>;

    /// Deletes all triggers for a workflow.
    async fn delete_for_workflow(&self, workflow_id: WorkflowId) -> Result<u32, TriggerError>;

    /// Lists triggers for a workflow.
    async fn list_for_workflow(&self, workflow_id: WorkflowId) -> Result<Vec<Trigger>, TriggerError>;

    /// Finds triggers by webhook path.
    async fn find_by_webhook_path(&self, path: &str) -> Result<Vec<TriggerRecord>, TriggerError>;

    /// Finds triggers by integration event.
    async fn find_by_integration_event(
        &self,
        integration_id: IntegrationAccountId,
        event_type: &str,
    ) -> Result<Vec<TriggerRecord>, TriggerError>;

    /// Gets all schedule triggers that need evaluation.
    async fn get_schedule_triggers(&self) -> Result<Vec<TriggerRecord>, TriggerError>;

    /// Reconciles triggers for a workflow (syncs from graph).
    ///
    /// This is called when a workflow is saved to ensure the trigger table
    /// matches the trigger nodes in the workflow graph.
    async fn reconcile(
        &self,
        workflow_id: WorkflowId,
        triggers: Vec<Trigger>,
    ) -> Result<ReconcileResult, TriggerError>;
}

/// Result of trigger reconciliation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReconcileResult {
    /// Number of triggers added.
    pub added: u32,
    /// Number of triggers updated.
    pub updated: u32,
    /// Number of triggers deleted.
    pub deleted: u32,
}

impl ReconcileResult {
    /// Returns whether any changes were made.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.added > 0 || self.updated > 0 || self.deleted > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use silver_telegram_workflow::trigger::MissedExecutionBehavior;

    #[test]
    fn trigger_record_from_trigger() {
        let workflow_id = WorkflowId::new();
        let node_id = NodeId::new();
        let config = TriggerConfig::Schedule {
            cron: "0 7 * * *".to_string(),
            timezone: Some("America/New_York".to_string()),
            next_run: None,
            missed_execution: MissedExecutionBehavior::Skip,
        };

        let trigger = Trigger::new(workflow_id, node_id, config);
        let record = TriggerRecord::from_trigger(&trigger);

        assert_eq!(record.id, trigger.id);
        assert_eq!(record.workflow_id, workflow_id);
        assert_eq!(record.trigger_type, TriggerType::Schedule);
    }

    #[test]
    fn lookup_key_from_webhook_config() {
        let config = TriggerConfig::Webhook {
            path: "/hooks/my-workflow".to_string(),
            secret: None,
        };

        let key = TriggerLookupKey::from(&config);
        match key {
            TriggerLookupKey::Webhook { path } => {
                assert_eq!(path, "/hooks/my-workflow");
            }
            _ => panic!("wrong key type"),
        }
    }

    #[test]
    fn reconcile_result_has_changes() {
        let empty = ReconcileResult::default();
        assert!(!empty.has_changes());

        let with_adds = ReconcileResult {
            added: 1,
            ..Default::default()
        };
        assert!(with_adds.has_changes());
    }
}
