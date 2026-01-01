//! Database repositories for integration accounts and configuration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::IntegrationAccountId;
use sqlx::{FromRow, PgPool};
use std::str::FromStr;

/// Status of an integration account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationStatus {
    /// Successfully connected and working.
    Connected,
    /// Connection failed or credentials invalid.
    Error,
    /// Awaiting OAuth completion or initial connection.
    Pending,
}

impl IntegrationStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Error => "error",
            Self::Pending => "pending",
        }
    }

    fn from_str_value(s: &str) -> Self {
        match s {
            "connected" => Self::Connected,
            "error" => Self::Error,
            _ => Self::Pending,
        }
    }
}

/// An integration account record.
///
/// Note: Ownership is stored in SpiceDB via relationships, not in this table.
/// Use AuthzClient to check permissions (view, edit, delete, use).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationAccount {
    /// Integration account ID.
    pub id: IntegrationAccountId,
    /// User-provided name/label.
    pub name: String,
    /// Type of integration (e.g., "imap", "gmail", "calendar_feed").
    pub integration_type: String,
    /// Current status.
    pub status: IntegrationStatus,
    /// Error message if status is error.
    pub error_message: Option<String>,
    /// When the integration was created.
    pub created_at: DateTime<Utc>,
    /// When the integration was last updated.
    pub updated_at: DateTime<Utc>,
    /// When the integration was last successfully used.
    pub last_used_at: Option<DateTime<Utc>>,
}

impl IntegrationAccount {
    /// Creates a new integration account.
    ///
    /// Note: After creating the record, you must also create an ownership
    /// relationship in SpiceDB using AuthzClient::write_relationship.
    #[must_use]
    pub fn new(name: String, integration_type: String) -> Self {
        let now = Utc::now();
        Self {
            id: IntegrationAccountId::new(),
            name,
            integration_type,
            status: IntegrationStatus::Pending,
            error_message: None,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        }
    }

    /// Sets the status to connected.
    pub fn mark_connected(&mut self) {
        self.status = IntegrationStatus::Connected;
        self.error_message = None;
        self.updated_at = Utc::now();
    }

    /// Sets the status to error.
    pub fn mark_error(&mut self, message: String) {
        self.status = IntegrationStatus::Error;
        self.error_message = Some(message);
        self.updated_at = Utc::now();
    }

    /// Updates the last used timestamp.
    pub fn mark_used(&mut self) {
        self.last_used_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

/// Row type for integration account queries.
#[derive(FromRow)]
struct IntegrationAccountRow {
    id: String,
    name: String,
    integration_type: String,
    status: String,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

impl IntegrationAccountRow {
    fn try_into_account(self) -> Result<IntegrationAccount, sqlx::Error> {
        let id = IntegrationAccountId::from_str(&self.id).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid integration account id '{}': {}", self.id, e),
            )))
        })?;

        Ok(IntegrationAccount {
            id,
            name: self.name,
            integration_type: self.integration_type,
            status: IntegrationStatus::from_str_value(&self.status),
            error_message: self.error_message,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_used_at: self.last_used_at,
        })
    }
}

/// Repository for integration account operations.
pub struct IntegrationAccountRepository {
    pool: PgPool,
}

impl IntegrationAccountRepository {
    /// Creates a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Lists integration accounts by IDs.
    ///
    /// Use this after querying SpiceDB for integration IDs the user has access to.
    pub async fn list_by_ids(
        &self,
        ids: &[IntegrationAccountId],
    ) -> Result<Vec<IntegrationAccount>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        let rows: Vec<IntegrationAccountRow> = sqlx::query_as(
            r#"
            SELECT id, name, integration_type, status, error_message,
                   created_at, updated_at, last_used_at
            FROM integration_accounts
            WHERE id = ANY($1)
            ORDER BY name ASC
            "#,
        )
        .bind(&id_strings)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into_account()).collect()
    }

    /// Finds an integration account by ID.
    pub async fn find_by_id(
        &self,
        id: IntegrationAccountId,
    ) -> Result<Option<IntegrationAccount>, sqlx::Error> {
        let row: Option<IntegrationAccountRow> = sqlx::query_as(
            r#"
            SELECT id, name, integration_type, status, error_message,
                   created_at, updated_at, last_used_at
            FROM integration_accounts
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.try_into_account()?)),
            None => Ok(None),
        }
    }

    /// Creates a new integration account.
    ///
    /// Note: After creating the integration, you must also create an ownership
    /// relationship in SpiceDB using AuthzClient::write_relationship.
    pub async fn create(&self, account: &IntegrationAccount) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO integration_accounts
                (id, name, integration_type, status, error_message,
                 created_at, updated_at, last_used_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(account.id.to_string())
        .bind(&account.name)
        .bind(&account.integration_type)
        .bind(account.status.as_str())
        .bind(&account.error_message)
        .bind(account.created_at)
        .bind(account.updated_at)
        .bind(account.last_used_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Updates an existing integration account.
    pub async fn update(&self, account: &IntegrationAccount) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE integration_accounts
            SET name = $2, status = $3, error_message = $4, updated_at = $5, last_used_at = $6
            WHERE id = $1
            "#,
        )
        .bind(account.id.to_string())
        .bind(&account.name)
        .bind(account.status.as_str())
        .bind(&account.error_message)
        .bind(account.updated_at)
        .bind(account.last_used_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes an integration account.
    pub async fn delete(&self, id: IntegrationAccountId) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM integration_accounts
            WHERE id = $1
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Checks if an integration is used by any workflows.
    pub async fn is_used_by_workflows(
        &self,
        id: IntegrationAccountId,
    ) -> Result<Vec<String>, sqlx::Error> {
        // Check workflow graph_data for references to this integration
        // This searches the JSONB for nodes that reference this integration
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT w.name
            FROM workflows w
            WHERE w.graph_data::text LIKE '%' || $1 || '%'
            ORDER BY w.name
            "#,
        )
        .bind(id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(name,)| name).collect())
    }
}

/// Integration configuration record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Config ID.
    pub id: String,
    /// Integration account this config belongs to.
    pub integration_account_id: IntegrationAccountId,
    /// Configuration data (type-specific).
    pub config_data: serde_json::Value,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// When last updated.
    pub updated_at: DateTime<Utc>,
}

/// Row type for integration config queries.
#[derive(FromRow)]
struct IntegrationConfigRow {
    id: String,
    integration_account_id: String,
    config_data: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl IntegrationConfigRow {
    fn try_into_config(self) -> Result<IntegrationConfig, sqlx::Error> {
        let integration_account_id = IntegrationAccountId::from_str(&self.integration_account_id)
            .map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "invalid integration account id '{}': {}",
                    self.integration_account_id, e
                ),
            )))
        })?;

        Ok(IntegrationConfig {
            id: self.id,
            integration_account_id,
            config_data: self.config_data,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Repository for integration configuration.
pub struct IntegrationConfigRepository {
    pool: PgPool,
}

impl IntegrationConfigRepository {
    /// Creates a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Finds config for an integration account.
    pub async fn find_by_integration(
        &self,
        integration_id: IntegrationAccountId,
    ) -> Result<Option<IntegrationConfig>, sqlx::Error> {
        let row: Option<IntegrationConfigRow> = sqlx::query_as(
            r#"
            SELECT id, integration_account_id, config_data, created_at, updated_at
            FROM integration_config
            WHERE integration_account_id = $1
            "#,
        )
        .bind(integration_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.try_into_config()?)),
            None => Ok(None),
        }
    }

    /// Creates or updates config for an integration.
    pub async fn upsert(
        &self,
        integration_id: IntegrationAccountId,
        config_data: serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let id = ulid::Ulid::new().to_string();

        sqlx::query(
            r#"
            INSERT INTO integration_config (id, integration_account_id, config_data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $4)
            ON CONFLICT (integration_account_id)
            DO UPDATE SET config_data = $3, updated_at = $4
            "#,
        )
        .bind(&id)
        .bind(integration_id.to_string())
        .bind(&config_data)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
