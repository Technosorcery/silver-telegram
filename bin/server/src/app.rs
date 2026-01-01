//! Main Leptos application component and routing.

use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::{Title, provide_meta_context};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

/// User info for display in the UI.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserInfo {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub timezone: Option<String>,
    pub is_admin: bool,
}

/// Server function to get the current user info.
#[server]
pub async fn get_current_user() -> Result<Option<UserInfo>, ServerFnError> {
    use crate::auth::db::{SessionRepository, UserRepository};
    use axum::Extension;
    use axum_extra::extract::CookieJar;
    use silver_telegram_platform_access::SessionId;
    use sqlx::PgPool;

    const SESSION_COOKIE: &str = "session";

    // Extract the cookie jar from the request
    let jar: CookieJar = leptos_axum::extract().await?;

    // Get database pool from request extension
    let Extension(db_pool): Extension<PgPool> = leptos_axum::extract().await?;

    // Get session ID from cookie
    let session_cookie = match jar.get(SESSION_COOKIE) {
        Some(cookie) => cookie,
        None => return Ok(None),
    };

    let session_id = SessionId::new(session_cookie.value().to_string());

    // Look up session in database
    let session_repo = SessionRepository::new(db_pool.clone());
    let session = match session_repo.find_by_id(&session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => return Ok(None),
        Err(_) => return Ok(None),
    };

    // Check if session is expired or has no access
    if session.is_expired() || !session.has_access() {
        return Ok(None);
    }

    // Load user from database
    let user_repo = UserRepository::new(db_pool);
    let user = match user_repo.find_by_id(session.user_id()).await {
        Ok(Some(user)) => user,
        Ok(None) => return Ok(None),
        Err(_) => return Ok(None),
    };

    Ok(Some(UserInfo {
        display_name: user.display_name().map(|s| s.to_string()),
        email: user.email().map(|s| s.to_string()),
        timezone: user.timezone().map(|s| s.to_string()),
        is_admin: session.is_admin(),
    }))
}

/// Server function to save the user's timezone.
#[server]
pub async fn save_timezone(timezone: String) -> Result<(), ServerFnError> {
    use crate::auth::db::{SessionRepository, UserRepository};
    use axum::Extension;
    use axum_extra::extract::CookieJar;
    use silver_telegram_platform_access::SessionId;
    use sqlx::PgPool;

    const SESSION_COOKIE: &str = "session";

    // Extract the cookie jar from the request
    let jar: CookieJar = leptos_axum::extract().await?;

    // Get database pool from request extension
    let Extension(db_pool): Extension<PgPool> = leptos_axum::extract().await?;

    // Get session ID from cookie
    let session_cookie = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let session_id = SessionId::new(session_cookie.value().to_string());

    // Look up session in database
    let session_repo = SessionRepository::new(db_pool.clone());
    let session = session_repo
        .find_by_id(&session_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Session not found"))?;

    // Check if session is expired or has no access
    if session.is_expired() || !session.has_access() {
        return Err(ServerFnError::new("Session expired or access denied"));
    }

    // Load user and update timezone
    let user_repo = UserRepository::new(db_pool.clone());
    let mut user = user_repo
        .find_by_id(session.user_id())
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not found"))?;

    user.set_timezone(Some(timezone));
    user_repo
        .update(&user)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to save timezone: {}", e)))?;

    Ok(())
}

/// The main application component.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="silver-telegram"/>
        <Router>
            <Header/>
            <main class="container">
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/login") view=LoginPage/>
                    <Route path=path!("/settings") view=SettingsPage/>
                    <Route path=path!("/integrations") view=IntegrationsPage/>
                    <Route path=path!("/admin") view=AdminPage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Header component with navigation and user menu.
#[component]
fn Header() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());

    view! {
        <header class="header">
            <div class="header-left">
                <a href="/" class="logo">"silver-telegram"</a>
            </div>
            <div class="header-right">
                <Suspense fallback=move || view! { <span>"Loading..."</span> }>
                    {move || {
                        user.get().map(|result| {
                            match result {
                                Ok(Some(user_info)) => view! {
                                    <UserMenu user_info=user_info/>
                                }.into_any(),
                                Ok(None) => view! {
                                    <a href="/auth/login" rel="external" class="login-button">"Log in"</a>
                                }.into_any(),
                                Err(_) => view! {
                                    <a href="/auth/login" rel="external" class="login-button">"Log in"</a>
                                }.into_any(),
                            }
                        })
                    }}
                </Suspense>
            </div>
        </header>
    }
}

/// User menu dropdown component.
#[component]
fn UserMenu(user_info: UserInfo) -> impl IntoView {
    let display_name = user_info
        .display_name
        .clone()
        .or_else(|| user_info.email.clone())
        .unwrap_or_else(|| "User".to_string());

    view! {
        <div class="user-menu">
            <span class="user-name">{display_name}</span>
            <div class="user-dropdown">
                <a href="/settings">"Settings"</a>
                {if user_info.is_admin {
                    view! { <a href="/admin">"Admin"</a> }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                <a href="/auth/logout" rel="external">"Log out"</a>
            </div>
        </div>
    }
}

/// The home page component.
#[component]
fn HomePage() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());

    view! {
        <div class="home-page">
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || {
                    user.get().map(|result| {
                        match result {
                            Ok(Some(user_info)) => {
                                let greeting = user_info.display_name.clone()
                                    .map(|n| format!("Welcome, {}!", n))
                                    .unwrap_or_else(|| "Welcome!".to_string());
                                view! {
                                    <div>
                                        <h1>{greeting}</h1>
                                        <p>"Your autonomous personal assistant is ready."</p>
                                    </div>
                                }.into_any()
                            },
                            Ok(None) => view! {
                                <div>
                                    <h1>"silver-telegram"</h1>
                                    <p>"Autonomous Personal Assistant Platform"</p>
                                    <p>"Please log in to access your assistant."</p>
                                    <a href="/auth/login" rel="external" class="cta-button">"Log in"</a>
                                </div>
                            }.into_any(),
                            Err(_) => view! {
                                <div>
                                    <h1>"silver-telegram"</h1>
                                    <p>"Autonomous Personal Assistant Platform"</p>
                                    <a href="/auth/login" rel="external" class="cta-button">"Log in"</a>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

/// Login page - redirects to OIDC provider.
#[component]
fn LoginPage() -> impl IntoView {
    view! {
        <div class="login-page">
            <div class="login-box">
                <h1>"Log in to silver-telegram"</h1>
                <p>"Click below to authenticate with your identity provider."</p>
                <a href="/auth/login" rel="external" class="login-button">"Log in with SSO"</a>
            </div>
        </div>
    }
}

/// User settings page.
#[component]
fn SettingsPage() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());

    view! {
        <div class="settings-page">
            <h1>"Settings"</h1>
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || {
                    user.get().map(|result| {
                        match result {
                            Ok(Some(user_info)) => view! {
                                <SettingsContent user_info=user_info/>
                            }.into_any(),
                            Ok(None) => view! {
                                <div>
                                    <p>"Please log in to access settings."</p>
                                    <a href="/auth/login" rel="external">"Log in"</a>
                                </div>
                            }.into_any(),
                            Err(_) => view! {
                                <div>
                                    <p>"Failed to load settings. Please try again."</p>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

/// Settings content component (requires authenticated user).
#[component]
fn SettingsContent(user_info: UserInfo) -> impl IntoView {
    let save_tz = ServerAction::<SaveTimezone>::new();
    let (save_message, set_save_message) = signal(Option::<String>::None);

    // Effect to show save confirmation
    Effect::new(move || {
        if let Some(result) = save_tz.value().get() {
            match result {
                Ok(()) => set_save_message.set(Some("Timezone saved!".to_string())),
                Err(e) => set_save_message.set(Some(format!("Error: {}", e))),
            }
        }
    });

    view! {
        <div class="settings-content">
            <section class="settings-section">
                <h2>"Profile"</h2>
                <div class="setting-row">
                    <label>"Display Name"</label>
                    <span>{user_info.display_name.unwrap_or_else(|| "Not set".to_string())}</span>
                </div>
                <div class="setting-row">
                    <label>"Email"</label>
                    <span>{user_info.email.unwrap_or_else(|| "Not set".to_string())}</span>
                </div>
            </section>

            <section class="settings-section">
                <h2>"Preferences"</h2>
                <ActionForm action=save_tz>
                    <div class="setting-row">
                        <label for="timezone">"Timezone"</label>
                        <TimezoneSelector current_timezone=user_info.timezone.clone().unwrap_or_default()/>
                    </div>
                    <div class="setting-row">
                        <button type="submit" class="save-button">"Save Timezone"</button>
                        {move || save_message.get().map(|msg| view! { <span class="save-message">{msg}</span> })}
                    </div>
                </ActionForm>
            </section>

            <section class="settings-section">
                <h2>"Integrations"</h2>
                <p>"Manage your connected services."</p>
                <a href="/integrations" class="link-button">"Manage Integrations"</a>
            </section>
        </div>
    }
}

/// Timezone selector component.
#[component]
fn TimezoneSelector(#[prop(optional)] current_timezone: Option<String>) -> impl IntoView {
    // Common timezones
    let timezones = vec![
        ("UTC", "UTC"),
        ("America/New_York", "Eastern Time"),
        ("America/Chicago", "Central Time"),
        ("America/Denver", "Mountain Time"),
        ("America/Los_Angeles", "Pacific Time"),
        ("Europe/London", "London"),
        ("Europe/Paris", "Paris"),
        ("Europe/Berlin", "Berlin"),
        ("Asia/Tokyo", "Tokyo"),
        ("Asia/Shanghai", "Shanghai"),
        ("Australia/Sydney", "Sydney"),
    ];

    let current = current_timezone.unwrap_or_else(|| "UTC".to_string());

    view! {
        <select name="timezone" id="timezone" class="timezone-select">
            {timezones.into_iter().map(|(value, label)| {
                let selected = value == current;
                view! {
                    <option value=value selected=selected>{label}</option>
                }
            }).collect_view()}
        </select>
    }
}

/// Integration info for display.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct IntegrationInfo {
    pub id: String,
    pub name: String,
    pub integration_type: String,
    pub status: String,
}

/// Server function to list user's integrations.
#[server]
pub async fn list_integrations() -> Result<Vec<IntegrationInfo>, ServerFnError> {
    use crate::auth::db::SessionRepository;
    use axum::Extension;
    use axum_extra::extract::CookieJar;
    use silver_telegram_platform_access::SessionId;
    use sqlx::PgPool;

    const SESSION_COOKIE: &str = "session";

    // Extract the cookie jar
    let jar: CookieJar = leptos_axum::extract().await?;

    // Get database pool from request extension
    let Extension(db_pool): Extension<PgPool> = leptos_axum::extract().await?;

    // Verify authentication
    let session_cookie = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let session_id = SessionId::new(session_cookie.value().to_string());
    let session_repo = SessionRepository::new(db_pool);
    let session = session_repo
        .find_by_id(&session_id)
        .await
        .map_err(|_| ServerFnError::new("Database error"))?
        .ok_or_else(|| ServerFnError::new("Session not found"))?;

    if session.is_expired() || !session.has_access() {
        return Err(ServerFnError::new("Session expired"));
    }

    // Return empty list for now - integrations will be populated as they are added
    // The integration system exists in lib/integration but needs database persistence
    Ok(vec![])
}

/// Integrations page.
#[component]
fn IntegrationsPage() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());
    let integrations = Resource::new(|| (), |_| list_integrations());

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

                                    <section class="integrations-list">
                                        <h2>"Connected Services"</h2>
                                        <Suspense fallback=move || view! { <p>"Loading integrations..."</p> }>
                                            {move || {
                                                integrations.get().map(|result| {
                                                    match result {
                                                        Ok(items) if items.is_empty() => view! {
                                                            <p class="empty-state">"No integrations configured yet."</p>
                                                        }.into_any(),
                                                        Ok(items) => view! {
                                                            <ul class="integration-items">
                                                                {items.into_iter().map(|item| view! {
                                                                    <li class="integration-item">
                                                                        <span class="integration-name">{item.name}</span>
                                                                        <span class="integration-type">{item.integration_type}</span>
                                                                        <span class="integration-status">{item.status}</span>
                                                                    </li>
                                                                }).collect_view()}
                                                            </ul>
                                                        }.into_any(),
                                                        Err(_) => view! {
                                                            <p class="error">"Failed to load integrations."</p>
                                                        }.into_any(),
                                                    }
                                                })
                                            }}
                                        </Suspense>
                                    </section>

                                    <section class="add-integration">
                                        <h2>"Available Integration Types"</h2>
                                        <ul class="integration-types">
                                            <li>"Email (IMAP)"</li>
                                            <li>"Gmail (OAuth)"</li>
                                            <li>"Calendar (iCal feed)"</li>
                                        </ul>
                                        <p class="note">"Integration setup UI coming soon."</p>
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

/// Workflow info for admin display.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub owner_name: String,
    pub owner_id: String,
    pub enabled: bool,
    pub last_run: Option<String>,
}

/// Server function to list all workflows (admin only).
#[server]
pub async fn list_all_workflows() -> Result<Vec<WorkflowSummary>, ServerFnError> {
    use crate::auth::db::SessionRepository;
    use axum::Extension;
    use axum_extra::extract::CookieJar;
    use silver_telegram_platform_access::SessionId;
    use sqlx::PgPool;

    const SESSION_COOKIE: &str = "session";

    // Extract request data
    let jar: CookieJar = leptos_axum::extract().await?;

    // Get database pool from request extension
    let Extension(db_pool): Extension<PgPool> = leptos_axum::extract().await?;

    // Verify admin authentication
    let session_cookie = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let session_id = SessionId::new(session_cookie.value().to_string());
    let session_repo = SessionRepository::new(db_pool);
    let session = session_repo
        .find_by_id(&session_id)
        .await
        .map_err(|_| ServerFnError::new("Database error"))?
        .ok_or_else(|| ServerFnError::new("Session not found"))?;

    if session.is_expired() || !session.has_access() {
        return Err(ServerFnError::new("Session expired"));
    }

    if !session.is_admin() {
        return Err(ServerFnError::new("Admin access required"));
    }

    // Return empty list for now - workflows will be populated as they are created
    // The workflow system exists in lib/workflow but needs database persistence
    Ok(vec![])
}

/// Server function to trigger a workflow (admin only).
#[server]
pub async fn trigger_workflow(workflow_id: String) -> Result<(), ServerFnError> {
    use crate::auth::db::SessionRepository;
    use axum::Extension;
    use axum_extra::extract::CookieJar;
    use silver_telegram_platform_access::SessionId;
    use sqlx::PgPool;

    const SESSION_COOKIE: &str = "session";

    // Extract request data
    let jar: CookieJar = leptos_axum::extract().await?;

    // Get database pool from request extension
    let Extension(db_pool): Extension<PgPool> = leptos_axum::extract().await?;

    // Verify admin authentication
    let session_cookie = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let session_id = SessionId::new(session_cookie.value().to_string());
    let session_repo = SessionRepository::new(db_pool);
    let session = session_repo
        .find_by_id(&session_id)
        .await
        .map_err(|_| ServerFnError::new("Database error"))?
        .ok_or_else(|| ServerFnError::new("Session not found"))?;

    if !session.is_admin() {
        return Err(ServerFnError::new("Admin access required"));
    }

    // Log the admin action (workflow trigger will be implemented when workflow execution is added)
    let _ = workflow_id; // Acknowledge the parameter
    Ok(())
}

/// Server function to cancel a workflow run (admin only).
#[server]
pub async fn cancel_workflow(workflow_id: String) -> Result<(), ServerFnError> {
    use crate::auth::db::SessionRepository;
    use axum::Extension;
    use axum_extra::extract::CookieJar;
    use silver_telegram_platform_access::SessionId;
    use sqlx::PgPool;

    const SESSION_COOKIE: &str = "session";

    // Extract request data
    let jar: CookieJar = leptos_axum::extract().await?;

    // Get database pool from request extension
    let Extension(db_pool): Extension<PgPool> = leptos_axum::extract().await?;

    // Verify admin authentication
    let session_cookie = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let session_id = SessionId::new(session_cookie.value().to_string());
    let session_repo = SessionRepository::new(db_pool);
    let session = session_repo
        .find_by_id(&session_id)
        .await
        .map_err(|_| ServerFnError::new("Database error"))?
        .ok_or_else(|| ServerFnError::new("Session not found"))?;

    if !session.is_admin() {
        return Err(ServerFnError::new("Admin access required"));
    }

    // Log the admin action (workflow cancel will be implemented when workflow execution is added)
    let _ = workflow_id; // Acknowledge the parameter
    Ok(())
}

/// Admin page (requires admin access).
#[component]
fn AdminPage() -> impl IntoView {
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
                                                                        <th>"Owner"</th>
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
                                                                                <td>{wf.name}</td>
                                                                                <td>{wf.owner_name}</td>
                                                                                <td>{if wf.enabled { "Enabled" } else { "Disabled" }}</td>
                                                                                <td>{wf.last_run.unwrap_or_else(|| "Never".to_string())}</td>
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
