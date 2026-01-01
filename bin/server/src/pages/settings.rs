//! Settings page component and related server functions.

use crate::types::UserInfo;
use crate::user::get_current_user;
use leptos::form::ActionForm;
use leptos::prelude::*;

/// Server function to save the user's timezone.
#[server]
pub async fn save_timezone(timezone: String) -> Result<(), ServerFnError> {
    use crate::auth::db::UserRepository;
    use crate::error::UserError;
    use crate::server_helpers::{get_authenticated_session, get_db_pool};

    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for save_timezone");
        e.into_server_error()
    })?;

    let db_pool = get_db_pool();
    let user_repo = UserRepository::new(db_pool.clone());

    let mut user = user_repo
        .find_by_id(auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %auth.user_id,
                "Database error loading user"
            );
            UserError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?
        .ok_or_else(|| {
            tracing::error!(user_id = %auth.user_id, "User not found after authentication");
            UserError::NotFound {
                id: auth.user_id.to_string(),
            }
            .into_server_error()
        })?;

    user.set_timezone(Some(timezone.clone()));
    user_repo.update(&user).await.map_err(|e| {
        tracing::error!(
            error = %e,
            user_id = %auth.user_id,
            timezone = %timezone,
            "Failed to update user timezone"
        );
        UserError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    tracing::info!(user_id = %auth.user_id, timezone = %timezone, "User timezone updated");

    Ok(())
}

/// User settings page.
#[component]
pub fn SettingsPage() -> impl IntoView {
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
