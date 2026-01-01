//! Workflow editor page module.
//!
//! Provides the visual workflow editor with tabs for graph editing,
//! settings, memory, and execution history.

mod editor;
mod graph;
mod history;

pub use graph::{WorkflowEdge, WorkflowGraph, WorkflowNode, update_workflow_graph};
pub use history::{
    DecisionTraceSummary, NodeExecutionSummary, RunDetailView, WorkflowRunSummary, get_run_detail,
    list_workflow_runs,
};

use crate::pages::integrations::list_integrations;
use editor::EditorTabContent;
use history::HistoryTab;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::{hooks::use_params, params::Params};

/// URL params for workflow editor.
#[derive(Params, PartialEq, Clone, Debug)]
struct WorkflowParams {
    id: Option<String>,
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

/// Server function to get workflow details for editing.
#[server]
pub async fn get_workflow_detail(workflow_id: String) -> Result<WorkflowDetail, ServerFnError> {
    use crate::db::{WorkflowMemoryRepository, WorkflowRepository};
    use crate::error::WorkflowError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::WorkflowId;
    use std::str::FromStr;

    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for get_workflow_detail");
        e.into_server_error()
    })?;
    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

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
                id: workflow_id.clone(),
            }
            .into_server_error()
        })?;

    let workflow_repo = WorkflowRepository::new(db_pool.clone());
    let workflow = workflow_repo
        .find_by_id(wf_id)
        .await
        .map_err(|e| {
            tracing::error!(
                workflow_id = %wf_id,
                error = %e,
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
                id: workflow_id.clone(),
            }
            .into_server_error()
        })?;

    // Get memory content
    let memory_repo = WorkflowMemoryRepository::new(db_pool);
    let memory_content = match memory_repo.find_by_workflow(wf_id).await {
        Ok(Some(mem)) => String::from_utf8(mem.content).ok(),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(
                workflow_id = %wf_id,
                error = %e,
                "Failed to load workflow memory, continuing without it"
            );
            None
        }
    };

    Ok(WorkflowDetail {
        id: workflow.id.to_string(),
        name: workflow.name,
        description: workflow.description,
        enabled: workflow.enabled,
        graph_data: workflow.graph_data.to_string(),
        memory_content,
    })
}

/// Server function to update workflow details.
#[server]
pub async fn update_workflow_detail(
    workflow_id: String,
    name: String,
    description: Option<String>,
) -> Result<(), ServerFnError> {
    use crate::db::WorkflowRepository;
    use crate::error::WorkflowError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::WorkflowId;
    use std::str::FromStr;

    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for update_workflow_detail");
        e.into_server_error()
    })?;
    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

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
                id: workflow_id.clone(),
            }
            .into_server_error()
        })?;

    let workflow_repo = WorkflowRepository::new(db_pool);
    let mut workflow = workflow_repo
        .find_by_id(wf_id)
        .await
        .map_err(|e| {
            tracing::error!(
                workflow_id = %wf_id,
                error = %e,
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
                id: workflow_id.clone(),
            }
            .into_server_error()
        })?;

    workflow.name = name.clone();
    workflow.description = description.clone();
    workflow.updated_at = chrono::Utc::now();

    workflow_repo.update(&workflow).await.map_err(|e| {
        tracing::error!(
            workflow_id = %wf_id,
            error = %e,
            "Failed to update workflow details"
        );
        WorkflowError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    tracing::info!(
        workflow_id = %wf_id,
        name = %name,
        "Updated workflow details"
    );

    Ok(())
}

/// Server function to update workflow memory.
#[server]
pub async fn update_workflow_memory(
    workflow_id: String,
    content: String,
) -> Result<(), ServerFnError> {
    use crate::db::WorkflowMemoryRepository;
    use crate::error::WorkflowError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::WorkflowId;
    use std::str::FromStr;

    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for update_workflow_memory");
        e.into_server_error()
    })?;
    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

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
                "Access denied to edit workflow memory"
            );
            WorkflowError::AccessDenied {
                id: workflow_id.clone(),
            }
            .into_server_error()
        })?;

    let content_size = content.len();
    let memory_repo = WorkflowMemoryRepository::new(db_pool);
    memory_repo
        .upsert(wf_id, content.into_bytes(), None)
        .await
        .map_err(|e| {
            tracing::error!(
                workflow_id = %wf_id,
                error = %e,
                "Failed to update workflow memory"
            );
            WorkflowError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    tracing::info!(
        workflow_id = %wf_id,
        content_size = content_size,
        "Updated workflow memory"
    );

    Ok(())
}

/// Workflow editor page.
#[component]
pub fn WorkflowEditorPage() -> impl IntoView {
    let params = use_params::<WorkflowParams>();
    let workflow_id = Signal::derive(move || params.get().ok().and_then(|p| p.id));

    let workflow = Resource::new(
        move || workflow_id.get(),
        |id| async move {
            match id {
                Some(id) => get_workflow_detail(id).await.ok(),
                None => None,
            }
        },
    );

    // State for editing
    let (edit_name, set_edit_name) = signal(String::new());
    let (edit_desc, set_edit_desc) = signal(String::new());
    let (graph, set_graph) = signal(WorkflowGraph::default());
    let (memory_content, set_memory_content) = signal(String::new());
    let (saving, set_saving) = signal(false);
    let (active_tab, set_active_tab) = signal("editor".to_string());

    // Selected node for configuration
    let (selected_node_id, set_selected_node_id) = signal(Option::<String>::None);

    // Available integrations for tool nodes
    let available_integrations = Resource::new(
        || (),
        |_| async move { list_integrations().await.ok().unwrap_or_default() },
    );

    // Initialize form when workflow loads
    Effect::new(move || {
        if let Some(Some(wf)) = workflow.get() {
            set_edit_name.set(wf.name.clone());
            set_edit_desc.set(wf.description.clone().unwrap_or_default());
            set_memory_content.set(wf.memory_content.clone().unwrap_or_default());

            // Parse graph
            if let Ok(g) = serde_json::from_str::<WorkflowGraph>(&wf.graph_data) {
                set_graph.set(g);
            }
        }
    });

    let on_save = move |_| {
        let wf_id = match workflow_id.get() {
            Some(id) => id,
            None => return,
        };
        let name = edit_name.get();
        let desc = if edit_desc.get().is_empty() {
            None
        } else {
            Some(edit_desc.get())
        };
        let g = graph.get();
        let graph_json = serde_json::to_string(&g).unwrap_or_default();
        let mem = memory_content.get();

        set_saving.set(true);
        spawn_local(async move {
            // Save details
            let _ = update_workflow_detail(wf_id.clone(), name, desc).await;

            // Save graph
            let _ = update_workflow_graph(wf_id.clone(), graph_json).await;

            // Save memory
            let _ = update_workflow_memory(wf_id, mem).await;

            set_saving.set(false);
        });
    };

    view! {
        <div class="workflow-editor-page">
            <Suspense fallback=move || view! { <p>"Loading workflow..."</p> }>
                {move || {
                    match workflow.get() {
                        Some(Some(wf)) => {
                            let wf_name = wf.name.clone();
                            view! {
                                <div class="workflow-editor">
                                    <EditorHeader
                                        wf_name=wf_name
                                        saving=saving
                                        on_save=on_save
                                    />

                                    <EditorTabs
                                        active_tab=active_tab
                                        set_active_tab=set_active_tab
                                    />

                                    // Editor Tab
                                    {move || (active_tab.get() == "editor").then(|| view! {
                                        <EditorTabContent
                                            graph=graph
                                            set_graph=set_graph
                                            selected_node_id=selected_node_id
                                            set_selected_node_id=set_selected_node_id
                                            available_integrations=available_integrations
                                        />
                                    })}

                                    // Settings Tab
                                    {move || (active_tab.get() == "settings").then(|| view! {
                                        <SettingsTabContent
                                            edit_name=edit_name
                                            set_edit_name=set_edit_name
                                            edit_desc=edit_desc
                                            set_edit_desc=set_edit_desc
                                        />
                                    })}

                                    // Memory Tab
                                    {move || (active_tab.get() == "memory").then(|| view! {
                                        <MemoryTabContent
                                            memory_content=memory_content
                                            set_memory_content=set_memory_content
                                        />
                                    })}

                                    // History Tab
                                    {move || (active_tab.get() == "history").then(|| view! {
                                        <HistoryTab workflow_id=workflow_id />
                                    })}
                                </div>
                            }.into_any()
                        },
                        Some(None) => view! {
                            <div class="not-found">
                                <h1>"Workflow Not Found"</h1>
                                <p>"The workflow you're looking for doesn't exist or you don't have access to it."</p>
                                <a href="/workflows">"Back to Workflows"</a>
                            </div>
                        }.into_any(),
                        None => view! { <p>"Loading..."</p> }.into_any(),
                    }
                }}
            </Suspense>
        </div>
    }
}

/// Editor header with title and save button.
#[component]
fn EditorHeader(
    wf_name: String,
    saving: ReadSignal<bool>,
    on_save: impl Fn(leptos::web_sys::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <header class="editor-header">
            <a href="/workflows" class="back-link">"← Back to Workflows"</a>
            <h1>{wf_name}</h1>
            <button
                class="save-btn primary-btn"
                on:click=on_save
                disabled=move || saving.get()
            >
                {move || if saving.get() { "Saving..." } else { "Save Changes" }}
            </button>
        </header>
    }
}

/// Tab navigation for the editor.
#[component]
fn EditorTabs(
    active_tab: ReadSignal<String>,
    set_active_tab: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="editor-tabs">
            <button
                class=move || if active_tab.get() == "editor" { "tab active" } else { "tab" }
                on:click=move |_| set_active_tab.set("editor".to_string())
            >"Editor"</button>
            <button
                class=move || if active_tab.get() == "settings" { "tab active" } else { "tab" }
                on:click=move |_| set_active_tab.set("settings".to_string())
            >"Settings"</button>
            <button
                class=move || if active_tab.get() == "memory" { "tab active" } else { "tab" }
                on:click=move |_| set_active_tab.set("memory".to_string())
            >"Memory"</button>
            <button
                class=move || if active_tab.get() == "history" { "tab active" } else { "tab" }
                on:click=move |_| set_active_tab.set("history".to_string())
            >"History"</button>
        </div>
    }
}

/// Settings tab content.
#[component]
fn SettingsTabContent(
    edit_name: ReadSignal<String>,
    set_edit_name: WriteSignal<String>,
    edit_desc: ReadSignal<String>,
    set_edit_desc: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="settings-content">
            <div class="form-group">
                <label>"Workflow Name"</label>
                <input
                    type="text"
                    prop:value=move || edit_name.get()
                    on:input=move |ev| set_edit_name.set(event_target_value(&ev))
                />
            </div>
            <div class="form-group">
                <label>"Description"</label>
                <textarea
                    rows="3"
                    prop:value=move || edit_desc.get()
                    on:input=move |ev| set_edit_desc.set(event_target_value(&ev))
                ></textarea>
            </div>
        </div>
    }
}

/// Memory tab content.
#[component]
fn MemoryTabContent(
    memory_content: ReadSignal<String>,
    set_memory_content: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="memory-content">
            <p>"Edit the raw memory content. This persists across workflow runs."</p>
            <textarea
                class="memory-editor"
                rows="20"
                prop:value=move || memory_content.get()
                on:input=move |ev| set_memory_content.set(event_target_value(&ev))
            ></textarea>
        </div>
    }
}
