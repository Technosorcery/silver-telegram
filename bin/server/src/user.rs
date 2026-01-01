//! User-related server functions for identity and session management.

use crate::types::UserInfo;
use leptos::prelude::*;

/// Server function to get the current user info.
#[server]
pub async fn get_current_user() -> Result<Option<UserInfo>, ServerFnError> {
    use crate::auth::db::{SessionRepository, UserRepository};
    use axum::Extension;
    use axum_extra::extract::CookieJar;
    use silver_telegram_platform_access::SessionId;
    use sqlx::PgPool;

    const SESSION_COOKIE: &str = "session";

    let jar: CookieJar = leptos_axum::extract().await?;
    let Extension(db_pool): Extension<PgPool> = leptos_axum::extract().await?;

    let session_cookie = match jar.get(SESSION_COOKIE) {
        Some(cookie) => cookie,
        None => return Ok(None),
    };

    let session_id = SessionId::new(session_cookie.value().to_string());

    let session_repo = SessionRepository::new(db_pool.clone());
    let session = match session_repo.find_by_id(&session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => return Ok(None),
        Err(_) => return Ok(None),
    };

    if session.is_expired() || !session.has_access() {
        return Ok(None);
    }

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
