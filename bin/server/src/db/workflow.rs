//! Database repositories for workflows, triggers, and memory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::{TriggerId, WorkflowId};
use sqlx::{FromRow, PgPool};
use std::str::FromStr;

/// A workflow record from the database.
///
/// Note: Ownership is stored in SpiceDB via relationships, not in this table.
/// Use AuthzClient to check permissions (view, edit, delete, execute).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRecord {
    /// Workflow ID.
    pub id: WorkflowId,
    /// Human-readable name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Whether the workflow is enabled.
    pub enabled: bool,
    /// Tags for organization.
    pub tags: Vec<String>,
    /// The workflow graph (nodes and edges).
    pub graph_data: serde_json::Value,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// When last updated.
    pub updated_at: DateTime<Utc>,
}

impl WorkflowRecord {
    /// Creates a new workflow record.
    ///
    /// Note: After creating the record, you must also create an ownership
    /// relationship in SpiceDB using AuthzClient::write_relationship.
    #[must_use]
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: WorkflowId::new(),
            name,
            description: None,
            enabled: true,
            tags: Vec::new(),
            graph_data: serde_json::json!({"nodes": [], "edges": []}),
            created_at: now,
            updated_at: now,
        }
    }

    /// Updates the graph data.
    pub fn set_graph(&mut self, graph: serde_json::Value) {
        self.graph_data = graph;
        self.updated_at = Utc::now();
    }

    /// Enables the workflow.
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
    }

    /// Disables the workflow.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
    }
}

/// Row type for workflow queries.
#[derive(FromRow)]
struct WorkflowRow {
    id: String,
    name: String,
    description: Option<String>,
    enabled: bool,
    tags: serde_json::Value,
    graph_data: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl WorkflowRow {
    fn try_into_record(self) -> Result<WorkflowRecord, sqlx::Error> {
        let id = WorkflowId::from_str(&self.id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid workflow id '{}': {}", self.id, e),
            )))
        })?;

        let tags: Vec<String> = serde_json::from_value(self.tags).unwrap_or_default();

        Ok(WorkflowRecord {
            id,
            name: self.name,
            description: self.description,
            enabled: self.enabled,
            tags,
            graph_data: self.graph_data,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Summary information for workflow listings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummaryRow {
    /// Workflow ID.
    pub id: WorkflowId,
    /// Workflow name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Whether enabled.
    pub enabled: bool,
    /// When last updated.
    pub updated_at: DateTime<Utc>,
    /// Last run time (if any).
    pub last_run_at: Option<DateTime<Utc>>,
    /// Last run duration in milliseconds.
    pub last_run_duration_ms: Option<i64>,
    /// Last run state.
    pub last_run_state: Option<String>,
}

/// Internal row type for workflow summary queries.
#[derive(FromRow)]
struct WorkflowSummaryDbRow {
    id: String,
    name: String,
    description: Option<String>,
    enabled: bool,
    updated_at: DateTime<Utc>,
    last_run_at: Option<DateTime<Utc>>,
    last_run_duration_ms: Option<i64>,
    last_run_state: Option<String>,
}

impl WorkflowSummaryDbRow {
    fn try_into_summary(self) -> Result<WorkflowSummaryRow, sqlx::Error> {
        let id = WorkflowId::from_str(&self.id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid workflow id '{}': {}", self.id, e),
            )))
        })?;
        Ok(WorkflowSummaryRow {
            id,
            name: self.name,
            description: self.description,
            enabled: self.enabled,
            updated_at: self.updated_at,
            last_run_at: self.last_run_at,
            last_run_duration_ms: self.last_run_duration_ms,
            last_run_state: self.last_run_state,
        })
    }
}

/// Repository for workflow operations.
pub struct WorkflowRepository {
    pool: PgPool,
}

impl WorkflowRepository {
    /// Creates a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Lists workflows by IDs with summary info.
    ///
    /// Use this after querying SpiceDB for workflow IDs the user has access to.
    pub async fn list_by_ids(
        &self,
        ids: &[WorkflowId],
    ) -> Result<Vec<WorkflowSummaryRow>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        let rows: Vec<WorkflowSummaryDbRow> = sqlx::query_as(
            r#"
            SELECT
                w.id,
                w.name,
                w.description,
                w.enabled,
                w.updated_at,
                r.finished_at as last_run_at,
                r.duration_ms as last_run_duration_ms,
                r.state as last_run_state
            FROM workflows w
            LEFT JOIN LATERAL (
                SELECT finished_at, duration_ms, state
                FROM workflow_runs
                WHERE workflow_id = w.id
                ORDER BY queued_at DESC
                LIMIT 1
            ) r ON true
            WHERE w.id = ANY($1)
            ORDER BY w.updated_at DESC
            "#,
        )
        .bind(&id_strings)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into_summary()).collect()
    }

    /// Lists all workflows with summary info (admin view, no permission filtering).
    pub async fn list_all_summaries(&self) -> Result<Vec<WorkflowSummaryRow>, sqlx::Error> {
        let rows: Vec<WorkflowSummaryDbRow> = sqlx::query_as(
            r#"
            SELECT
                w.id,
                w.name,
                w.description,
                w.enabled,
                w.updated_at,
                r.finished_at as last_run_at,
                r.duration_ms as last_run_duration_ms,
                r.state as last_run_state
            FROM workflows w
            LEFT JOIN LATERAL (
                SELECT finished_at, duration_ms, state
                FROM workflow_runs
                WHERE workflow_id = w.id
                ORDER BY queued_at DESC
                LIMIT 1
            ) r ON true
            ORDER BY w.updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into_summary()).collect()
    }

    /// Finds a workflow by ID.
    pub async fn find_by_id(&self, id: WorkflowId) -> Result<Option<WorkflowRecord>, sqlx::Error> {
        let row: Option<WorkflowRow> = sqlx::query_as(
            r#"
            SELECT id, name, description, enabled, tags, graph_data,
                   created_at, updated_at
            FROM workflows
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.try_into_record()?)),
            None => Ok(None),
        }
    }

    /// Creates a new workflow.
    ///
    /// Note: After creating the workflow, you must also create an ownership
    /// relationship in SpiceDB using AuthzClient::write_relationship.
    pub async fn create(&self, workflow: &WorkflowRecord) -> Result<(), sqlx::Error> {
        let tags_json = serde_json::to_value(&workflow.tags).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO workflows
                (id, name, description, enabled, tags, graph_data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(workflow.id.to_string())
        .bind(&workflow.name)
        .bind(&workflow.description)
        .bind(workflow.enabled)
        .bind(&tags_json)
        .bind(&workflow.graph_data)
        .bind(workflow.created_at)
        .bind(workflow.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Updates an existing workflow.
    pub async fn update(&self, workflow: &WorkflowRecord) -> Result<(), sqlx::Error> {
        let tags_json = serde_json::to_value(&workflow.tags).unwrap_or_default();

        sqlx::query(
            r#"
            UPDATE workflows
            SET name = $2, description = $3, enabled = $4, tags = $5, graph_data = $6,
                updated_at = $7
            WHERE id = $1
            "#,
        )
        .bind(workflow.id.to_string())
        .bind(&workflow.name)
        .bind(&workflow.description)
        .bind(workflow.enabled)
        .bind(&tags_json)
        .bind(&workflow.graph_data)
        .bind(workflow.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes a workflow.
    pub async fn delete(&self, id: WorkflowId) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM workflows
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Toggles the enabled state of a workflow.
    pub async fn toggle_enabled(&self, id: WorkflowId) -> Result<bool, sqlx::Error> {
        let row: Option<(bool,)> = sqlx::query_as(
            r#"
            UPDATE workflows
            SET enabled = NOT enabled, updated_at = NOW()
            WHERE id = $1
            RETURNING enabled
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(e,)| e).unwrap_or(false))
    }
}

/// A trigger record from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRecord {
    /// Trigger ID.
    pub id: TriggerId,
    /// Workflow this trigger belongs to.
    pub workflow_id: WorkflowId,
    /// Node ID within the workflow.
    pub node_id: String,
    /// Type of trigger.
    pub trigger_type: String,
    /// Trigger configuration.
    pub config_data: serde_json::Value,
    /// Whether the trigger is active.
    pub active: bool,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// When last updated.
    pub updated_at: DateTime<Utc>,
}

/// Row type for trigger queries.
#[derive(FromRow)]
struct TriggerRow {
    id: String,
    workflow_id: String,
    node_id: String,
    trigger_type: String,
    config_data: serde_json::Value,
    active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TriggerRow {
    fn try_into_record(self) -> Result<TriggerRecord, sqlx::Error> {
        let id = TriggerId::from_str(&self.id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid trigger id '{}': {}", self.id, e),
            )))
        })?;
        let workflow_id = WorkflowId::from_str(&self.workflow_id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid workflow id '{}': {}", self.workflow_id, e),
            )))
        })?;

        Ok(TriggerRecord {
            id,
            workflow_id,
            node_id: self.node_id,
            trigger_type: self.trigger_type,
            config_data: self.config_data,
            active: self.active,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Repository for trigger operations.
pub struct TriggerRepository {
    pool: PgPool,
}

impl TriggerRepository {
    /// Creates a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Lists all active schedule triggers.
    pub async fn list_active_schedules(&self) -> Result<Vec<TriggerRecord>, sqlx::Error> {
        let rows: Vec<TriggerRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_id, node_id, trigger_type, config_data, active, created_at, updated_at
            FROM triggers
            WHERE trigger_type = 'schedule' AND active = true
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into_record()).collect()
    }

    /// Lists triggers for a workflow.
    pub async fn list_by_workflow(
        &self,
        workflow_id: WorkflowId,
    ) -> Result<Vec<TriggerRecord>, sqlx::Error> {
        let rows: Vec<TriggerRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_id, node_id, trigger_type, config_data, active, created_at, updated_at
            FROM triggers
            WHERE workflow_id = $1
            "#,
        )
        .bind(workflow_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into_record()).collect()
    }

    /// Creates or updates a trigger for a workflow node.
    pub async fn upsert(&self, trigger: &TriggerRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO triggers (id, workflow_id, node_id, trigger_type, config_data, active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (workflow_id, node_id)
            DO UPDATE SET trigger_type = $4, config_data = $5, active = $6, updated_at = $8
            "#,
        )
        .bind(trigger.id.to_string())
        .bind(trigger.workflow_id.to_string())
        .bind(&trigger.node_id)
        .bind(&trigger.trigger_type)
        .bind(&trigger.config_data)
        .bind(trigger.active)
        .bind(trigger.created_at)
        .bind(trigger.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes triggers for a workflow that are not in the given node list.
    pub async fn delete_except(
        &self,
        workflow_id: WorkflowId,
        keep_nodes: &[String],
    ) -> Result<(), sqlx::Error> {
        if keep_nodes.is_empty() {
            sqlx::query(
                r#"
                DELETE FROM triggers
                WHERE workflow_id = $1
                "#,
            )
            .bind(workflow_id.to_string())
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                DELETE FROM triggers
                WHERE workflow_id = $1 AND node_id != ALL($2)
                "#,
            )
            .bind(workflow_id.to_string())
            .bind(keep_nodes)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Updates the active state for all triggers of a workflow.
    pub async fn set_active(
        &self,
        workflow_id: WorkflowId,
        active: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE triggers
            SET active = $2, updated_at = NOW()
            WHERE workflow_id = $1
            "#,
        )
        .bind(workflow_id.to_string())
        .bind(active)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Workflow memory record.
#[derive(Debug, Clone)]
pub struct WorkflowMemoryRecord {
    /// Memory ID.
    pub id: String,
    /// Workflow this memory belongs to.
    pub workflow_id: WorkflowId,
    /// Memory content.
    pub content: Vec<u8>,
    /// Version for optimistic concurrency.
    pub version: i32,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// When last updated.
    pub updated_at: DateTime<Utc>,
}

/// Row type for memory queries.
#[derive(FromRow)]
struct WorkflowMemoryRow {
    id: String,
    workflow_id: String,
    content: Vec<u8>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl WorkflowMemoryRow {
    fn try_into_record(self) -> Result<WorkflowMemoryRecord, sqlx::Error> {
        let workflow_id = WorkflowId::from_str(&self.workflow_id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid workflow id '{}': {}", self.workflow_id, e),
            )))
        })?;

        Ok(WorkflowMemoryRecord {
            id: self.id,
            workflow_id,
            content: self.content,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Repository for workflow memory operations.
pub struct WorkflowMemoryRepository {
    pool: PgPool,
}

impl WorkflowMemoryRepository {
    /// Creates a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Gets memory for a workflow.
    pub async fn find_by_workflow(
        &self,
        workflow_id: WorkflowId,
    ) -> Result<Option<WorkflowMemoryRecord>, sqlx::Error> {
        let row: Option<WorkflowMemoryRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_id, content, version, created_at, updated_at
            FROM workflow_memory
            WHERE workflow_id = $1
            "#,
        )
        .bind(workflow_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.try_into_record()?)),
            None => Ok(None),
        }
    }

    /// Creates or updates memory for a workflow with optimistic concurrency.
    pub async fn upsert(
        &self,
        workflow_id: WorkflowId,
        content: Vec<u8>,
        expected_version: Option<i32>,
    ) -> Result<i32, sqlx::Error> {
        let now = Utc::now();
        let id = ulid::Ulid::new().to_string();

        // If we have an expected version, this is an update
        if let Some(version) = expected_version {
            let result = sqlx::query(
                r#"
                UPDATE workflow_memory
                SET content = $2, version = version + 1, updated_at = $3
                WHERE workflow_id = $1 AND version = $4
                RETURNING version
                "#,
            )
            .bind(workflow_id.to_string())
            .bind(&content)
            .bind(now)
            .bind(version)
            .execute(&self.pool)
            .await?;

            if result.rows_affected() == 0 {
                return Err(sqlx::Error::RowNotFound);
            }

            Ok(version + 1)
        } else {
            // Initial insert
            sqlx::query(
                r#"
                INSERT INTO workflow_memory (id, workflow_id, content, version, created_at, updated_at)
                VALUES ($1, $2, $3, 1, $4, $4)
                ON CONFLICT (workflow_id)
                DO UPDATE SET content = $3, version = workflow_memory.version + 1, updated_at = $4
                RETURNING version
                "#,
            )
            .bind(&id)
            .bind(workflow_id.to_string())
            .bind(&content)
            .bind(now)
            .execute(&self.pool)
            .await?;

            Ok(1)
        }
    }
}
