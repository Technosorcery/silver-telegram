//! Workflow run history types, server functions, and UI components.
//!
//! Contains everything related to viewing workflow execution history.

use leptos::prelude::*;

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

/// Server function to list workflow runs.
#[server]
pub async fn list_workflow_runs(
    workflow_id: String,
) -> Result<Vec<WorkflowRunSummary>, ServerFnError> {
    use crate::db::WorkflowRunRepository;
    use crate::error::{WorkflowError, WorkflowRunError};
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::WorkflowId;
    use std::str::FromStr;

    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for list_workflow_runs");
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

    // Check view permission via SpiceDB
    let authz_client = get_authz_client();
    let resource = Resource::workflow(wf_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::View, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                workflow_id = %wf_id,
                user_id = %auth.user_id,
                error = %e,
                "Access denied to workflow"
            );
            WorkflowError::AccessDenied {
                id: wf_id.to_string(),
            }
            .into_server_error()
        })?;

    let db_pool = get_db_pool();
    let run_repo = WorkflowRunRepository::new(db_pool);
    let runs = run_repo.list_by_workflow(wf_id, 50).await.map_err(|e| {
        tracing::error!(
            error = %e,
            workflow_id = %wf_id,
            "Database error loading workflow runs"
        );
        WorkflowRunError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    Ok(runs
        .into_iter()
        .map(|r| WorkflowRunSummary {
            id: r.id.to_string(),
            state: format!("{:?}", r.state).to_lowercase(),
            queued_at: r.queued_at.to_rfc3339(),
            started_at: r.started_at.map(|dt| dt.to_rfc3339()),
            finished_at: r.finished_at.map(|dt| dt.to_rfc3339()),
            duration_ms: r.duration_ms,
            error_message: r.error_message,
        })
        .collect())
}

/// Server function to get run details with node executions.
#[server]
pub async fn get_run_detail(
    workflow_id: String,
    run_id: String,
) -> Result<RunDetailView, ServerFnError> {
    use crate::db::{NodeExecutionRepository, WorkflowRunRepository};
    use crate::error::{WorkflowError, WorkflowRunError};
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::{WorkflowId, WorkflowRunId};
    use std::str::FromStr;

    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for get_run_detail");
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

    let r_id = WorkflowRunId::from_str(&run_id).map_err(|e| {
        tracing::debug!(
            run_id = %run_id,
            error = %e,
            "Invalid run ID format"
        );
        WorkflowRunError::InvalidId {
            id: run_id.clone(),
            reason: e.to_string(),
        }
        .into_server_error()
    })?;

    // Check view permission via SpiceDB
    let authz_client = get_authz_client();
    let resource = Resource::workflow(wf_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::View, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                workflow_id = %wf_id,
                user_id = %auth.user_id,
                error = %e,
                "Access denied to workflow"
            );
            WorkflowError::AccessDenied {
                id: wf_id.to_string(),
            }
            .into_server_error()
        })?;

    // Get run
    let db_pool = get_db_pool();
    let run_repo = WorkflowRunRepository::new(db_pool.clone());
    let run = run_repo
        .find_by_id(r_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                run_id = %r_id,
                "Database error loading workflow run"
            );
            WorkflowRunError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?
        .ok_or_else(|| {
            tracing::debug!(run_id = %r_id, "Workflow run not found");
            WorkflowRunError::NotFound {
                id: r_id.to_string(),
            }
            .into_server_error()
        })?;

    // Get node executions
    let exec_repo = NodeExecutionRepository::new(db_pool);
    let executions = exec_repo.list_by_run(r_id).await.map_err(|e| {
        tracing::error!(
            error = %e,
            run_id = %r_id,
            "Database error loading node executions"
        );
        WorkflowRunError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    let node_executions = executions
        .into_iter()
        .map(|e| NodeExecutionSummary {
            id: e.id.to_string(),
            node_id: e.node_id,
            state: format!("{:?}", e.state).to_lowercase(),
            started_at: e.started_at.map(|dt| dt.to_rfc3339()),
            finished_at: e.finished_at.map(|dt| dt.to_rfc3339()),
            duration_ms: e.duration_ms,
            error_message: e.error_message,
            input_data: e.input_data,
            output_key: e.output_key,
        })
        .collect();

    Ok(RunDetailView {
        id: run.id.to_string(),
        state: format!("{:?}", run.state).to_lowercase(),
        queued_at: run.queued_at.to_rfc3339(),
        started_at: run.started_at.map(|dt| dt.to_rfc3339()),
        finished_at: run.finished_at.map(|dt| dt.to_rfc3339()),
        duration_ms: run.duration_ms,
        error_message: run.error_message,
        input_data: run.input_data,
        output_data: run.output_data,
        node_executions,
    })
}

/// History tab component displaying workflow runs and run details.
#[component]
pub fn HistoryTab(workflow_id: Signal<Option<String>>) -> impl IntoView {
    let (selected_run_id, set_selected_run_id) = signal(Option::<String>::None);

    // Runs resource
    let runs = Resource::new(
        move || workflow_id.get(),
        |id| async move {
            match id {
                Some(id) => list_workflow_runs(id).await.ok().unwrap_or_default(),
                None => vec![],
            }
        },
    );

    // Run detail resource
    let run_detail = Resource::new(
        move || (workflow_id.get(), selected_run_id.get()),
        |(wf_id, run_id)| async move {
            match (wf_id, run_id) {
                (Some(wf_id), Some(run_id)) => get_run_detail(wf_id, run_id).await.ok(),
                _ => None,
            }
        },
    );

    view! {
        <div class="history-content">
            <div class="history-layout">
                <div class="runs-list">
                    <h3>"Execution History"</h3>
                    <Suspense fallback=move || view! { <p>"Loading runs..."</p> }>
                        {move || {
                            let runs_list = runs.get().unwrap_or_default();
                            if runs_list.is_empty() {
                                view! {
                                    <p class="empty-state">"No runs yet. The workflow will run according to its schedule."</p>
                                }.into_any()
                            } else {
                                view! {
                                    <table class="runs-table">
                                        <thead>
                                            <tr>
                                                <th>"Status"</th>
                                                <th>"Started"</th>
                                                <th>"Duration"</th>
                                                <th>"Error"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {runs_list.into_iter().map(|run| {
                                                let run_id = run.id.clone();
                                                let run_id_for_click = run.id.clone();
                                                let status_class = format!("status-{}", run.state);
                                                let duration = run.duration_ms.map(|ms| format!("{}ms", ms)).unwrap_or_else(|| "-".to_string());
                                                let started = run.started_at.unwrap_or_else(|| run.queued_at.clone());
                                                view! {
                                                    <tr
                                                        class:selected=move || selected_run_id.get().as_ref() == Some(&run_id)
                                                        on:click=move |_| {
                                                            set_selected_run_id.set(Some(run_id_for_click.clone()));
                                                        }
                                                    >
                                                        <td class=status_class>{run.state}</td>
                                                        <td>{started}</td>
                                                        <td>{duration}</td>
                                                        <td class="error-cell">
                                                            {run.error_message.map(|e| view! { <span class="error">{e}</span> })}
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                }.into_any()
                            }
                        }}
                    </Suspense>
                </div>

                // Run detail panel
                {move || selected_run_id.get().map(|_| view! {
                    <div class="run-detail-panel">
                        <div class="run-detail-header">
                            <h3>"Run Details"</h3>
                            <button
                                class="close-btn"
                                on:click=move |_| set_selected_run_id.set(None)
                            >
                                "×"
                            </button>
                        </div>
                        <Suspense fallback=move || view! { <p>"Loading details..."</p> }>
                            {move || run_detail.get().map(|detail_opt| {
                                match detail_opt {
                                    Some(detail) => {
                                        view! { <RunDetailPanel detail=detail /> }.into_any()
                                    },
                                    None => view! {
                                        <p class="error">"Failed to load run details."</p>
                                    }.into_any(),
                                }
                            })}
                        </Suspense>
                    </div>
                })}
            </div>
        </div>
    }
}

/// Run detail panel component showing execution information.
#[component]
fn RunDetailPanel(detail: RunDetailView) -> impl IntoView {
    let run_state = detail.state.clone();
    let duration = detail
        .duration_ms
        .map(|ms| format!("{}ms", ms))
        .unwrap_or_else(|| "-".to_string());
    let status_class = format!("status-{}", detail.state);
    let run_error = detail.error_message.clone();
    let node_execs = detail.node_executions;
    let has_nodes = !node_execs.is_empty();
    let run_input = detail
        .input_data
        .map(|d| serde_json::to_string_pretty(&d).unwrap_or_default());
    let run_output = detail
        .output_data
        .map(|d| serde_json::to_string_pretty(&d).unwrap_or_default());

    view! {
        <div class="run-detail-content">
            <div class="run-summary">
                <p><strong>"Status:"</strong>" "<span class=status_class>{run_state}</span></p>
                <p><strong>"Duration:"</strong>" "{duration}</p>
                {run_error.map(|e| view! {
                    <p class="run-error"><strong>"Error:"</strong>" "{e}</p>
                })}
            </div>

            <h4>"Node Executions"</h4>
            {if !has_nodes {
                view! {
                    <p class="empty-state">"No node executions recorded."</p>
                }.into_any()
            } else {
                view! {
                    <div class="node-executions">
                        {node_execs.into_iter().map(|exec| {
                            view! { <NodeExecutionItem exec=exec /> }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}

            // Input/Output data for the run
            {run_input.map(|data| view! {
                <details class="run-data">
                    <summary>"Run Input Data"</summary>
                    <pre>{data}</pre>
                </details>
            })}
            {run_output.map(|data| view! {
                <details class="run-data">
                    <summary>"Run Output Data"</summary>
                    <pre>{data}</pre>
                </details>
            })}
        </div>
    }
}

/// Individual node execution item.
#[component]
fn NodeExecutionItem(exec: NodeExecutionSummary) -> impl IntoView {
    let node_id = exec.node_id;
    let node_state = exec.state.clone();
    let node_status_class = format!("status-{}", exec.state);
    let node_duration = exec
        .duration_ms
        .map(|ms| format!("{}ms", ms))
        .unwrap_or_else(|| "-".to_string());
    let error_msg = exec.error_message;
    let input_data_str = exec
        .input_data
        .map(|d| serde_json::to_string_pretty(&d).unwrap_or_default());
    let output_key = exec.output_key;

    view! {
        <div class="node-execution">
            <div class="node-exec-header">
                <span class="node-id">{node_id}</span>
                <span class=node_status_class>{node_state}</span>
                <span class="node-duration">{node_duration}</span>
            </div>
            {error_msg.map(|e| view! {
                <div class="node-error">
                    <strong>"Error:"</strong>" "{e}
                </div>
            })}
            {input_data_str.map(|data| view! {
                <details class="node-data">
                    <summary>"Input Data"</summary>
                    <pre>{data}</pre>
                </details>
            })}
            {output_key.map(|key| view! {
                <div class="node-output-key">
                    <strong>"Output Key:"</strong>" "{key}
                </div>
            })}
        </div>
    }
}
