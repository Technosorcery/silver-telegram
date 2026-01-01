//! Database repositories for users and sessions.

use chrono::{DateTime, Utc};
use silver_telegram_core::UserId;
use silver_telegram_platform_access::{RoleSet, Session, SessionId, User};
use sqlx::{FromRow, PgPool};
use std::str::FromStr;

/// Row type for user queries.
#[derive(FromRow)]
struct UserRow {
    id: String,
    subject: String,
    issuer: String,
    email: Option<String>,
    display_name: Option<String>,
    timezone: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl UserRow {
    fn try_into_user(self) -> Result<User, sqlx::Error> {
        let id = UserId::from_str(&self.id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid user id '{}': {}", self.id, e),
            )))
        })?;
        Ok(User::with_all_fields(
            id,
            self.subject,
            self.issuer,
            self.email,
            self.display_name,
            self.timezone,
            self.created_at,
            self.updated_at,
        ))
    }
}

/// Row type for session queries.
#[derive(FromRow)]
struct SessionRow {
    id: String,
    user_id: String,
    roles: serde_json::Value,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    access_token: Option<String>,
    refresh_token: Option<String>,
}

impl SessionRow {
    fn try_into_session(self) -> Result<Session, sqlx::Error> {
        let roles: RoleSet = serde_json::from_value(self.roles).unwrap_or_default();
        let user_id = UserId::from_str(&self.user_id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid user id '{}': {}", self.user_id, e),
            )))
        })?;

        let session =
            if let (Some(access_token), refresh_token) = (self.access_token, self.refresh_token) {
                Session::with_tokens(
                    SessionId::new(self.id),
                    user_id,
                    roles,
                    self.expires_at - self.created_at,
                    access_token,
                    refresh_token,
                )
            } else {
                Session::new(
                    SessionId::new(self.id),
                    user_id,
                    roles,
                    self.expires_at - self.created_at,
                )
            };
        Ok(session)
    }
}

/// Repository for user operations.
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Creates a new user repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Finds a user by their OIDC subject and issuer.
    pub async fn find_by_subject_issuer(
        &self,
        subject: &str,
        issuer: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        let row: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, subject, issuer, email, display_name, timezone, created_at, updated_at
            FROM users
            WHERE subject = $1 AND issuer = $2
            "#,
        )
        .bind(subject)
        .bind(issuer)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.try_into_user()?)),
            None => Ok(None),
        }
    }

    /// Finds a user by their internal ID.
    pub async fn find_by_id(&self, id: UserId) -> Result<Option<User>, sqlx::Error> {
        let row: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, subject, issuer, email, display_name, timezone, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.try_into_user()?)),
            None => Ok(None),
        }
    }

    /// Creates a new user.
    pub async fn create(&self, user: &User) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO users (id, subject, issuer, email, display_name, timezone, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(user.id().to_string())
        .bind(user.subject())
        .bind(user.issuer())
        .bind(user.email())
        .bind(user.display_name())
        .bind(user.timezone())
        .bind(user.created_at())
        .bind(user.updated_at())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Updates an existing user.
    pub async fn update(&self, user: &User) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET email = $2, display_name = $3, timezone = $4, updated_at = $5
            WHERE id = $1
            "#,
        )
        .bind(user.id().to_string())
        .bind(user.email())
        .bind(user.display_name())
        .bind(user.timezone())
        .bind(user.updated_at())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Repository for session operations.
pub struct SessionRepository {
    pool: PgPool,
}

impl SessionRepository {
    /// Creates a new session repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Finds a session by ID.
    pub async fn find_by_id(&self, id: &SessionId) -> Result<Option<Session>, sqlx::Error> {
        let row: Option<SessionRow> = sqlx::query_as(
            r#"
            SELECT id, user_id, roles, created_at, expires_at, access_token, refresh_token
            FROM sessions
            WHERE id = $1
            "#,
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.try_into_session()?)),
            None => Ok(None),
        }
    }

    /// Creates a new session.
    pub async fn create(&self, session: &Session) -> Result<(), sqlx::Error> {
        let roles_json = serde_json::to_value(session.roles()).expect("serialize roles");

        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, roles, created_at, expires_at, access_token, refresh_token)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(session.id().as_str())
        .bind(session.user_id().to_string())
        .bind(roles_json)
        .bind(session.created_at())
        .bind(session.expires_at())
        .bind(session.access_token())
        .bind(session.refresh_token())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes a session by ID (logout).
    pub async fn delete(&self, id: &SessionId) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE id = $1
            "#,
        )
        .bind(id.as_str())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes all sessions for a user.
    pub async fn delete_all_for_user(&self, user_id: UserId) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE user_id = $1
            "#,
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes expired sessions.
    pub async fn delete_expired(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE expires_at < NOW()
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

/// Generates a unique session ID using ULID.
pub fn generate_session_id() -> SessionId {
    SessionId::new(ulid::Ulid::new().to_string())
}
