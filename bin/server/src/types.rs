//! Shared types used across server functions and UI components.

/// User info for display in the UI.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserInfo {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub timezone: Option<String>,
    pub is_admin: bool,
}

/// Integration information for display.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct IntegrationInfo {
    pub id: String,
    pub name: String,
    pub integration_type: String,
    pub status: String,
    pub error_message: Option<String>,
}

/// Integration configuration for editing.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IntegrationConfigData {
    // IMAP fields
    pub server: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub use_tls: Option<bool>,
    // Calendar feed fields
    pub url: Option<String>,
}

/// Workflow summary for admin view.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub owner_name: String,
    pub owner_id: String,
    pub enabled: bool,
    pub last_run: Option<String>,
}

/// User workflow info for the workflows list.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserWorkflowInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub last_run_state: Option<String>,
    pub last_run_duration_ms: Option<i64>,
}

/// Workflow detail for editing.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkflowDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub graph_data: String,
    pub memory_content: Option<String>,
}

/// Workflow node for the graph editor.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub config: String,
    pub x: f64,
    pub y: f64,
}

/// Workflow edge connecting nodes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_port: String,
    pub target_port: String,
}

/// Workflow graph data structure.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WorkflowGraph {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

/// Workflow run summary for history list.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkflowRunSummary {
    pub id: String,
    pub state: String,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
}

/// Node execution summary for run details.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NodeExecutionSummary {
    pub id: String,
    pub node_id: String,
    pub state: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
    pub input_data: Option<serde_json::Value>,
    pub output_key: Option<String>,
}

/// Decision trace summary for AI node debugging.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DecisionTraceSummary {
    pub sequence: i32,
    pub trace_type: String,
    pub trace_data: serde_json::Value,
}

/// Detailed run information with node executions.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RunDetailView {
    pub id: String,
    pub state: String,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
    pub input_data: Option<serde_json::Value>,
    pub output_data: Option<serde_json::Value>,
    pub node_executions: Vec<NodeExecutionSummary>,
}
