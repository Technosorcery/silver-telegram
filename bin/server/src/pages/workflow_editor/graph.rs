//! Workflow graph types and operations.
//!
//! Contains the node/edge graph structure and server functions for updating graphs.

use leptos::prelude::*;

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

/// Server function to update workflow graph.
#[server]
pub async fn update_workflow_graph(
    workflow_id: String,
    graph_json: String,
) -> Result<(), ServerFnError> {
    use crate::db::{TriggerRecord, TriggerRepository, WorkflowRepository};
    use crate::error::WorkflowError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::{TriggerId, WorkflowId};
    use std::str::FromStr;

    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for update_workflow_graph");
        e.into_server_error()
    })?;

    let wf_id = WorkflowId::from_str(&workflow_id).map_err(|e| {
        tracing::debug!(
            workflow_id = %workflow_id,
            error = %e,
            "Invalid workflow ID format"
        );
        WorkflowError::InvalidId {
            id: workflow_id.clone(),
            reason: e.to_string(),
        }
        .into_server_error()
    })?;

    // Check edit permission via SpiceDB
    let authz_client = get_authz_client();
    let resource = Resource::workflow(wf_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::Edit, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                workflow_id = %wf_id,
                user_id = %auth.user_id,
                error = %e,
                "Access denied to edit workflow"
            );
            WorkflowError::AccessDenied {
                id: wf_id.to_string(),
            }
            .into_server_error()
        })?;

    let graph: serde_json::Value = serde_json::from_str(&graph_json).map_err(|e| {
        tracing::debug!(
            workflow_id = %wf_id,
            error = %e,
            "Invalid graph JSON"
        );
        WorkflowError::InvalidGraph {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    let db_pool = get_db_pool();
    let workflow_repo = WorkflowRepository::new(db_pool.clone());
    let mut workflow = workflow_repo
        .find_by_id(wf_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                workflow_id = %wf_id,
                "Database error loading workflow"
            );
            WorkflowError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?
        .ok_or_else(|| {
            tracing::debug!(workflow_id = %wf_id, "Workflow not found");
            WorkflowError::NotFound {
                id: wf_id.to_string(),
            }
            .into_server_error()
        })?;

    workflow.set_graph(graph.clone());

    workflow_repo.update(&workflow).await.map_err(|e| {
        tracing::error!(
            error = %e,
            workflow_id = %wf_id,
            "Failed to update workflow"
        );
        WorkflowError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    // Update triggers from graph nodes
    let trigger_repo = TriggerRepository::new(db_pool);
    let nodes = graph.get("nodes").and_then(|n| n.as_array());
    let mut trigger_node_ids = Vec::new();

    if let Some(nodes) = nodes {
        for node in nodes {
            let node_type = node.get("node_type").and_then(|t| t.as_str()).unwrap_or("");
            if node_type == "trigger"
                && let Some(node_id) = node.get("id").and_then(|i| i.as_str())
            {
                trigger_node_ids.push(node_id.to_string());
                let config = node.get("config").cloned().unwrap_or_default();
                let now = chrono::Utc::now();
                let trigger = TriggerRecord {
                    id: TriggerId::new(),
                    workflow_id: wf_id,
                    node_id: node_id.to_string(),
                    trigger_type: "schedule".to_string(),
                    config_data: config,
                    active: workflow.enabled,
                    created_at: now,
                    updated_at: now,
                };
                trigger_repo.upsert(&trigger).await.map_err(|e| {
                    tracing::error!(
                        error = %e,
                        workflow_id = %wf_id,
                        node_id = %node_id,
                        "Failed to save trigger"
                    );
                    WorkflowError::DatabaseError {
                        details: e.to_string(),
                    }
                    .into_server_error()
                })?;
            }
        }
    }

    // Remove triggers for deleted nodes
    trigger_repo
        .delete_except(wf_id, &trigger_node_ids)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                workflow_id = %wf_id,
                "Failed to cleanup triggers"
            );
            WorkflowError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    tracing::info!(
        workflow_id = %wf_id,
        user_id = %auth.user_id,
        trigger_count = trigger_node_ids.len(),
        "Workflow graph updated"
    );

    Ok(())
}
