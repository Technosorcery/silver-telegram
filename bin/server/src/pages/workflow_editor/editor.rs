//! Editor tab content with visual node canvas and configuration panel.

use super::graph::{WorkflowEdge, WorkflowGraph, WorkflowNode};
use crate::pages::integrations::{IntegrationInfo, ModelInfo, discover_models};
use leptos::prelude::*;

/// Node dimensions for layout calculations.
const NODE_WIDTH: f64 = 160.0;
const NODE_HEIGHT: f64 = 60.0;

/// Editor tab content with visual node canvas and config panel.
#[component]
pub fn EditorTabContent(
    graph: ReadSignal<WorkflowGraph>,
    set_graph: WriteSignal<WorkflowGraph>,
    selected_node_id: ReadSignal<Option<String>>,
    set_selected_node_id: WriteSignal<Option<String>>,
    available_integrations: Resource<Vec<IntegrationInfo>>,
) -> impl IntoView {
    // Track which node is being dragged
    let (dragging_node, set_dragging_node) = signal(Option::<String>::None);

    // Track connection mode (drawing an edge from a node)
    let (connecting_from, set_connecting_from) = signal(Option::<String>::None);

    // Add node at a default position
    let add_node = move |node_type: &str| {
        let mut g = graph.get();
        let id = format!("node_{}", ulid::Ulid::new());
        let label = match node_type {
            "trigger" => "Schedule Trigger",
            "model" => "Model",
            "ai" => "AI Node",
            "tool" => "Tool Node",
            "data" => "Data Injection",
            _ => "Node",
        };
        // Position new nodes in a grid pattern
        let count = g.nodes.len();
        let col = count % 3;
        let row = count / 3;
        let node = WorkflowNode {
            id,
            node_type: node_type.to_string(),
            label: label.to_string(),
            config: "{}".to_string(),
            x: 80.0 + (col as f64 * 200.0),
            y: 80.0 + (row as f64 * 120.0),
        };
        g.nodes.push(node);
        set_graph.set(g);
    };

    view! {
        <div class="editor-content">
            <div class="node-toolbar">
                <span class="toolbar-label">"Add Node:"</span>
                <button class="toolbar-btn trigger" on:click=move |_| add_node("trigger")>"Trigger"</button>
                <button class="toolbar-btn model" on:click=move |_| add_node("model")>"Model"</button>
                <button class="toolbar-btn ai" on:click=move |_| add_node("ai")>"AI"</button>
                <button class="toolbar-btn tool" on:click=move |_| add_node("tool")>"Tool"</button>
                <button class="toolbar-btn data" on:click=move |_| add_node("data")>"Data"</button>
                <span class="toolbar-spacer"></span>
                {move || connecting_from.get().map(|_| view! {
                    <span class="connecting-hint">"Click a node to connect, or "</span>
                    <button class="cancel-btn" on:click=move |_| set_connecting_from.set(None)>"Cancel"</button>
                })}
            </div>

            <div class="editor-layout">
                <NodeCanvas
                    graph=graph
                    set_graph=set_graph
                    selected_node_id=selected_node_id
                    set_selected_node_id=set_selected_node_id
                    dragging_node=dragging_node
                    set_dragging_node=set_dragging_node
                    connecting_from=connecting_from
                    set_connecting_from=set_connecting_from
                />

                <NodeConfigPanel
                    graph=graph
                    set_graph=set_graph
                    selected_node_id=selected_node_id
                    set_selected_node_id=set_selected_node_id
                    available_integrations=available_integrations
                />
            </div>
        </div>
    }
}

/// SVG-based visual node canvas.
#[component]
fn NodeCanvas(
    graph: ReadSignal<WorkflowGraph>,
    set_graph: WriteSignal<WorkflowGraph>,
    selected_node_id: ReadSignal<Option<String>>,
    set_selected_node_id: WriteSignal<Option<String>>,
    dragging_node: ReadSignal<Option<String>>,
    set_dragging_node: WriteSignal<Option<String>>,
    connecting_from: ReadSignal<Option<String>>,
    set_connecting_from: WriteSignal<Option<String>>,
) -> impl IntoView {
    // Track last mouse position for drag delta calculation
    let (last_mouse_pos, set_last_mouse_pos) = signal((0.0f64, 0.0f64));

    // Handle mouse move for dragging
    let on_mouse_move = move |ev: leptos::ev::MouseEvent| {
        if let Some(node_id) = dragging_node.get() {
            let current_x = ev.client_x() as f64;
            let current_y = ev.client_y() as f64;
            let (last_x, last_y) = last_mouse_pos.get();

            // Calculate delta from last position
            let dx = current_x - last_x;
            let dy = current_y - last_y;

            if last_x > 0.0 || last_y > 0.0 {
                let mut g = graph.get();
                if let Some(node) = g.nodes.iter_mut().find(|n| n.id == node_id) {
                    node.x += dx;
                    node.y += dy;
                }
                set_graph.set(g);
            }

            set_last_mouse_pos.set((current_x, current_y));
        }
    };

    let on_mouse_up = move |_: leptos::ev::MouseEvent| {
        set_dragging_node.set(None);
        set_last_mouse_pos.set((0.0, 0.0));
    };

    // Delete a node
    let delete_node = move |node_id: String| {
        let mut g = graph.get();
        g.nodes.retain(|n| n.id != node_id);
        g.edges
            .retain(|e| e.source != node_id && e.target != node_id);
        if selected_node_id.get() == Some(node_id) {
            set_selected_node_id.set(None);
        }
        set_graph.set(g);
    };

    // Add edge between nodes
    let add_edge = move |source: String, target: String| {
        if source == target {
            return;
        }
        let mut g = graph.get();
        // Check if edge already exists
        if g.edges
            .iter()
            .any(|e| e.source == source && e.target == target)
        {
            return;
        }
        let id = format!("edge_{}", ulid::Ulid::new());
        let edge = WorkflowEdge {
            id,
            source,
            target,
            source_port: "output".to_string(),
            target_port: "input".to_string(),
        };
        g.edges.push(edge);
        set_graph.set(g);
    };

    view! {
        <div class="node-canvas-container">
            <svg
                class="node-canvas-svg"
                viewBox="0 0 800 500"
                on:mousemove=on_mouse_move
                on:mouseup=on_mouse_up
                on:mouseleave=move |_| set_dragging_node.set(None)
            >
                // Grid background
                <defs>
                    <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
                        <path d="M 20 0 L 0 0 0 20" fill="none" stroke="#2a2a2a" stroke-width="0.5"/>
                    </pattern>
                </defs>
                <rect width="100%" height="100%" fill="url(#grid)"/>

                // Render edges first (under nodes)
                {move || {
                    let g = graph.get();
                    g.edges.iter().map(|edge| {
                        let source_node = g.nodes.iter().find(|n| n.id == edge.source);
                        let target_node = g.nodes.iter().find(|n| n.id == edge.target);
                        if let (Some(src), Some(tgt)) = (source_node, target_node) {
                            let x1 = src.x + NODE_WIDTH;
                            let y1 = src.y + NODE_HEIGHT / 2.0;
                            let x2 = tgt.x;
                            let y2 = tgt.y + NODE_HEIGHT / 2.0;
                            // Bezier curve for smooth connection
                            let ctrl_offset = ((x2 - x1).abs() / 2.0).max(50.0);
                            let path = format!(
                                "M {} {} C {} {} {} {} {} {}",
                                x1, y1,
                                x1 + ctrl_offset, y1,
                                x2 - ctrl_offset, y2,
                                x2, y2
                            );
                            let edge_id = edge.id.clone();
                            Some(view! {
                                <g class="edge-group">
                                    <path
                                        class="edge-path"
                                        d=path.clone()
                                        fill="none"
                                        stroke="#666"
                                        stroke-width="2"
                                        marker-end="url(#arrowhead)"
                                    />
                                    <path
                                        class="edge-hitbox"
                                        d=path
                                        fill="none"
                                        stroke="transparent"
                                        stroke-width="10"
                                        on:click=move |_| {
                                            let mut g = graph.get();
                                            g.edges.retain(|e| e.id != edge_id);
                                            set_graph.set(g);
                                        }
                                    />
                                </g>
                            })
                        } else {
                            None
                        }
                    }).collect_view()
                }}

                // Arrow marker definition
                <defs>
                    <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
                        <polygon points="0 0, 10 3.5, 0 7" fill="#666"/>
                    </marker>
                </defs>

                // Render nodes
                {move || {
                    let g = graph.get();
                    let sel_id = selected_node_id.get();
                    let conn_from = connecting_from.get();
                    g.nodes.iter().map(|node| {
                        let node_id = node.id.clone();
                        let node_id_select = node.id.clone();
                        let node_id_drag = node.id.clone();
                        let node_id_delete = node.id.clone();
                        let node_id_connect_start = node.id.clone();
                        let node_id_connect_end = node.id.clone();
                        let node_type = node.node_type.clone();
                        let label = node.label.clone();
                        let x = node.x;
                        let y = node.y;
                        let is_selected = sel_id.as_ref() == Some(&node_id);
                        let is_connecting = conn_from.is_some();
                        let type_class = format!("node-type-{}", node_type);

                        view! {
                            <g
                                class=format!("workflow-node {} {}", type_class, if is_selected { "selected" } else { "" })
                                transform=format!("translate({}, {})", x, y)
                                on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                    ev.prevent_default();
                                    if connecting_from.get().is_some() {
                                        // Complete connection
                                        if let Some(from_id) = connecting_from.get() {
                                            add_edge(from_id, node_id_connect_end.clone());
                                        }
                                        set_connecting_from.set(None);
                                    } else {
                                        // Start drag
                                        set_selected_node_id.set(Some(node_id_select.clone()));
                                        set_last_mouse_pos.set((ev.client_x() as f64, ev.client_y() as f64));
                                        set_dragging_node.set(Some(node_id_drag.clone()));
                                    }
                                }
                            >
                                // Node background
                                <rect
                                    class="node-bg"
                                    width=NODE_WIDTH
                                    height=NODE_HEIGHT
                                    rx="6"
                                    ry="6"
                                />

                                // Node type indicator bar
                                <rect
                                    class="node-type-bar"
                                    width=NODE_WIDTH
                                    height="6"
                                    rx="6"
                                    ry="6"
                                />
                                <rect
                                    class="node-type-bar-bottom"
                                    y="3"
                                    width=NODE_WIDTH
                                    height="3"
                                />

                                // Node label
                                <text
                                    class="node-label"
                                    x=NODE_WIDTH / 2.0
                                    y="28"
                                    text-anchor="middle"
                                >{label}</text>

                                // Node type text
                                <text
                                    class="node-type-text"
                                    x=NODE_WIDTH / 2.0
                                    y="45"
                                    text-anchor="middle"
                                >{node_type}</text>

                                // Input port (left side)
                                <circle
                                    class="port input-port"
                                    cx="0"
                                    cy=NODE_HEIGHT / 2.0
                                    r="6"
                                />

                                // Output port (right side) - click to start connection
                                <circle
                                    class=format!("port output-port {}", if is_connecting { "connecting" } else { "" })
                                    cx=NODE_WIDTH
                                    cy=NODE_HEIGHT / 2.0
                                    r="6"
                                    on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                        ev.stop_propagation();
                                        set_connecting_from.set(Some(node_id_connect_start.clone()));
                                    }
                                />

                                // Delete button
                                <g
                                    class="delete-btn"
                                    transform=format!("translate({}, 0)", NODE_WIDTH - 16.0)
                                    on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                        ev.stop_propagation();
                                        delete_node(node_id_delete.clone());
                                    }
                                >
                                    <circle cx="8" cy="8" r="8" class="delete-bg"/>
                                    <text x="8" y="12" text-anchor="middle" class="delete-x">"×"</text>
                                </g>
                            </g>
                        }
                    }).collect_view()
                }}
            </svg>

            // Empty state
            {move || {
                let g = graph.get();
                if g.nodes.is_empty() {
                    Some(view! {
                        <div class="canvas-empty-state">
                            <p>"No nodes yet."</p>
                            <p>"Add nodes using the toolbar above, then connect them by clicking output ports."</p>
                        </div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}

/// Node configuration panel.
#[component]
fn NodeConfigPanel(
    graph: ReadSignal<WorkflowGraph>,
    set_graph: WriteSignal<WorkflowGraph>,
    selected_node_id: ReadSignal<Option<String>>,
    set_selected_node_id: WriteSignal<Option<String>>,
    available_integrations: Resource<Vec<IntegrationInfo>>,
) -> impl IntoView {
    let update_node_config = move |node_id: String, config: String| {
        let mut g = graph.get();
        if let Some(node) = g.nodes.iter_mut().find(|n| n.id == node_id) {
            node.config = config;
        }
        set_graph.set(g);
    };

    let update_node_label = move |node_id: String, label: String| {
        let mut g = graph.get();
        if let Some(node) = g.nodes.iter_mut().find(|n| n.id == node_id) {
            node.label = label;
        }
        set_graph.set(g);
    };

    view! {
        <div class="node-config-panel">
            {move || {
                let sel_id = selected_node_id.get();
                let g = graph.get();
                let selected_node = sel_id.as_ref()
                    .and_then(|id| g.nodes.iter().find(|n| &n.id == id).cloned());

                match selected_node {
                    Some(node) => {
                        let node_id = node.id.clone();
                        let node_id_trigger = node.id.clone();
                        let node_id_model = node.id.clone();
                        let node_id_ai = node.id.clone();
                        let node_id_tool = node.id.clone();
                        let node_id_tool_int = node.id.clone();
                        let node_id_data = node.id.clone();
                        let node_type = node.node_type.clone();
                        let label = node.label.clone();
                        let config = node.config.clone();
                        let config_for_tool = node.config.clone();
                        let config_for_model = node.config.clone();

                        view! {
                            <div class="config-content">
                                <div class="config-header">
                                    <h3>"Node Configuration"</h3>
                                    <button
                                        class="close-config"
                                        on:click=move |_| set_selected_node_id.set(None)
                                    >"×"</button>
                                </div>

                                <div class="form-group">
                                    <label>"Label"</label>
                                    <input
                                        type="text"
                                        value=label
                                        on:change=move |ev| {
                                            update_node_label(node_id.clone(), event_target_value(&ev));
                                        }
                                    />
                                </div>

                                // Type-specific config
                                {match node_type.as_str() {
                                    "trigger" => {
                                        let current_cron = serde_json::from_str::<serde_json::Value>(&config)
                                            .ok()
                                            .and_then(|v: serde_json::Value| v.get("cron").and_then(|c| c.as_str()).map(|s| s.to_string()))
                                            .unwrap_or_default();
                                        view! {
                                            <div class="type-config">
                                                <div class="form-group">
                                                    <label>"Schedule (Cron)"</label>
                                                    <input
                                                        type="text"
                                                        placeholder="0 9 * * *"
                                                        value=current_cron
                                                        on:change=move |ev| {
                                                            let cron = event_target_value(&ev);
                                                            let cfg = serde_json::json!({"cron": cron}).to_string();
                                                            update_node_config(node_id_trigger.clone(), cfg);
                                                        }
                                                    />
                                                </div>
                                                <p class="help">"Examples: '0 9 * * *' (9am daily), '*/15 * * * *' (every 15 min)"</p>
                                            </div>
                                        }.into_any()
                                    },
                                    "ai" => {
                                        let current_prompt = serde_json::from_str::<serde_json::Value>(&config)
                                            .ok()
                                            .and_then(|v: serde_json::Value| v.get("prompt").and_then(|p| p.as_str()).map(|s| s.to_string()))
                                            .unwrap_or_default();
                                        view! {
                                            <div class="type-config">
                                                <div class="form-group">
                                                    <label>"System Prompt"</label>
                                                    <textarea
                                                        rows="6"
                                                        placeholder="Instructions for the AI..."
                                                        on:change=move |ev| {
                                                            let prompt = event_target_value(&ev);
                                                            let cfg = serde_json::json!({"prompt": prompt}).to_string();
                                                            update_node_config(node_id_ai.clone(), cfg);
                                                        }
                                                    >
                                                        {current_prompt}
                                                    </textarea>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "tool" => {
                                        let current_int_id = serde_json::from_str::<serde_json::Value>(&config_for_tool)
                                            .ok()
                                            .and_then(|v: serde_json::Value| v.get("integration_id").and_then(|i| i.as_str()).map(|s| s.to_string()))
                                            .unwrap_or_default();
                                        let current_int_id_for_mode = current_int_id.clone();
                                        let current_mode = serde_json::from_str::<serde_json::Value>(&config_for_tool)
                                            .ok()
                                            .and_then(|v: serde_json::Value| v.get("mode").and_then(|m| m.as_str()).map(|s| s.to_string()))
                                            .unwrap_or_else(|| "read".to_string());
                                        let current_mode_for_select = current_mode.clone();
                                        view! {
                                            <div class="type-config">
                                                <div class="form-group">
                                                    <label>"Integration"</label>
                                                    <Suspense fallback=move || view! { <select disabled=true><option>"Loading..."</option></select> }>
                                                        {
                                                            let mode_for_int = current_mode.clone();
                                                            let node_id_for_int = node_id_tool_int.clone();
                                                            move || {
                                                                let integrations = available_integrations.get().unwrap_or_default();
                                                                let current_id = current_int_id.clone();
                                                                let mode_val = mode_for_int.clone();
                                                                let nid = node_id_for_int.clone();
                                                                view! {
                                                                    <select on:change=move |ev| {
                                                                        let int_id = event_target_value(&ev);
                                                                        let cfg = serde_json::json!({
                                                                            "integration_id": int_id,
                                                                            "mode": mode_val.clone()
                                                                        }).to_string();
                                                                        update_node_config(nid.clone(), cfg);
                                                                    }>
                                                                        <option value="" selected={current_id.is_empty()}>"-- Select --"</option>
                                                                        // Built-in tools
                                                                        <option value="__workflow_memory__" selected={current_id == "__workflow_memory__"}>"Workflow Memory"</option>
                                                                        // User integrations
                                                                        {integrations.into_iter().map(|int| {
                                                                            let id = int.id.clone();
                                                                            let is_selected = id == current_id;
                                                                            let type_label = match int.integration_type.as_str() {
                                                                                "imap" => "Email",
                                                                                "gmail" => "Gmail",
                                                                                "calendar_feed" => "Calendar",
                                                                                other => other
                                                                            };
                                                                            view! {
                                                                                <option value=id selected=is_selected>
                                                                                    {format!("{} ({})", int.name, type_label)}
                                                                                </option>
                                                                            }
                                                                        }).collect_view()}
                                                                    </select>
                                                                }
                                                            }
                                                        }
                                                    </Suspense>
                                                </div>
                                                <div class="form-group">
                                                    <label>"Access Mode"</label>
                                                    <select on:change=move |ev| {
                                                        let mode = event_target_value(&ev);
                                                        let cfg = serde_json::json!({
                                                            "integration_id": current_int_id_for_mode.clone(),
                                                            "mode": mode
                                                        }).to_string();
                                                        update_node_config(node_id_tool.clone(), cfg);
                                                    }>
                                                        <option value="read" selected={current_mode_for_select == "read"}>"Read Only"</option>
                                                        <option value="read_write" selected={current_mode_for_select == "read_write"}>"Read + Write"</option>
                                                    </select>
                                                </div>
                                                <p class="help">"Read+Write mode allows actions like sending email or updating workflow memory."</p>
                                            </div>
                                        }.into_any()
                                    },
                                    "data" => {
                                        let current_source = serde_json::from_str::<serde_json::Value>(&config)
                                            .ok()
                                            .and_then(|v: serde_json::Value| v.get("source").and_then(|s| s.as_str()).map(|s| s.to_string()))
                                            .unwrap_or_else(|| "__workflow_memory__".to_string());
                                        view! {
                                            <div class="type-config">
                                                <div class="form-group">
                                                    <label>"Data Source"</label>
                                                    <Suspense fallback=move || view! { <select disabled=true><option>"Loading..."</option></select> }>
                                                        {
                                                            let node_id_for_data = node_id_data.clone();
                                                            move || {
                                                                let integrations = available_integrations.get().unwrap_or_default();
                                                                let current_id = current_source.clone();
                                                                let nid = node_id_for_data.clone();
                                                                view! {
                                                                    <select on:change=move |ev| {
                                                                        let source = event_target_value(&ev);
                                                                        let cfg = serde_json::json!({"source": source}).to_string();
                                                                        update_node_config(nid.clone(), cfg);
                                                                    }>
                                                                        <option value="" selected={current_id.is_empty()}>"-- Select --"</option>
                                                                        // Built-in: Workflow Memory
                                                                        <option value="__workflow_memory__" selected={current_id == "__workflow_memory__"}>"Workflow Memory"</option>
                                                                        // User integrations
                                                                        {integrations.into_iter().map(|int| {
                                                                            let id = int.id.clone();
                                                                            let is_selected = id == current_id;
                                                                            let type_label = match int.integration_type.as_str() {
                                                                                "imap" => "Email",
                                                                                "gmail" => "Gmail",
                                                                                "calendar_feed" => "Calendar",
                                                                                other => other
                                                                            };
                                                                            view! {
                                                                                <option value=id selected=is_selected>
                                                                                    {format!("{} ({})", int.name, type_label)}
                                                                                </option>
                                                                            }
                                                                        }).collect_view()}
                                                                    </select>
                                                                }
                                                            }
                                                        }
                                                    </Suspense>
                                                </div>
                                                <p class="help">"Injects entire data source content into connected AI nodes."</p>
                                            </div>
                                        }.into_any()
                                    },
                                    "model" => {
                                        let current_integration = serde_json::from_str::<serde_json::Value>(&config_for_model)
                                            .ok()
                                            .and_then(|v: serde_json::Value| v.get("integration_id").and_then(|i| i.as_str()).map(|s| s.to_string()))
                                            .unwrap_or_default();
                                        let current_model = serde_json::from_str::<serde_json::Value>(&config_for_model)
                                            .ok()
                                            .and_then(|v: serde_json::Value| v.get("model_id").and_then(|m| m.as_str()).map(|s| s.to_string()))
                                            .unwrap_or_default();
                                        // Signal to track selected integration for model discovery
                                        let (selected_integration, set_selected_integration) = signal(current_integration.clone());
                                        // Signal to trigger retry (increment to refetch)
                                        let (retry_count, set_retry_count) = signal(0u32);
                                        // Resource to discover models when integration changes (or retry triggered)
                                        let discovered_models: Resource<Result<Vec<ModelInfo>, String>> = Resource::new(
                                            move || (selected_integration.get(), retry_count.get()),
                                            |(int_id, _retry)| async move {
                                                if int_id.is_empty() {
                                                    return Ok(Vec::new());
                                                }
                                                discover_models(int_id)
                                                    .await
                                                    .map_err(|e| e.to_string())
                                            }
                                        );
                                        let current_model_for_select = current_model.clone();
                                        let node_id_model_for_select = node_id_model.clone();
                                        view! {
                                            <div class="type-config">
                                                <div class="form-group">
                                                    <label>"LLM Provider"</label>
                                                    <Suspense fallback=move || view! { <select disabled=true><option>"Loading..."</option></select> }>
                                                        {
                                                            let nid = node_id_model.clone();
                                                            let curr_model = current_model.clone();
                                                            move || {
                                                                let integrations = available_integrations.get().unwrap_or_default();
                                                                // Filter to only openai_compatible integrations
                                                                let openai_integrations: Vec<_> = integrations.into_iter()
                                                                    .filter(|i| i.integration_type == "openai_compatible")
                                                                    .collect();
                                                                let current_id = current_integration.clone();
                                                                let model_val = curr_model.clone();
                                                                let node_id_for_int = nid.clone();
                                                                view! {
                                                                    <select on:change=move |ev| {
                                                                        let int_id = event_target_value(&ev);
                                                                        // Update signal to trigger model discovery
                                                                        set_selected_integration.set(int_id.clone());
                                                                        let cfg = serde_json::json!({
                                                                            "integration_id": int_id,
                                                                            "model_id": model_val.clone()
                                                                        }).to_string();
                                                                        update_node_config(node_id_for_int.clone(), cfg);
                                                                    }>
                                                                        <option value="" selected={current_id.is_empty()}>"-- Select Provider --"</option>
                                                                        {openai_integrations.into_iter().map(|int| {
                                                                            let id = int.id.clone();
                                                                            let is_selected = id == current_id;
                                                                            view! {
                                                                                <option value=id selected=is_selected>
                                                                                    {int.name}
                                                                                </option>
                                                                            }
                                                                        }).collect_view()}
                                                                    </select>
                                                                }
                                                            }
                                                        }
                                                    </Suspense>
                                                </div>
                                                <div class="form-group">
                                                    <label>"Model"</label>
                                                    <Suspense fallback=move || view! { <select disabled=true><option>"Loading models..."</option></select> }>
                                                        {
                                                            let nid = node_id_model_for_select.clone();
                                                            let curr_model = current_model_for_select.clone();
                                                            move || {
                                                                let result = discovered_models.get();
                                                                let current_model_id = curr_model.clone();
                                                                let node_id_for_model = nid.clone();

                                                                // No integration selected yet
                                                                if selected_integration.get().is_empty() {
                                                                    return view! {
                                                                        <select disabled=true>
                                                                            <option>"-- Select a provider first --"</option>
                                                                        </select>
                                                                    }.into_any();
                                                                }

                                                                // Handle the result
                                                                match result {
                                                                    None => {
                                                                        // Still loading
                                                                        view! {
                                                                            <select disabled=true>
                                                                                <option>"Discovering models..."</option>
                                                                            </select>
                                                                        }.into_any()
                                                                    }
                                                                    Some(Err(error)) => {
                                                                        // Error occurred - show error with retry button
                                                                        view! {
                                                                            <div class="model-discovery-error">
                                                                                <select disabled=true>
                                                                                    <option>"Discovery failed"</option>
                                                                                </select>
                                                                                <p class="error-message">{error}</p>
                                                                                <button
                                                                                    type="button"
                                                                                    class="retry-btn"
                                                                                    on:click=move |_| {
                                                                                        set_retry_count.update(|c| *c += 1);
                                                                                    }
                                                                                >
                                                                                    "Retry"
                                                                                </button>
                                                                            </div>
                                                                        }.into_any()
                                                                    }
                                                                    Some(Ok(models)) if models.is_empty() => {
                                                                        // No models found
                                                                        view! {
                                                                            <div class="model-discovery-empty">
                                                                                <select disabled=true>
                                                                                    <option>"No models found"</option>
                                                                                </select>
                                                                                <button
                                                                                    type="button"
                                                                                    class="retry-btn"
                                                                                    on:click=move |_| {
                                                                                        set_retry_count.update(|c| *c += 1);
                                                                                    }
                                                                                >
                                                                                    "Refresh"
                                                                                </button>
                                                                            </div>
                                                                        }.into_any()
                                                                    }
                                                                    Some(Ok(models)) => {
                                                                        // Show discovered models
                                                                        view! {
                                                                            <select on:change=move |ev| {
                                                                                let model_id = event_target_value(&ev);
                                                                                let cfg = serde_json::json!({
                                                                                    "integration_id": selected_integration.get(),
                                                                                    "model_id": model_id
                                                                                }).to_string();
                                                                                update_node_config(node_id_for_model.clone(), cfg);
                                                                            }>
                                                                                <option value="" selected={current_model_id.is_empty()}>"-- Select Model --"</option>
                                                                                {models.into_iter().map(|model| {
                                                                                    let id = model.id.clone();
                                                                                    let is_selected = id == current_model_id;
                                                                                    view! {
                                                                                        <option value=id.clone() selected=is_selected>
                                                                                            {model.name}
                                                                                        </option>
                                                                                    }
                                                                                }).collect_view()}
                                                                            </select>
                                                                        }.into_any()
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    </Suspense>
                                                </div>
                                                <p class="help">"Connect this model node's output to AI nodes to specify which model they should use."</p>
                                            </div>
                                        }.into_any()
                                    },
                                    _ => view! { <p>"Unknown node type"</p> }.into_any()
                                }}
                            </div>
                        }.into_any()
                    },
                    None => view! {
                        <div class="no-selection">
                            <p>"Select a node to configure"</p>
                            <p class="hint">"Click on a node in the canvas to view and edit its settings."</p>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
