//! Integrations page component and server functions.

mod server;

pub use server::{
    IntegrationConfigData, IntegrationInfo, ModelInfo, create_integration, delete_integration,
    discover_models, get_integration_config, list_integrations, list_openai_integrations,
    test_openai_connection, update_integration_config, update_integration_name,
};

use crate::user::get_current_user;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Integrations page.
#[component]
pub fn IntegrationsPage() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());
    let integrations = Resource::new(|| (), |_| list_integrations());

    // State for create form
    let (show_create, set_show_create) = signal(false);
    let (create_type, set_create_type) = signal(String::new());
    let (create_type_filter, set_create_type_filter) = signal(String::new());
    let (create_name, set_create_name) = signal(String::new());
    let (create_server, set_create_server) = signal(String::new());
    let (create_port, set_create_port) = signal("993".to_string());
    let (create_username, set_create_username) = signal(String::new());
    let (create_password, set_create_password) = signal(String::new());
    let (create_url, set_create_url) = signal(String::new());
    let (create_endpoint_url, set_create_endpoint_url) = signal(String::new());
    let (create_api_key, set_create_api_key) = signal(String::new());
    let (testing_connection, set_testing_connection) = signal(false);
    let (connection_test_result, set_connection_test_result) =
        signal(Option::<Result<(), String>>::None);
    let (creating, set_creating) = signal(false);
    let (create_error, set_create_error) = signal(Option::<String>::None);

    // State for edit modal
    let (editing_id, set_editing_id) = signal(Option::<String>::None);
    let (edit_name, set_edit_name) = signal(String::new());
    let (edit_type, set_edit_type) = signal(String::new());
    let (edit_server, set_edit_server) = signal(String::new());
    let (edit_port, set_edit_port) = signal("993".to_string());
    let (edit_username, set_edit_username) = signal(String::new());
    let (edit_password, set_edit_password) = signal(String::new());
    let (edit_url, set_edit_url) = signal(String::new());
    let (edit_endpoint_url, set_edit_endpoint_url) = signal(String::new());
    let (edit_api_key, set_edit_api_key) = signal(String::new());
    let (edit_testing_connection, set_edit_testing_connection) = signal(false);
    let (edit_connection_test_result, set_edit_connection_test_result) =
        signal(Option::<Result<(), String>>::None);
    let (saving, set_saving) = signal(false);
    let (loading_config, set_loading_config) = signal(false);

    // State for delete confirmation
    let (delete_id, set_delete_id) = signal(Option::<String>::None);
    let (delete_warning, set_delete_warning) = signal(Option::<Vec<String>>::None);
    let (deleting, set_deleting) = signal(false);

    let on_create = move |_| {
        let int_type = create_type.get();
        let name = create_name.get();
        if name.is_empty() || int_type.is_empty() {
            set_create_error.set(Some("Name and type are required".to_string()));
            return;
        }

        let config = match int_type.as_str() {
            "imap" => {
                serde_json::json!({
                    "server": create_server.get(),
                    "port": create_port.get().parse::<u16>().unwrap_or(993),
                    "username": create_username.get(),
                    "password": create_password.get(),
                    "use_tls": true
                })
            }
            "gmail" => serde_json::json!({"oauth_pending": true}),
            "calendar_feed" => serde_json::json!({"url": create_url.get()}),
            "openai_compatible" => {
                let mut config = serde_json::json!({
                    "endpoint_url": create_endpoint_url.get()
                });
                let api_key = create_api_key.get();
                if !api_key.is_empty() {
                    config["api_key"] = serde_json::json!(api_key);
                }
                config
            }
            _ => serde_json::json!({}),
        };

        set_creating.set(true);
        set_create_error.set(None);
        let config_str = config.to_string();
        let is_gmail = int_type == "gmail";
        spawn_local(async move {
            match create_integration(name, int_type, config_str).await {
                Ok(integration_id) => {
                    set_show_create.set(false);
                    set_create_type.set(String::new());
                    set_create_type_filter.set(String::new());
                    set_create_name.set(String::new());
                    set_create_server.set(String::new());
                    set_create_port.set("993".to_string());
                    set_create_username.set(String::new());
                    set_create_password.set(String::new());
                    set_create_url.set(String::new());
                    set_create_endpoint_url.set(String::new());
                    set_create_api_key.set(String::new());
                    set_connection_test_result.set(None);

                    // For Gmail, redirect to OAuth flow
                    if is_gmail {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let oauth_url =
                                format!("/auth/gmail/start?integration_id={}", integration_id);
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href(&oauth_url);
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            // SSR path - refetch since we can't redirect
                            let _ = integration_id; // suppress warning
                            integrations.refetch();
                        }
                    } else {
                        integrations.refetch();
                    }
                }
                Err(e) => {
                    set_create_error.set(Some(format!("{}", e)));
                }
            }
            set_creating.set(false);
        });
    };

    let on_save_integration = move |_| {
        let id = match editing_id.get() {
            Some(id) => id,
            None => return,
        };
        let name = edit_name.get();
        if name.is_empty() {
            return;
        }
        let int_type = edit_type.get();

        // Build config based on integration type
        let config = match int_type.as_str() {
            "imap" => {
                serde_json::json!({
                    "server": edit_server.get(),
                    "port": edit_port.get().parse::<u16>().unwrap_or(993),
                    "username": edit_username.get(),
                    "password": edit_password.get(),
                    "use_tls": true
                })
            }
            "calendar_feed" => serde_json::json!({"url": edit_url.get()}),
            "openai_compatible" => {
                let mut config = serde_json::json!({
                    "endpoint_url": edit_endpoint_url.get()
                });
                let api_key = edit_api_key.get();
                if !api_key.is_empty() {
                    config["api_key"] = serde_json::json!(api_key);
                }
                config
            }
            _ => serde_json::json!({}),
        };
        let config_str = config.to_string();

        set_saving.set(true);
        spawn_local(async move {
            // Update name first
            if update_integration_name(id.clone(), name).await.is_err() {
                set_saving.set(false);
                return;
            }
            // Then update config
            if update_integration_config(id, config_str).await.is_ok() {
                set_editing_id.set(None);
                integrations.refetch();
            }
            set_saving.set(false);
        });
    };

    let on_delete = move |_| {
        let id = match delete_id.get() {
            Some(id) => id,
            None => return,
        };
        set_deleting.set(true);
        spawn_local(async move {
            match delete_integration(id.clone()).await {
                Ok(Some(workflows)) => {
                    set_delete_warning.set(Some(workflows));
                    set_deleting.set(false);
                }
                Ok(None) => {
                    set_delete_id.set(None);
                    set_delete_warning.set(None);
                    integrations.refetch();
                    set_deleting.set(false);
                }
                Err(_) => {
                    set_deleting.set(false);
                }
            }
        });
    };

    let on_force_delete = move |_| {
        let id = match delete_id.get() {
            Some(id) => id,
            None => return,
        };
        set_deleting.set(true);
        spawn_local(async move {
            let _ = delete_integration(id).await;
            set_delete_id.set(None);
            set_delete_warning.set(None);
            integrations.refetch();
            set_deleting.set(false);
        });
    };

    view! {
        <div class="integrations-page">
            <h1>"Integrations"</h1>
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || {
                    user.get().map(|result| {
                        match result {
                            Ok(Some(_user_info)) => view! {
                                <div class="integrations-content">
                                    <p>"Connect external services to your assistant."</p>

                                    <div class="actions-bar">
                                        <button
                                            class="primary-btn"
                                            on:click=move |_| set_show_create.set(true)
                                        >"Add Integration"</button>
                                    </div>

                                    // Create Integration Modal
                                    {move || show_create.get().then(|| view! {
                                        <div class="modal-overlay">
                                            <div class="modal">
                                                <h2>"Add Integration"</h2>

                                                <div class="form-group">
                                                    <label>"Integration Type"</label>
                                                    <input
                                                        type="text"
                                                        class="type-filter"
                                                        placeholder="Search integration types..."
                                                        prop:value=move || create_type_filter.get()
                                                        on:input=move |ev| set_create_type_filter.set(event_target_value(&ev))
                                                    />
                                                    <div class="type-picker">
                                                        {move || {
                                                            let filter = create_type_filter.get().to_lowercase();
                                                            let types = vec![
                                                                ("openai_compatible", "LLM Provider (OpenAI-compatible)", "Connect to Ollama, OpenAI, or any OpenAI-compatible API"),
                                                                ("imap", "Email (IMAP)", "Connect via IMAP to read and send email"),
                                                                ("gmail", "Gmail (OAuth)", "Connect Gmail with secure OAuth authentication"),
                                                                ("calendar_feed", "Calendar Feed", "Subscribe to iCal/CalDAV calendar feeds"),
                                                            ];
                                                            let selected = create_type.get();
                                                            types.into_iter()
                                                                .filter(|(_, label, desc)| {
                                                                    filter.is_empty() ||
                                                                    label.to_lowercase().contains(&filter) ||
                                                                    desc.to_lowercase().contains(&filter)
                                                                })
                                                                .map(|(value, label, desc)| {
                                                                    let is_selected = selected == value;
                                                                    let value_for_click = value.to_string();
                                                                    view! {
                                                                        <div
                                                                            class="type-option"
                                                                            class:selected=is_selected
                                                                            on:click=move |_| {
                                                                                set_create_type.set(value_for_click.clone());
                                                                            }
                                                                        >
                                                                            <strong>{label}</strong>
                                                                            <span class="type-desc">{desc}</span>
                                                                        </div>
                                                                    }
                                                                })
                                                                .collect_view()
                                                        }}
                                                    </div>
                                                </div>

                                                <div class="form-group">
                                                    <label>"Name"</label>
                                                    <input
                                                        type="text"
                                                        placeholder="e.g., Work Email, Personal Calendar"
                                                        prop:value=move || create_name.get()
                                                        on:input=move |ev| set_create_name.set(event_target_value(&ev))
                                                    />
                                                </div>

                                                {move || (create_type.get() == "imap").then(|| view! {
                                                    <div class="type-fields">
                                                        <div class="form-group">
                                                            <label>"Server"</label>
                                                            <input
                                                                type="text"
                                                                placeholder="imap.example.com"
                                                                prop:value=move || create_server.get()
                                                                on:input=move |ev| set_create_server.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                        <div class="form-group">
                                                            <label>"Port"</label>
                                                            <input
                                                                type="number"
                                                                prop:value=move || create_port.get()
                                                                on:input=move |ev| set_create_port.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                        <div class="form-group">
                                                            <label>"Username"</label>
                                                            <input
                                                                type="text"
                                                                prop:value=move || create_username.get()
                                                                on:input=move |ev| set_create_username.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                        <div class="form-group">
                                                            <label>"Password"</label>
                                                            <input
                                                                type="password"
                                                                prop:value=move || create_password.get()
                                                                on:input=move |ev| set_create_password.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                    </div>
                                                })}

                                                {move || (create_type.get() == "gmail").then(|| view! {
                                                    <div class="type-fields">
                                                        <p class="info">"After creating, you'll be redirected to Google to authorize access."</p>
                                                    </div>
                                                })}

                                                {move || (create_type.get() == "calendar_feed").then(|| view! {
                                                    <div class="type-fields">
                                                        <div class="form-group">
                                                            <label>"Calendar URL"</label>
                                                            <input
                                                                type="url"
                                                                placeholder="https://calendar.example.com/feed.ics"
                                                                prop:value=move || create_url.get()
                                                                on:input=move |ev| set_create_url.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                    </div>
                                                })}

                                                {move || (create_type.get() == "openai_compatible").then(|| {
                                                    let on_test_connection = move |_| {
                                                        let endpoint = create_endpoint_url.get();
                                                        let api_key = create_api_key.get();
                                                        if endpoint.is_empty() {
                                                            set_connection_test_result.set(Some(Err("Endpoint URL is required".to_string())));
                                                            return;
                                                        }
                                                        set_testing_connection.set(true);
                                                        set_connection_test_result.set(None);
                                                        let api_key_opt = if api_key.is_empty() { None } else { Some(api_key) };
                                                        spawn_local(async move {
                                                            match test_openai_connection(endpoint, api_key_opt).await {
                                                                Ok(_) => {
                                                                    set_connection_test_result.set(Some(Ok(())));
                                                                }
                                                                Err(e) => {
                                                                    set_connection_test_result.set(Some(Err(e.to_string())));
                                                                }
                                                            }
                                                            set_testing_connection.set(false);
                                                        });
                                                    };
                                                    view! {
                                                        <div class="type-fields">
                                                            <div class="form-group">
                                                                <label>"Endpoint URL" <span class="required">"*"</span></label>
                                                                <input
                                                                    type="url"
                                                                    placeholder="http://localhost:11434 (Ollama) or https://api.openai.com"
                                                                    prop:value=move || create_endpoint_url.get()
                                                                    on:input=move |ev| {
                                                                        set_create_endpoint_url.set(event_target_value(&ev));
                                                                        set_connection_test_result.set(None);
                                                                    }
                                                                />
                                                                <p class="help">"Base URL for the OpenAI-compatible API"</p>
                                                            </div>
                                                            <div class="form-group">
                                                                <label>"API Key" <span class="optional">"(optional)"</span></label>
                                                                <input
                                                                    type="password"
                                                                    placeholder="sk-... or leave empty for local providers"
                                                                    prop:value=move || create_api_key.get()
                                                                    on:input=move |ev| {
                                                                        set_create_api_key.set(event_target_value(&ev));
                                                                        set_connection_test_result.set(None);
                                                                    }
                                                                />
                                                                <p class="help">"Required for cloud providers like OpenAI, optional for local providers like Ollama"</p>
                                                            </div>
                                                            <div class="form-group">
                                                                <button
                                                                    class="secondary-btn"
                                                                    type="button"
                                                                    on:click=on_test_connection
                                                                    disabled=move || testing_connection.get() || create_endpoint_url.get().is_empty()
                                                                >
                                                                    {move || if testing_connection.get() { "Testing..." } else { "Test Connection" }}
                                                                </button>
                                                                {move || connection_test_result.get().map(|result| {
                                                                    match result {
                                                                        Ok(()) => view! { <span class="success">" Connected successfully!"</span> }.into_any(),
                                                                        Err(e) => view! { <span class="error">{format!(" {}", e)}</span> }.into_any()
                                                                    }
                                                                })}
                                                            </div>
                                                        </div>
                                                    }
                                                })}

                                                {move || create_error.get().map(|e| view! {
                                                    <p class="error">{e}</p>
                                                })}

                                                <div class="modal-actions">
                                                    <button
                                                        class="secondary-btn"
                                                        on:click=move |_| {
                                                            set_show_create.set(false);
                                                            set_create_type_filter.set(String::new());
                                                        }
                                                    >"Cancel"</button>
                                                    <button
                                                        class="primary-btn"
                                                        on:click=on_create
                                                        disabled=move || creating.get()
                                                    >
                                                        {move || if creating.get() { "Creating..." } else { "Create" }}
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    })}

                                    // Edit Name Modal
                                    {move || editing_id.get().map(|_| view! {
                                        <div class="modal-overlay">
                                            <div class="modal">
                                                <h2>"Edit Integration"</h2>
                                                {move || if loading_config.get() {
                                                    view! { <p>"Loading configuration..."</p> }.into_any()
                                                } else {
                                                    view! {
                                                        <div>
                                                            <div class="form-group">
                                                                <label>"Name"</label>
                                                                <input
                                                                    type="text"
                                                                    prop:value=move || edit_name.get()
                                                                    on:input=move |ev| set_edit_name.set(event_target_value(&ev))
                                                                />
                                                            </div>

                                                            // IMAP-specific fields
                                                            {move || if edit_type.get() == "imap" {
                                                                view! {
                                                                    <div>
                                                                        <div class="form-group">
                                                                            <label>"IMAP Server"</label>
                                                                            <input
                                                                                type="text"
                                                                                placeholder="imap.example.com"
                                                                                prop:value=move || edit_server.get()
                                                                                on:input=move |ev| set_edit_server.set(event_target_value(&ev))
                                                                            />
                                                                        </div>
                                                                        <div class="form-group">
                                                                            <label>"Port"</label>
                                                                            <input
                                                                                type="text"
                                                                                placeholder="993"
                                                                                prop:value=move || edit_port.get()
                                                                                on:input=move |ev| set_edit_port.set(event_target_value(&ev))
                                                                            />
                                                                        </div>
                                                                        <div class="form-group">
                                                                            <label>"Username"</label>
                                                                            <input
                                                                                type="text"
                                                                                placeholder="user@example.com"
                                                                                prop:value=move || edit_username.get()
                                                                                on:input=move |ev| set_edit_username.set(event_target_value(&ev))
                                                                            />
                                                                        </div>
                                                                        <div class="form-group">
                                                                            <label>"Password"</label>
                                                                            <input
                                                                                type="password"
                                                                                placeholder="Leave blank to keep existing"
                                                                                prop:value=move || edit_password.get()
                                                                                on:input=move |ev| set_edit_password.set(event_target_value(&ev))
                                                                            />
                                                                        </div>
                                                                    </div>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span></span> }.into_any()
                                                            }}

                                                            // Calendar Feed fields
                                                            {move || if edit_type.get() == "calendar_feed" {
                                                                view! {
                                                                    <div class="form-group">
                                                                        <label>"Calendar URL"</label>
                                                                        <input
                                                                            type="url"
                                                                            placeholder="https://calendar.example.com/feed.ics"
                                                                            prop:value=move || edit_url.get()
                                                                            on:input=move |ev| set_edit_url.set(event_target_value(&ev))
                                                                        />
                                                                    </div>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span></span> }.into_any()
                                                            }}

                                                            // Gmail re-authenticate
                                                            {move || if edit_type.get() == "gmail" {
                                                                #[cfg(target_arch = "wasm32")]
                                                                let id_for_reauth = editing_id.get();
                                                                view! {
                                                                    <div class="gmail-reauth">
                                                                        <p class="info">"Gmail uses OAuth for authentication."</p>
                                                                        <button
                                                                            type="button"
                                                                            class="reauth-btn"
                                                                            on:click=move |_| {
                                                                                #[cfg(target_arch = "wasm32")]
                                                                                if let Some(id) = id_for_reauth.clone() {
                                                                                    let oauth_url = format!("/auth/gmail/start?integration_id={}", id);
                                                                                    if let Some(window) = web_sys::window() {
                                                                                        let _ = window.location().set_href(&oauth_url);
                                                                                    }
                                                                                }
                                                                            }
                                                                        >"Re-authenticate with Google"</button>
                                                                    </div>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span></span> }.into_any()
                                                            }}

                                                            // OpenAI-compatible fields
                                                            {move || if edit_type.get() == "openai_compatible" {
                                                                let on_test_edit_connection = move |_| {
                                                                    let endpoint = edit_endpoint_url.get();
                                                                    let api_key = edit_api_key.get();
                                                                    if endpoint.is_empty() {
                                                                        set_edit_connection_test_result.set(Some(Err("Endpoint URL is required".to_string())));
                                                                        return;
                                                                    }
                                                                    set_edit_testing_connection.set(true);
                                                                    set_edit_connection_test_result.set(None);
                                                                    let api_key_opt = if api_key.is_empty() { None } else { Some(api_key) };
                                                                    spawn_local(async move {
                                                                        match test_openai_connection(endpoint, api_key_opt).await {
                                                                            Ok(_) => {
                                                                                set_edit_connection_test_result.set(Some(Ok(())));
                                                                            }
                                                                            Err(e) => {
                                                                                set_edit_connection_test_result.set(Some(Err(e.to_string())));
                                                                            }
                                                                        }
                                                                        set_edit_testing_connection.set(false);
                                                                    });
                                                                };
                                                                view! {
                                                                    <div>
                                                                        <div class="form-group">
                                                                            <label>"Endpoint URL"</label>
                                                                            <input
                                                                                type="url"
                                                                                placeholder="http://localhost:11434"
                                                                                prop:value=move || edit_endpoint_url.get()
                                                                                on:input=move |ev| {
                                                                                    set_edit_endpoint_url.set(event_target_value(&ev));
                                                                                    set_edit_connection_test_result.set(None);
                                                                                }
                                                                            />
                                                                            <p class="help">"Base URL for the OpenAI-compatible API"</p>
                                                                        </div>
                                                                        <div class="form-group">
                                                                            <label>"API Key"</label>
                                                                            <input
                                                                                type="password"
                                                                                placeholder="Leave blank to keep existing"
                                                                                prop:value=move || edit_api_key.get()
                                                                                on:input=move |ev| {
                                                                                    set_edit_api_key.set(event_target_value(&ev));
                                                                                    set_edit_connection_test_result.set(None);
                                                                                }
                                                                            />
                                                                            <p class="help">"Optional - required for cloud providers, leave empty for local providers"</p>
                                                                        </div>
                                                                        <div class="form-group">
                                                                            <button
                                                                                class="secondary-btn"
                                                                                type="button"
                                                                                on:click=on_test_edit_connection
                                                                                disabled=move || edit_testing_connection.get() || edit_endpoint_url.get().is_empty()
                                                                            >
                                                                                {move || if edit_testing_connection.get() { "Testing..." } else { "Test Connection" }}
                                                                            </button>
                                                                            {move || edit_connection_test_result.get().map(|result| {
                                                                                match result {
                                                                                    Ok(()) => view! { <span class="success">" Connected successfully!"</span> }.into_any(),
                                                                                    Err(e) => view! { <span class="error">{format!(" {}", e)}</span> }.into_any()
                                                                                }
                                                                            })}
                                                                        </div>
                                                                    </div>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span></span> }.into_any()
                                                            }}
                                                        </div>
                                                    }.into_any()
                                                }}
                                                <div class="modal-actions">
                                                    <button
                                                        class="secondary-btn"
                                                        on:click=move |_| set_editing_id.set(None)
                                                    >"Cancel"</button>
                                                    <button
                                                        class="primary-btn"
                                                        on:click=on_save_integration
                                                        disabled=move || saving.get() || loading_config.get()
                                                    >
                                                        {move || if saving.get() { "Saving..." } else { "Save" }}
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    })}

                                    // Delete Confirmation Modal
                                    {move || delete_id.get().map(|_| view! {
                                        <div class="modal-overlay">
                                            <div class="modal">
                                                <h2>"Delete Integration?"</h2>
                                                {move || match delete_warning.get() {
                                                    Some(workflows) => view! {
                                                        <div class="warning">
                                                            <p><strong>"Warning:"</strong>" This integration is used by the following workflows:"</p>
                                                            <ul>
                                                                {workflows.into_iter().map(|w| view! { <li>{w}</li> }).collect_view()}
                                                            </ul>
                                                            <p>"Deleting it may break these workflows."</p>
                                                        </div>
                                                    }.into_any(),
                                                    None => view! {
                                                        <p>"Are you sure you want to delete this integration?"</p>
                                                    }.into_any()
                                                }}
                                                <div class="modal-actions">
                                                    <button
                                                        class="secondary-btn"
                                                        on:click=move |_| {
                                                            set_delete_id.set(None);
                                                            set_delete_warning.set(None);
                                                        }
                                                    >"Cancel"</button>
                                                    {move || if delete_warning.get().is_some() {
                                                        view! {
                                                            <button
                                                                class="danger-btn"
                                                                on:click=on_force_delete
                                                                disabled=move || deleting.get()
                                                            >
                                                                {move || if deleting.get() { "Deleting..." } else { "Delete Anyway" }}
                                                            </button>
                                                        }.into_any()
                                                    } else {
                                                        view! {
                                                            <button
                                                                class="danger-btn"
                                                                on:click=on_delete
                                                                disabled=move || deleting.get()
                                                            >
                                                                {move || if deleting.get() { "Deleting..." } else { "Delete" }}
                                                            </button>
                                                        }.into_any()
                                                    }}
                                                </div>
                                            </div>
                                        </div>
                                    })}

                                    // Integration List
                                    <section class="integrations-list">
                                        <h2>"Connected Services"</h2>
                                        <Suspense fallback=move || view! { <p>"Loading integrations..."</p> }>
                                            {move || {
                                                integrations.get().map(|result| {
                                                    match result {
                                                        Ok(items) if items.is_empty() => view! {
                                                            <p class="empty-state">"No integrations configured yet. Click \"Add Integration\" to get started."</p>
                                                        }.into_any(),
                                                        Ok(items) => view! {
                                                            <table class="integrations-table">
                                                                <thead>
                                                                    <tr>
                                                                        <th>"Name"</th>
                                                                        <th>"Type"</th>
                                                                        <th>"Status"</th>
                                                                        <th>"Actions"</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody>
                                                                    {items.into_iter().map(|item| {
                                                                        let item_id = item.id.clone();
                                                                        let item_id2 = item.id.clone();
                                                                        let item_id_for_edit = item.id.clone();
                                                                        let item_name = item.name.clone();
                                                                        let item_type = item.integration_type.clone();
                                                                        let display_name = item.name.clone();
                                                                        let type_display = match item.integration_type.as_str() {
                                                                            "imap" => "Email (IMAP)".to_string(),
                                                                            "gmail" => "Gmail".to_string(),
                                                                            "calendar_feed" => "Calendar Feed".to_string(),
                                                                            "openai_compatible" => "LLM Provider".to_string(),
                                                                            other => other.to_string()
                                                                        };
                                                                        let status_class = format!("status-{}", item.status);
                                                                        let status_display = match item.status.as_str() {
                                                                            "connected" => "Connected".to_string(),
                                                                            "error" => "Error".to_string(),
                                                                            "pending" => "Pending".to_string(),
                                                                            other => other.to_string()
                                                                        };
                                                                        let error_msg = item.error_message.clone();
                                                                        view! {
                                                                            <tr>
                                                                                <td>{display_name}</td>
                                                                                <td>{type_display}</td>
                                                                                <td class=status_class>
                                                                                    {status_display}
                                                                                    {error_msg.map(|e| view! { <span class="error-hint" title=e>" ⚠"</span> })}
                                                                                </td>
                                                                                <td class="actions">
                                                                                    <button
                                                                                        class="edit-btn"
                                                                                        on:click=move |_| {
                                                                                            set_edit_name.set(item_name.clone());
                                                                                            set_edit_type.set(item_type.clone());
                                                                                            set_editing_id.set(Some(item_id.clone()));
                                                                                            set_edit_connection_test_result.set(None); // Reset test result
                                                                                            // Load current config
                                                                                            let id = item_id_for_edit.clone();
                                                                                            set_loading_config.set(true);
                                                                                            spawn_local(async move {
                                                                                                if let Ok(config) = get_integration_config(id).await {
                                                                                                    set_edit_server.set(config.server.unwrap_or_default());
                                                                                                    set_edit_port.set(config.port.map(|p| p.to_string()).unwrap_or_else(|| "993".to_string()));
                                                                                                    set_edit_username.set(config.username.unwrap_or_default());
                                                                                                    set_edit_password.set(String::new()); // Don't show existing password
                                                                                                    set_edit_url.set(config.url.unwrap_or_default());
                                                                                                    set_edit_endpoint_url.set(config.endpoint_url.unwrap_or_default());
                                                                                                    set_edit_api_key.set(String::new()); // Don't show existing API key
                                                                                                }
                                                                                                set_loading_config.set(false);
                                                                                            });
                                                                                        }
                                                                                    >"Edit"</button>
                                                                                    <button
                                                                                        class="delete-btn"
                                                                                        on:click=move |_| {
                                                                                            set_delete_id.set(Some(item_id2.clone()));
                                                                                            set_delete_warning.set(None);
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
                                                            <p class="error">"Failed to load integrations."</p>
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
                                    <p>"Please log in to manage integrations."</p>
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
