//! Workflows page component and server functions.

use crate::user::get_current_user;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// User workflow info for display.
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

/// Server function to list user's workflows.
#[server]
pub async fn list_user_workflows() -> Result<Vec<UserWorkflowInfo>, ServerFnError> {
    use crate::db::WorkflowRepository;
    use crate::error::WorkflowError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, ResourceType, Subject};
    use silver_telegram_core::WorkflowId;
    use std::str::FromStr;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for list_user_workflows");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    // Query SpiceDB for workflow IDs the user can view
    let subject = Subject::user(auth.user_id);
    let workflow_ids = authz_client
        .lookup_resources(ResourceType::Workflow, Permission::View, &subject)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %auth.user_id,
                "Failed to lookup accessible workflows from SpiceDB"
            );
            WorkflowError::AuthorizationError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    // Parse workflow IDs
    let workflow_ids: Vec<WorkflowId> = workflow_ids
        .iter()
        .filter_map(|id| WorkflowId::from_str(id).ok())
        .collect();

    // Fetch workflow details from database
    let workflow_repo = WorkflowRepository::new(db_pool);
    let workflows = workflow_repo
        .list_by_ids(&workflow_ids)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %auth.user_id,
                workflow_count = workflow_ids.len(),
                "Failed to load workflows from database"
            );
            WorkflowError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    Ok(workflows
        .into_iter()
        .map(|w| UserWorkflowInfo {
            id: w.id.to_string(),
            name: w.name,
            description: w.description,
            enabled: w.enabled,
            last_run_at: w.last_run_at.map(|dt| dt.to_rfc3339()),
            last_run_state: w.last_run_state,
            last_run_duration_ms: w.last_run_duration_ms,
        })
        .collect())
}

/// Server function to create a new workflow.
#[server]
pub async fn create_workflow(
    name: String,
    description: Option<String>,
) -> Result<String, ServerFnError> {
    use crate::db::{WorkflowRecord, WorkflowRepository};
    use crate::error::WorkflowError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::Relationship;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for create_workflow");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    let mut workflow = WorkflowRecord::new(name.clone());
    workflow.description = description;

    let workflow_repo = WorkflowRepository::new(db_pool);
    workflow_repo.create(&workflow).await.map_err(|e| {
        tracing::error!(
            error = %e,
            user_id = %auth.user_id,
            workflow_name = %name,
            "Failed to create workflow in database"
        );
        WorkflowError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    // Create ownership relationship in SpiceDB
    let relationship = Relationship::workflow_owner(workflow.id, auth.user_id);
    authz_client
        .write_relationship(&relationship)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %auth.user_id,
                workflow_id = %workflow.id,
                "Failed to set workflow ownership in SpiceDB"
            );
            WorkflowError::AuthorizationError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    tracing::info!(
        user_id = %auth.user_id,
        workflow_id = %workflow.id,
        workflow_name = %name,
        "Created new workflow"
    );

    Ok(workflow.id.to_string())
}

/// Server function to toggle workflow enabled state.
#[server]
pub async fn toggle_workflow_enabled(workflow_id: String) -> Result<bool, ServerFnError> {
    use crate::db::WorkflowRepository;
    use crate::error::WorkflowError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::WorkflowId;
    use std::str::FromStr;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for toggle_workflow_enabled");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    let wf_id = WorkflowId::from_str(&workflow_id).map_err(|e| {
        tracing::debug!(
            error = %e,
            workflow_id = %workflow_id,
            "Invalid workflow ID format"
        );
        WorkflowError::InvalidId {
            id: workflow_id.clone(),
            reason: e.to_string(),
        }
        .into_server_error()
    })?;

    // Check edit permission via SpiceDB
    let resource = Resource::workflow(wf_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::Edit, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                error = %e,
                user_id = %auth.user_id,
                workflow_id = %wf_id,
                permission = "edit",
                "Access denied to toggle workflow"
            );
            WorkflowError::AccessDenied {
                id: wf_id.to_string(),
            }
            .into_server_error()
        })?;

    let workflow_repo = WorkflowRepository::new(db_pool);
    let new_enabled = workflow_repo.toggle_enabled(wf_id).await.map_err(|e| {
        tracing::error!(
            error = %e,
            workflow_id = %wf_id,
            "Failed to toggle workflow enabled state"
        );
        WorkflowError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    tracing::info!(
        user_id = %auth.user_id,
        workflow_id = %wf_id,
        enabled = new_enabled,
        "Toggled workflow enabled state"
    );

    Ok(new_enabled)
}

/// Server function to delete a workflow.
#[server]
pub async fn delete_workflow(workflow_id: String) -> Result<(), ServerFnError> {
    use crate::db::WorkflowRepository;
    use crate::error::WorkflowError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::WorkflowId;
    use std::str::FromStr;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for delete_workflow");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    let wf_id = WorkflowId::from_str(&workflow_id).map_err(|e| {
        tracing::debug!(
            error = %e,
            workflow_id = %workflow_id,
            "Invalid workflow ID format"
        );
        WorkflowError::InvalidId {
            id: workflow_id.clone(),
            reason: e.to_string(),
        }
        .into_server_error()
    })?;

    // Check delete permission via SpiceDB
    let resource = Resource::workflow(wf_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::Delete, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                error = %e,
                user_id = %auth.user_id,
                workflow_id = %wf_id,
                permission = "delete",
                "Access denied to delete workflow"
            );
            WorkflowError::AccessDenied {
                id: wf_id.to_string(),
            }
            .into_server_error()
        })?;

    // Delete from database
    let workflow_repo = WorkflowRepository::new(db_pool);
    workflow_repo.delete(wf_id).await.map_err(|e| {
        tracing::error!(
            error = %e,
            workflow_id = %wf_id,
            "Failed to delete workflow from database"
        );
        WorkflowError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    // Delete relationships from SpiceDB
    authz_client
        .delete_relationships(&resource, None)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                workflow_id = %wf_id,
                "Failed to delete workflow relationships from SpiceDB"
            );
            WorkflowError::AuthorizationError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    tracing::info!(
        user_id = %auth.user_id,
        workflow_id = %wf_id,
        "Deleted workflow"
    );

    Ok(())
}

/// Workflows page for users.
#[component]
pub fn WorkflowsPage() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());
    let workflows = Resource::new(|| (), |_| list_user_workflows());

    let (new_wf_name, set_new_wf_name) = signal(String::new());
    let (new_wf_desc, set_new_wf_desc) = signal(String::new());
    let (creating, set_creating) = signal(false);

    // Delete confirmation state
    let (delete_id, set_delete_id) = signal(Option::<String>::None);
    let (delete_name, set_delete_name) = signal(String::new());
    let (deleting, set_deleting) = signal(false);
    let (delete_error, set_delete_error) = signal(Option::<String>::None);

    let on_confirm_delete = move |_| {
        let id = match delete_id.get() {
            Some(id) => id,
            None => return,
        };
        set_deleting.set(true);
        set_delete_error.set(None);
        spawn_local(async move {
            match delete_workflow(id).await {
                Ok(()) => {
                    set_delete_id.set(None);
                    set_delete_name.set(String::new());
                    workflows.refetch();
                }
                Err(e) => {
                    set_delete_error.set(Some(e.to_string()));
                }
            }
            set_deleting.set(false);
        });
    };

    let on_create = move |_| {
        let name = new_wf_name.get();
        if name.is_empty() {
            return;
        }
        let desc = if new_wf_desc.get().is_empty() {
            None
        } else {
            Some(new_wf_desc.get())
        };
        set_creating.set(true);
        spawn_local(async move {
            if create_workflow(name, desc).await.is_ok() {
                set_new_wf_name.set(String::new());
                set_new_wf_desc.set(String::new());
                workflows.refetch();
            }
            set_creating.set(false);
        });
    };

    view! {
        <div class="workflows-page">
            <h1>"Workflows"</h1>
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || {
                    user.get().map(|result| {
                        match result {
                            Ok(Some(_user_info)) => view! {
                                <div class="workflows-content">
                                    <p>"Create and manage your automation workflows."</p>

                                    <section class="create-workflow">
                                        <h2>"Create New Workflow"</h2>
                                        <div class="create-form">
                                            <input
                                                type="text"
                                                placeholder="Workflow name"
                                                prop:value=move || new_wf_name.get()
                                                on:input=move |ev| set_new_wf_name.set(event_target_value(&ev))
                                            />
                                            <input
                                                type="text"
                                                placeholder="Description (optional)"
                                                prop:value=move || new_wf_desc.get()
                                                on:input=move |ev| set_new_wf_desc.set(event_target_value(&ev))
                                            />
                                            <button
                                                on:click=on_create
                                                disabled=move || creating.get() || new_wf_name.get().is_empty()
                                            >
                                                {move || if creating.get() { "Creating..." } else { "Create Workflow" }}
                                            </button>
                                        </div>
                                    </section>

                                    // Delete Confirmation Modal
                                    {move || delete_id.get().map(|_| view! {
                                        <div class="modal-overlay">
                                            <div class="modal">
                                                <h2>"Delete Workflow?"</h2>
                                                <p>"Are you sure you want to delete \""{move || delete_name.get()}"\"?"</p>
                                                <p class="warning">"This action cannot be undone. All execution history will be lost."</p>
                                                {move || delete_error.get().map(|e| view! {
                                                    <p class="error">{e}</p>
                                                })}
                                                <div class="modal-actions">
                                                    <button
                                                        class="secondary-btn"
                                                        on:click=move |_| {
                                                            set_delete_id.set(None);
                                                            set_delete_name.set(String::new());
                                                            set_delete_error.set(None);
                                                        }
                                                    >"Cancel"</button>
                                                    <button
                                                        class="danger-btn"
                                                        on:click=on_confirm_delete
                                                        disabled=move || deleting.get()
                                                    >
                                                        {move || if deleting.get() { "Deleting..." } else { "Delete" }}
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    })}

                                    <section class="workflows-list">
                                        <h2>"Your Workflows"</h2>
                                        <Suspense fallback=move || view! { <p>"Loading workflows..."</p> }>
                                            {move || {
                                                workflows.get().map(|result| {
                                                    match result {
                                                        Ok(items) if items.is_empty() => view! {
                                                            <p class="empty-state">"No workflows yet. Create one above!"</p>
                                                        }.into_any(),
                                                        Ok(items) => view! {
                                                            <table class="workflows-table">
                                                                <thead>
                                                                    <tr>
                                                                        <th>"Name"</th>
                                                                        <th>"Status"</th>
                                                                        <th>"Last Run"</th>
                                                                        <th>"Actions"</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody>
                                                                    {items.into_iter().map(|wf| {
                                                                        let wf_id = wf.id.clone();
                                                                        let wf_id2 = wf.id.clone();
                                                                        let wf_name = wf.name.clone();
                                                                        let wf_name_for_delete = wf.name.clone();
                                                                        let enabled = wf.enabled;
                                                                        view! {
                                                                            <tr>
                                                                                <td>
                                                                                    <strong>{wf_name}</strong>
                                                                                    {wf.description.map(|d| view! { <br/><small>{d}</small> })}
                                                                                </td>
                                                                                <td class=move || if enabled { "status-enabled" } else { "status-disabled" }>
                                                                                    {if enabled { "Enabled" } else { "Disabled" }}
                                                                                </td>
                                                                                <td>
                                                                                    {wf.last_run_at.map(|dt| view! { <span>{dt}</span> }.into_any()).unwrap_or_else(|| view! { <span class="muted">"Never"</span> }.into_any())}
                                                                                    {wf.last_run_state.map(|s| view! { <span class="run-state">{format!(" ({})", s)}</span> })}
                                                                                </td>
                                                                                <td class="workflow-actions">
                                                                                    <button
                                                                                        class="toggle-btn"
                                                                                        on:click=move |_| {
                                                                                            let id = wf_id.clone();
                                                                                            spawn_local(async move {
                                                                                                if toggle_workflow_enabled(id).await.is_ok() {
                                                                                                    workflows.refetch();
                                                                                                }
                                                                                            });
                                                                                        }
                                                                                    >
                                                                                        {if enabled { "Disable" } else { "Enable" }}
                                                                                    </button>
                                                                                    <a
                                                                                        href=format!("/workflows/{}", wf_id2.clone())
                                                                                        class="edit-btn"
                                                                                    >"Edit"</a>
                                                                                    <button
                                                                                        class="delete-btn"
                                                                                        on:click=move |_| {
                                                                                            set_delete_id.set(Some(wf_id2.clone()));
                                                                                            set_delete_name.set(wf_name_for_delete.clone());
                                                                                        }
                                                                                    >"Delete"</button>
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
                            Ok(None) => view! {
                                <div>
                                    <p>"Please log in to manage workflows."</p>
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
