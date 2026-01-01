//! Database repositories for workflow runs and execution history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::{NodeExecutionId, TriggerId, WorkflowId, WorkflowRunId};
use sqlx::{FromRow, PgPool};
use std::str::FromStr;

/// Execution state for a workflow run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    /// Waiting for an orchestrator.
    Queued,
    /// Actively executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with error.
    Failed,
    /// Cancelled by user or system.
    Cancelled,
}

impl RunState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn from_str_value(s: &str) -> Self {
        match s {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Queued,
        }
    }

    /// Returns true if this is a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// A workflow run record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunRecord {
    /// Run ID.
    pub id: WorkflowRunId,
    /// Workflow being executed.
    pub workflow_id: WorkflowId,
    /// Trigger that initiated the run.
    pub trigger_id: Option<TriggerId>,
    /// Current state.
    pub state: RunState,
    /// When queued.
    pub queued_at: DateTime<Utc>,
    /// When started executing.
    pub started_at: Option<DateTime<Utc>>,
    /// When finished.
    pub finished_at: Option<DateTime<Utc>>,
    /// Input data.
    pub input_data: Option<serde_json::Value>,
    /// Output data.
    pub output_data: Option<serde_json::Value>,
    /// Error message.
    pub error_message: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
}

impl WorkflowRunRecord {
    /// Creates a new run record in queued state.
    #[must_use]
    pub fn new(
        workflow_id: WorkflowId,
        trigger_id: Option<TriggerId>,
        input_data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: WorkflowRunId::new(),
            workflow_id,
            trigger_id,
            state: RunState::Queued,
            queued_at: Utc::now(),
            started_at: None,
            finished_at: None,
            input_data,
            output_data: None,
            error_message: None,
            duration_ms: None,
        }
    }

    /// Starts the run.
    pub fn start(&mut self) {
        self.state = RunState::Running;
        self.started_at = Some(Utc::now());
    }

    /// Completes the run.
    pub fn complete(&mut self, output: Option<serde_json::Value>) {
        self.state = RunState::Completed;
        self.finished_at = Some(Utc::now());
        self.output_data = output;
        if let Some(start) = self.started_at {
            self.duration_ms = Some((Utc::now() - start).num_milliseconds());
        }
    }

    /// Fails the run.
    pub fn fail(&mut self, error: String) {
        self.state = RunState::Failed;
        self.finished_at = Some(Utc::now());
        self.error_message = Some(error);
        if let Some(start) = self.started_at {
            self.duration_ms = Some((Utc::now() - start).num_milliseconds());
        }
    }

    /// Cancels the run.
    pub fn cancel(&mut self) {
        self.state = RunState::Cancelled;
        self.finished_at = Some(Utc::now());
        if let Some(start) = self.started_at {
            self.duration_ms = Some((Utc::now() - start).num_milliseconds());
        }
    }
}

/// Row type for run queries.
#[derive(FromRow)]
struct WorkflowRunRow {
    id: String,
    workflow_id: String,
    trigger_id: Option<String>,
    state: String,
    queued_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    input_data: Option<serde_json::Value>,
    output_data: Option<serde_json::Value>,
    error_message: Option<String>,
    duration_ms: Option<i64>,
}

impl WorkflowRunRow {
    fn try_into_record(self) -> Result<WorkflowRunRecord, sqlx::Error> {
        let id = WorkflowRunId::from_str(&self.id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid run id '{}': {}", self.id, e),
            )))
        })?;
        let workflow_id = WorkflowId::from_str(&self.workflow_id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid workflow id '{}': {}", self.workflow_id, e),
            )))
        })?;
        let trigger_id = self
            .trigger_id
            .map(|tid| {
                TriggerId::from_str(&tid).map_err(|e| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid trigger id '{}': {}", tid, e),
                    )))
                })
            })
            .transpose()?;

        Ok(WorkflowRunRecord {
            id,
            workflow_id,
            trigger_id,
            state: RunState::from_str_value(&self.state),
            queued_at: self.queued_at,
            started_at: self.started_at,
            finished_at: self.finished_at,
            input_data: self.input_data,
            output_data: self.output_data,
            error_message: self.error_message,
            duration_ms: self.duration_ms,
        })
    }
}

/// Repository for workflow run operations.
pub struct WorkflowRunRepository {
    pool: PgPool,
}

impl WorkflowRunRepository {
    /// Creates a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Lists recent runs for a workflow.
    pub async fn list_by_workflow(
        &self,
        workflow_id: WorkflowId,
        limit: i64,
    ) -> Result<Vec<WorkflowRunRecord>, sqlx::Error> {
        let rows: Vec<WorkflowRunRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_id, trigger_id, state, queued_at, started_at, finished_at,
                   input_data, output_data, error_message, duration_ms
            FROM workflow_runs
            WHERE workflow_id = $1
            ORDER BY queued_at DESC
            LIMIT $2
            "#,
        )
        .bind(workflow_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into_record()).collect()
    }

    /// Finds a run by ID.
    pub async fn find_by_id(
        &self,
        id: WorkflowRunId,
    ) -> Result<Option<WorkflowRunRecord>, sqlx::Error> {
        let row: Option<WorkflowRunRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_id, trigger_id, state, queued_at, started_at, finished_at,
                   input_data, output_data, error_message, duration_ms
            FROM workflow_runs
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

    /// Creates a new run.
    pub async fn create(&self, run: &WorkflowRunRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO workflow_runs
                (id, workflow_id, trigger_id, state, queued_at, started_at, finished_at,
                 input_data, output_data, error_message, duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(run.id.to_string())
        .bind(run.workflow_id.to_string())
        .bind(run.trigger_id.map(|t| t.to_string()))
        .bind(run.state.as_str())
        .bind(run.queued_at)
        .bind(run.started_at)
        .bind(run.finished_at)
        .bind(&run.input_data)
        .bind(&run.output_data)
        .bind(&run.error_message)
        .bind(run.duration_ms)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Updates a run.
    pub async fn update(&self, run: &WorkflowRunRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE workflow_runs
            SET state = $2, started_at = $3, finished_at = $4, output_data = $5,
                error_message = $6, duration_ms = $7
            WHERE id = $1
            "#,
        )
        .bind(run.id.to_string())
        .bind(run.state.as_str())
        .bind(run.started_at)
        .bind(run.finished_at)
        .bind(&run.output_data)
        .bind(&run.error_message)
        .bind(run.duration_ms)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Lists queued or running runs (for the orchestrator).
    pub async fn list_active(&self) -> Result<Vec<WorkflowRunRecord>, sqlx::Error> {
        let rows: Vec<WorkflowRunRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_id, trigger_id, state, queued_at, started_at, finished_at,
                   input_data, output_data, error_message, duration_ms
            FROM workflow_runs
            WHERE state IN ('queued', 'running')
            ORDER BY queued_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into_record()).collect()
    }

    /// Cancels all running runs for a workflow.
    pub async fn cancel_for_workflow(&self, workflow_id: WorkflowId) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE workflow_runs
            SET state = 'cancelled', finished_at = NOW()
            WHERE workflow_id = $1 AND state IN ('queued', 'running')
            "#,
        )
        .bind(workflow_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

/// Execution state for a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    /// Waiting for predecessors.
    Pending,
    /// Ready to execute.
    Ready,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with error.
    Failed,
    /// Skipped.
    Skipped,
}

impl NodeState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    fn from_str_value(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "ready" => Self::Ready,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            _ => Self::Pending,
        }
    }
}

/// A node execution record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionRecord {
    /// Execution ID.
    pub id: NodeExecutionId,
    /// Run this execution belongs to.
    pub run_id: WorkflowRunId,
    /// Node ID within the workflow.
    pub node_id: String,
    /// Current state.
    pub state: NodeState,
    /// When started.
    pub started_at: Option<DateTime<Utc>>,
    /// When finished.
    pub finished_at: Option<DateTime<Utc>>,
    /// Input data.
    pub input_data: Option<serde_json::Value>,
    /// Output key in NATS Object Store.
    pub output_key: Option<String>,
    /// Error message.
    pub error_message: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
}

impl NodeExecutionRecord {
    /// Creates a new execution record.
    #[must_use]
    pub fn new(run_id: WorkflowRunId, node_id: String) -> Self {
        Self {
            id: NodeExecutionId::new(),
            run_id,
            node_id,
            state: NodeState::Pending,
            started_at: None,
            finished_at: None,
            input_data: None,
            output_key: None,
            error_message: None,
            duration_ms: None,
        }
    }
}

/// Row type for node execution queries.
#[derive(FromRow)]
struct NodeExecutionRow {
    id: String,
    run_id: String,
    node_id: String,
    state: String,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    input_data: Option<serde_json::Value>,
    output_key: Option<String>,
    error_message: Option<String>,
    duration_ms: Option<i64>,
}

impl NodeExecutionRow {
    fn try_into_record(self) -> Result<NodeExecutionRecord, sqlx::Error> {
        let id = NodeExecutionId::from_str(&self.id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid execution id '{}': {}", self.id, e),
            )))
        })?;
        let run_id = WorkflowRunId::from_str(&self.run_id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid run id '{}': {}", self.run_id, e),
            )))
        })?;

        Ok(NodeExecutionRecord {
            id,
            run_id,
            node_id: self.node_id,
            state: NodeState::from_str_value(&self.state),
            started_at: self.started_at,
            finished_at: self.finished_at,
            input_data: self.input_data,
            output_key: self.output_key,
            error_message: self.error_message,
            duration_ms: self.duration_ms,
        })
    }
}

/// Repository for node execution operations.
pub struct NodeExecutionRepository {
    pool: PgPool,
}

impl NodeExecutionRepository {
    /// Creates a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Lists executions for a run.
    pub async fn list_by_run(
        &self,
        run_id: WorkflowRunId,
    ) -> Result<Vec<NodeExecutionRecord>, sqlx::Error> {
        let rows: Vec<NodeExecutionRow> = sqlx::query_as(
            r#"
            SELECT id, run_id, node_id, state, started_at, finished_at,
                   input_data, output_key, error_message, duration_ms
            FROM node_executions
            WHERE run_id = $1
            ORDER BY started_at ASC NULLS FIRST
            "#,
        )
        .bind(run_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into_record()).collect()
    }

    /// Creates a node execution.
    pub async fn create(&self, execution: &NodeExecutionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO node_executions
                (id, run_id, node_id, state, started_at, finished_at,
                 input_data, output_key, error_message, duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(execution.id.to_string())
        .bind(execution.run_id.to_string())
        .bind(&execution.node_id)
        .bind(execution.state.as_str())
        .bind(execution.started_at)
        .bind(execution.finished_at)
        .bind(&execution.input_data)
        .bind(&execution.output_key)
        .bind(&execution.error_message)
        .bind(execution.duration_ms)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Updates a node execution.
    pub async fn update(&self, execution: &NodeExecutionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE node_executions
            SET state = $2, started_at = $3, finished_at = $4, input_data = $5,
                output_key = $6, error_message = $7, duration_ms = $8
            WHERE id = $1
            "#,
        )
        .bind(execution.id.to_string())
        .bind(execution.state.as_str())
        .bind(execution.started_at)
        .bind(execution.finished_at)
        .bind(&execution.input_data)
        .bind(&execution.output_key)
        .bind(&execution.error_message)
        .bind(execution.duration_ms)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// A decision trace record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTraceRecord {
    /// Trace ID.
    pub id: String,
    /// Node execution this trace belongs to.
    pub node_execution_id: NodeExecutionId,
    /// Sequence number.
    pub sequence: i32,
    /// Type of trace.
    pub trace_type: String,
    /// Trace data.
    pub trace_data: serde_json::Value,
    /// When recorded.
    pub created_at: DateTime<Utc>,
}

/// Row type for trace queries.
#[derive(FromRow)]
struct DecisionTraceRow {
    id: String,
    node_execution_id: String,
    sequence: i32,
    trace_type: String,
    trace_data: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl DecisionTraceRow {
    fn try_into_record(self) -> Result<DecisionTraceRecord, sqlx::Error> {
        let node_execution_id =
            NodeExecutionId::from_str(&self.node_execution_id).map_err(|e| {
                sqlx::Error::Decode(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid execution id '{}': {}", self.node_execution_id, e),
                )))
            })?;

        Ok(DecisionTraceRecord {
            id: self.id,
            node_execution_id,
            sequence: self.sequence,
            trace_type: self.trace_type,
            trace_data: self.trace_data,
            created_at: self.created_at,
        })
    }
}

/// Repository for decision trace operations.
pub struct DecisionTraceRepository {
    pool: PgPool,
}

impl DecisionTraceRepository {
    /// Creates a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Lists traces for a node execution.
    pub async fn list_by_execution(
        &self,
        execution_id: NodeExecutionId,
    ) -> Result<Vec<DecisionTraceRecord>, sqlx::Error> {
        let rows: Vec<DecisionTraceRow> = sqlx::query_as(
            r#"
            SELECT id, node_execution_id, sequence, trace_type, trace_data, created_at
            FROM decision_traces
            WHERE node_execution_id = $1
            ORDER BY sequence ASC
            "#,
        )
        .bind(execution_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into_record()).collect()
    }

    /// Creates a trace.
    pub async fn create(&self, trace: &DecisionTraceRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO decision_traces
                (id, node_execution_id, sequence, trace_type, trace_data, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(&trace.id)
        .bind(trace.node_execution_id.to_string())
        .bind(trace.sequence)
        .bind(&trace.trace_type)
        .bind(&trace.trace_data)
        .bind(trace.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
