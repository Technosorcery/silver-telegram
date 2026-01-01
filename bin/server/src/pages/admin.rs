//! Admin page component and server functions.

use crate::user::get_current_user;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Workflow info for admin display.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub last_run_state: Option<String>,
}

/// Server function to list all workflows (admin only).
#[server]
pub async fn list_all_workflows() -> Result<Vec<WorkflowSummary>, ServerFnError> {
    use crate::db::WorkflowRepository;
    use crate::error::WorkflowError;
    use crate::server_helpers::get_admin_session;

    let _auth = get_admin_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Admin auth failed for list_all_workflows");
        e.into_server_error()
    })?;

    let db_pool: sqlx::PgPool = leptos::prelude::expect_context();
    let workflow_repo = WorkflowRepository::new(db_pool);

    let workflows = workflow_repo.list_all_summaries().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to load workflow summaries");
        WorkflowError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    tracing::info!(count = workflows.len(), "Listed all workflows for admin");

    Ok(workflows
        .into_iter()
        .map(|summary| WorkflowSummary {
            id: summary.id.to_string(),
            name: summary.name,
            description: summary.description,
            enabled: summary.enabled,
            last_run: summary.last_run_at.map(|dt| dt.to_rfc3339()),
            last_run_state: summary.last_run_state,
        })
        .collect())
}

/// Server function to trigger a workflow (admin only).
#[server]
pub async fn trigger_workflow(workflow_id: String) -> Result<(), ServerFnError> {
    use crate::db::{WorkflowRepository, WorkflowRunRecord, WorkflowRunRepository};
    use crate::error::{WorkflowError, WorkflowRunError};
    use crate::server_helpers::get_admin_session;
    use silver_telegram_core::WorkflowId;
    use std::str::FromStr;

    let _auth = get_admin_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Admin auth failed for trigger_workflow");
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

    // Verify workflow exists
    let db_pool: sqlx::PgPool = leptos::prelude::expect_context();
    let workflow_repo = WorkflowRepository::new(db_pool.clone());
    let workflow = workflow_repo
        .find_by_id(wf_id)
        .await
        .map_err(|e| {
            tracing::error!(
                workflow_id = %workflow_id,
                error = %e,
                "Database error finding workflow"
            );
            WorkflowError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?
        .ok_or_else(|| {
            tracing::debug!(workflow_id = %workflow_id, "Workflow not found");
            WorkflowError::NotFound {
                id: workflow_id.clone(),
            }
            .into_server_error()
        })?;

    if !workflow.enabled {
        tracing::debug!(
            workflow_id = %workflow_id,
            "Attempted to trigger disabled workflow"
        );
        return Err(WorkflowError::InvalidState {
            id: workflow_id,
            state: "disabled".to_string(),
            required: "enabled".to_string(),
        }
        .into_server_error());
    }

    // Create a new run in queued state (orchestrator will pick it up)
    let run = WorkflowRunRecord::new(
        wf_id,
        None,
        Some(serde_json::json!({"triggered_by": "admin"})),
    );
    let run_repo = WorkflowRunRepository::new(db_pool);
    run_repo.create(&run).await.map_err(|e| {
        tracing::error!(
            workflow_id = %workflow_id,
            error = %e,
            "Failed to create workflow run"
        );
        WorkflowRunError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    tracing::info!(
        workflow_id = %workflow_id,
        "Admin triggered workflow successfully"
    );

    Ok(())
}

/// Server function to cancel a workflow run (admin only).
#[server]
pub async fn cancel_workflow(workflow_id: String) -> Result<(), ServerFnError> {
    use crate::db::WorkflowRunRepository;
    use crate::error::{WorkflowError, WorkflowRunError};
    use crate::server_helpers::get_admin_session;
    use silver_telegram_core::WorkflowId;
    use std::str::FromStr;

    let _auth = get_admin_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Admin auth failed for cancel_workflow");
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

    // Cancel all active runs for this workflow
    let db_pool: sqlx::PgPool = leptos::prelude::expect_context();
    let run_repo = WorkflowRunRepository::new(db_pool);
    run_repo.cancel_for_workflow(wf_id).await.map_err(|e| {
        tracing::error!(
            workflow_id = %workflow_id,
            error = %e,
            "Failed to cancel workflow runs"
        );
        WorkflowRunError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    tracing::info!(
        workflow_id = %workflow_id,
        "Admin cancelled workflow runs successfully"
    );

    Ok(())
}

/// Admin page (requires admin access).
#[component]
pub fn AdminPage() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());
    let workflows = Resource::new(|| (), |_| list_all_workflows());

    view! {
        <div class="admin-page">
            <h1>"Admin"</h1>
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || {
                    user.get().map(|result| {
                        match result {
                            Ok(Some(user_info)) if user_info.is_admin => view! {
                                <div class="admin-content">
                                    <p>"Platform administration and oversight."</p>

                                    <section class="admin-section">
                                        <h2>"User Workflows"</h2>
                                        <p>"Manage workflows across all users."</p>
                                        <Suspense fallback=move || view! { <p>"Loading workflows..."</p> }>
                                            {move || {
                                                workflows.get().map(|result| {
                                                    match result {
                                                        Ok(items) if items.is_empty() => view! {
                                                            <p class="empty-state">"No workflows configured yet."</p>
                                                        }.into_any(),
                                                        Ok(items) => view! {
                                                            <table class="workflows-table">
                                                                <thead>
                                                                    <tr>
                                                                        <th>"Workflow"</th>
                                                                        <th>"Status"</th>
                                                                        <th>"Last Run"</th>
                                                                        <th>"Actions"</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody>
                                                                    {items.into_iter().map(|wf| {
                                                                        let wf_id = wf.id.clone();
                                                                        let wf_id2 = wf.id.clone();
                                                                        view! {
                                                                            <tr>
                                                                                <td>
                                                                                    <strong>{wf.name}</strong>
                                                                                    {wf.description.map(|d| view! { <br/><small>{d}</small> })}
                                                                                </td>
                                                                                <td>{if wf.enabled { "Enabled" } else { "Disabled" }}</td>
                                                                                <td>
                                                                                    {wf.last_run.unwrap_or_else(|| "Never".to_string())}
                                                                                    {wf.last_run_state.map(|s| format!(" ({})", s))}
                                                                                </td>
                                                                                <td class="workflow-actions">
                                                                                    <button
                                                                                        class="trigger-btn"
                                                                                        on:click=move |_| {
                                                                                            let id = wf_id.clone();
                                                                                            spawn_local(async move {
                                                                                                let _ = trigger_workflow(id).await;
                                                                                            });
                                                                                        }
                                                                                    >"Trigger"</button>
                                                                                    <button
                                                                                        class="cancel-btn"
                                                                                        on:click=move |_| {
                                                                                            let id = wf_id2.clone();
                                                                                            spawn_local(async move {
                                                                                                let _ = cancel_workflow(id).await;
                                                                                            });
                                                                                        }
                                                                                    >"Cancel"</button>
                                                                                </td>
                                                                            </tr>
                                                                        }
                                                                    }).collect_view()}
                                                                </tbody>
                                                            </table>
                                                        }.into_any(),
                                                        Err(_) => view! {
                                                            <p class="error">"Failed to load workflows."</p>
                                                        }.into_any(),
                                                    }
                                                })
                                            }}
                                        </Suspense>
                                    </section>
                                </div>
                            }.into_any(),
                            Ok(Some(_)) => view! {
                                <div>
                                    <p>"You do not have admin access."</p>
                                    <a href="/">"Return to Home"</a>
                                </div>
                            }.into_any(),
                            Ok(None) => view! {
                                <div>
                                    <p>"Please log in to access admin features."</p>
                                    <a href="/auth/login" rel="external">"Log in"</a>
                                </div>
                            }.into_any(),
                            Err(_) => view! {
                                <div>
                                    <p>"Failed to load page. Please try again."</p>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
