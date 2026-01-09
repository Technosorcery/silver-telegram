//! Integration server functions and types.
//!
//! Contains all server-side CRUD operations for integrations.

use leptos::prelude::*;

/// Integration info for display.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct IntegrationInfo {
    pub id: String,
    pub name: String,
    pub integration_type: String,
    pub status: String,
    pub error_message: Option<String>,
}

/// Integration configuration for editing.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IntegrationConfigData {
    pub server: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub use_tls: Option<bool>,
    pub url: Option<String>,
    // OpenAI-compatible fields
    pub endpoint_url: Option<String>,
    pub api_key: Option<String>,
}

/// Model info from discovery.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
}

/// Server function to list user's integrations.
#[server]
pub async fn list_integrations() -> Result<Vec<IntegrationInfo>, ServerFnError> {
    use crate::db::IntegrationAccountRepository;
    use crate::error::IntegrationError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, ResourceType, Subject};
    use silver_telegram_core::IntegrationAccountId;
    use std::str::FromStr;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for list_integrations");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    // Query SpiceDB for integration IDs the user can view
    let subject = Subject::user(auth.user_id);
    let integration_ids = authz_client
        .lookup_resources(ResourceType::Integration, Permission::View, &subject)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %auth.user_id,
                "Failed to lookup accessible integrations from SpiceDB"
            );
            IntegrationError::AuthorizationError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    // Parse integration IDs
    let integration_ids: Vec<IntegrationAccountId> = integration_ids
        .iter()
        .filter_map(|id| IntegrationAccountId::from_str(id).ok())
        .collect();

    let integration_repo = IntegrationAccountRepository::new(db_pool);
    let integrations = integration_repo
        .list_by_ids(&integration_ids)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %auth.user_id,
                integration_count = integration_ids.len(),
                "Failed to load integrations from database"
            );
            IntegrationError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    Ok(integrations
        .into_iter()
        .map(|i| IntegrationInfo {
            id: i.id.to_string(),
            name: i.name,
            integration_type: i.integration_type,
            status: format!("{:?}", i.status).to_lowercase(),
            error_message: i.error_message,
        })
        .collect())
}

/// Server function to get integration config for editing.
#[server]
pub async fn get_integration_config(
    integration_id: String,
) -> Result<IntegrationConfigData, ServerFnError> {
    use crate::db::IntegrationConfigRepository;
    use crate::error::IntegrationError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::IntegrationAccountId;
    use std::str::FromStr;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for get_integration_config");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    let int_id = IntegrationAccountId::from_str(&integration_id).map_err(|e| {
        tracing::debug!(
            error = %e,
            integration_id = %integration_id,
            "Invalid integration ID format"
        );
        IntegrationError::InvalidId {
            id: integration_id.clone(),
            reason: e.to_string(),
        }
        .into_server_error()
    })?;

    // Check view permission via SpiceDB
    let resource = Resource::integration(int_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::View, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                error = %e,
                user_id = %auth.user_id,
                integration_id = %int_id,
                permission = "view",
                "Access denied to view integration config"
            );
            IntegrationError::AccessDenied {
                id: int_id.to_string(),
            }
            .into_server_error()
        })?;

    let config_repo = IntegrationConfigRepository::new(db_pool);
    let config = config_repo.find_by_integration(int_id).await.map_err(|e| {
        tracing::error!(
            error = %e,
            integration_id = %int_id,
            "Failed to load integration config from database"
        );
        IntegrationError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    match config {
        Some(c) => {
            let data: IntegrationConfigData =
                serde_json::from_value(c.config_data).unwrap_or_default();
            Ok(data)
        }
        None => Ok(IntegrationConfigData::default()),
    }
}

/// Server function to create a new integration.
#[server]
pub async fn create_integration(
    name: String,
    integration_type: String,
    config_json: String,
) -> Result<String, ServerFnError> {
    use crate::db::{
        IntegrationAccount, IntegrationAccountRepository, IntegrationConfigRepository,
    };
    use crate::error::IntegrationError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::Relationship;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for create_integration");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    let config: serde_json::Value = serde_json::from_str(&config_json).map_err(|e| {
        tracing::debug!(
            error = %e,
            "Invalid config JSON for create_integration"
        );
        IntegrationError::InvalidConfig {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    let integration = IntegrationAccount::new(name.clone(), integration_type.clone());

    let integration_repo = IntegrationAccountRepository::new(db_pool.clone());
    integration_repo.create(&integration).await.map_err(|e| {
        tracing::error!(
            error = %e,
            user_id = %auth.user_id,
            integration_name = %name,
            integration_type = %integration_type,
            "Failed to create integration in database"
        );
        IntegrationError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    // Create ownership relationship in SpiceDB
    let relationship = Relationship::integration_owner(integration.id, auth.user_id);
    authz_client
        .write_relationship(&relationship)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %auth.user_id,
                integration_id = %integration.id,
                "Failed to set integration ownership in SpiceDB"
            );
            IntegrationError::AuthorizationError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    let config_repo = IntegrationConfigRepository::new(db_pool);
    config_repo
        .upsert(integration.id, config)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                integration_id = %integration.id,
                "Failed to save integration config"
            );
            IntegrationError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    tracing::info!(
        user_id = %auth.user_id,
        integration_id = %integration.id,
        integration_name = %name,
        integration_type = %integration_type,
        "Created new integration"
    );

    Ok(integration.id.to_string())
}

/// Server function to update integration name.
#[server]
pub async fn update_integration_name(
    integration_id: String,
    new_name: String,
) -> Result<(), ServerFnError> {
    use crate::db::IntegrationAccountRepository;
    use crate::error::IntegrationError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::IntegrationAccountId;
    use std::str::FromStr;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for update_integration_name");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    let int_id = IntegrationAccountId::from_str(&integration_id).map_err(|e| {
        tracing::debug!(
            error = %e,
            integration_id = %integration_id,
            "Invalid integration ID format"
        );
        IntegrationError::InvalidId {
            id: integration_id.clone(),
            reason: e.to_string(),
        }
        .into_server_error()
    })?;

    // Check edit permission via SpiceDB
    let resource = Resource::integration(int_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::Edit, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                error = %e,
                user_id = %auth.user_id,
                integration_id = %int_id,
                permission = "edit",
                "Access denied to update integration name"
            );
            IntegrationError::AccessDenied {
                id: int_id.to_string(),
            }
            .into_server_error()
        })?;

    let integration_repo = IntegrationAccountRepository::new(db_pool);
    let mut integration = integration_repo
        .find_by_id(int_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                integration_id = %int_id,
                "Failed to find integration in database"
            );
            IntegrationError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?
        .ok_or_else(|| {
            tracing::debug!(integration_id = %int_id, "Integration not found");
            IntegrationError::NotFound {
                id: int_id.to_string(),
            }
            .into_server_error()
        })?;

    integration.name = new_name.clone();
    integration_repo.update(&integration).await.map_err(|e| {
        tracing::error!(
            error = %e,
            integration_id = %int_id,
            "Failed to update integration name"
        );
        IntegrationError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    tracing::info!(
        user_id = %auth.user_id,
        integration_id = %int_id,
        new_name = %new_name,
        "Updated integration name"
    );

    Ok(())
}

/// Server function to update integration config.
#[server]
pub async fn update_integration_config(
    integration_id: String,
    config_json: String,
) -> Result<(), ServerFnError> {
    use crate::db::IntegrationConfigRepository;
    use crate::error::IntegrationError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::IntegrationAccountId;
    use std::str::FromStr;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for update_integration_config");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    let int_id = IntegrationAccountId::from_str(&integration_id).map_err(|e| {
        tracing::debug!(
            error = %e,
            integration_id = %integration_id,
            "Invalid integration ID format"
        );
        IntegrationError::InvalidId {
            id: integration_id.clone(),
            reason: e.to_string(),
        }
        .into_server_error()
    })?;

    // Check edit permission via SpiceDB
    let resource = Resource::integration(int_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::Edit, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                error = %e,
                user_id = %auth.user_id,
                integration_id = %int_id,
                permission = "edit",
                "Access denied to update integration config"
            );
            IntegrationError::AccessDenied {
                id: int_id.to_string(),
            }
            .into_server_error()
        })?;

    let config: serde_json::Value = serde_json::from_str(&config_json).map_err(|e| {
        tracing::debug!(
            error = %e,
            integration_id = %int_id,
            "Invalid config JSON"
        );
        IntegrationError::InvalidConfig {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    let config_repo = IntegrationConfigRepository::new(db_pool);
    config_repo.upsert(int_id, config).await.map_err(|e| {
        tracing::error!(
            error = %e,
            integration_id = %int_id,
            "Failed to update integration config"
        );
        IntegrationError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    tracing::info!(
        user_id = %auth.user_id,
        integration_id = %int_id,
        "Updated integration config"
    );

    Ok(())
}

/// Server function to delete an integration.
#[server]
pub async fn delete_integration(
    integration_id: String,
) -> Result<Option<Vec<String>>, ServerFnError> {
    use crate::db::IntegrationAccountRepository;
    use crate::error::IntegrationError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::IntegrationAccountId;
    use std::str::FromStr;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for delete_integration");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    let int_id = IntegrationAccountId::from_str(&integration_id).map_err(|e| {
        tracing::debug!(
            error = %e,
            integration_id = %integration_id,
            "Invalid integration ID format"
        );
        IntegrationError::InvalidId {
            id: integration_id.clone(),
            reason: e.to_string(),
        }
        .into_server_error()
    })?;

    // Check delete permission via SpiceDB
    let resource = Resource::integration(int_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::Delete, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                error = %e,
                user_id = %auth.user_id,
                integration_id = %int_id,
                permission = "delete",
                "Access denied to delete integration"
            );
            IntegrationError::AccessDenied {
                id: int_id.to_string(),
            }
            .into_server_error()
        })?;

    let integration_repo = IntegrationAccountRepository::new(db_pool);

    let using_workflows = integration_repo
        .is_used_by_workflows(int_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                integration_id = %int_id,
                "Failed to check workflow usage"
            );
            IntegrationError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    if !using_workflows.is_empty() {
        tracing::debug!(
            integration_id = %int_id,
            workflows = ?using_workflows,
            "Cannot delete integration - in use by workflows"
        );
        return Ok(Some(using_workflows));
    }

    integration_repo.delete(int_id).await.map_err(|e| {
        tracing::error!(
            error = %e,
            integration_id = %int_id,
            "Failed to delete integration from database"
        );
        IntegrationError::DatabaseError {
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
                integration_id = %int_id,
                "Failed to delete integration relationships from SpiceDB"
            );
            IntegrationError::AuthorizationError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    tracing::info!(
        user_id = %auth.user_id,
        integration_id = %int_id,
        "Deleted integration"
    );

    Ok(None)
}

/// Server function to discover available models from an OpenAI-compatible integration.
#[server]
pub async fn discover_models(integration_id: String) -> Result<Vec<ModelInfo>, ServerFnError> {
    use crate::db::{IntegrationAccountRepository, IntegrationConfigRepository};
    use crate::error::ModelDiscoveryError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, Resource, Subject};
    use silver_telegram_core::IntegrationAccountId;
    use std::str::FromStr;
    use std::time::Duration;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for discover_models");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    let int_id = IntegrationAccountId::from_str(&integration_id).map_err(|e| {
        tracing::debug!(
            error = %e,
            integration_id = %integration_id,
            "Invalid integration ID format"
        );
        ModelDiscoveryError::IntegrationNotFound {
            id: integration_id.clone(),
        }
        .into_server_error()
    })?;

    // Check view permission via SpiceDB
    let resource = Resource::integration(int_id);
    let subject = Subject::user(auth.user_id);
    authz_client
        .require_permission(&resource, Permission::View, &subject)
        .await
        .map_err(|e| {
            tracing::warn!(
                error = %e,
                user_id = %auth.user_id,
                integration_id = %int_id,
                permission = "view",
                "Access denied to discover models"
            );
            ModelDiscoveryError::AccessDenied {
                id: int_id.to_string(),
            }
            .into_server_error()
        })?;

    // Get integration to verify type
    let integration_repo = IntegrationAccountRepository::new(db_pool.clone());
    let integration = integration_repo
        .find_by_id(int_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                integration_id = %int_id,
                "Failed to find integration in database"
            );
            ModelDiscoveryError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?
        .ok_or_else(|| {
            tracing::debug!(integration_id = %int_id, "Integration not found");
            ModelDiscoveryError::IntegrationNotFound {
                id: int_id.to_string(),
            }
            .into_server_error()
        })?;

    // Verify it's an openai_compatible integration
    if integration.integration_type != "openai_compatible" {
        return Err(ModelDiscoveryError::InvalidIntegrationType {
            id: int_id.to_string(),
            actual_type: integration.integration_type,
        }
        .into_server_error());
    }

    // Get config to find endpoint
    let config_repo = IntegrationConfigRepository::new(db_pool);
    let config = config_repo.find_by_integration(int_id).await.map_err(|e| {
        tracing::error!(
            error = %e,
            integration_id = %int_id,
            "Failed to load integration config from database"
        );
        ModelDiscoveryError::DatabaseError {
            details: e.to_string(),
        }
        .into_server_error()
    })?;

    let config_data: IntegrationConfigData = match config {
        Some(c) => serde_json::from_value(c.config_data).unwrap_or_default(),
        None => IntegrationConfigData::default(),
    };

    let endpoint_url = config_data.endpoint_url.ok_or_else(|| {
        ModelDiscoveryError::ConnectionFailed {
            endpoint: String::new(),
            reason: "No endpoint URL configured".to_string(),
        }
        .into_server_error()
    })?;

    // Make request to /v1/models
    let models_url = format!("{}/v1/models", endpoint_url.trim_end_matches('/'));

    tracing::debug!(
        integration_id = %int_id,
        endpoint = %models_url,
        "Discovering models from OpenAI-compatible endpoint"
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| {
            ModelDiscoveryError::ConnectionFailed {
                endpoint: models_url.clone(),
                reason: e.to_string(),
            }
            .into_server_error()
        })?;

    let mut request = client.get(&models_url);

    // Add API key if configured
    if let Some(api_key) = config_data.api_key
        && !api_key.is_empty()
    {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.send().await.map_err(|e| {
        tracing::warn!(
            error = %e,
            integration_id = %int_id,
            endpoint = %models_url,
            "Failed to connect to OpenAI-compatible endpoint"
        );
        if e.is_timeout() {
            ModelDiscoveryError::Timeout {
                endpoint: models_url.clone(),
            }
            .into_server_error()
        } else {
            ModelDiscoveryError::ConnectionFailed {
                endpoint: models_url.clone(),
                reason: e.to_string(),
            }
            .into_server_error()
        }
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(
            integration_id = %int_id,
            endpoint = %models_url,
            status = %status,
            body = %body,
            "Model discovery endpoint returned error"
        );
        return Err(ModelDiscoveryError::ConnectionFailed {
            endpoint: models_url,
            reason: format!("HTTP {}: {}", status, body),
        }
        .into_server_error());
    }

    // Parse OpenAI models response
    #[derive(serde::Deserialize)]
    struct ModelsResponse {
        data: Vec<Model>,
    }

    #[derive(serde::Deserialize)]
    struct Model {
        id: String,
    }

    let models_response: ModelsResponse = response.json().await.map_err(|e| {
        tracing::warn!(
            error = %e,
            integration_id = %int_id,
            endpoint = %models_url,
            "Failed to parse models response"
        );
        ModelDiscoveryError::ParseError {
            reason: e.to_string(),
        }
        .into_server_error()
    })?;

    let models: Vec<ModelInfo> = models_response
        .data
        .into_iter()
        .map(|m| ModelInfo {
            name: m.id.clone(),
            id: m.id,
        })
        .collect();

    tracing::info!(
        integration_id = %int_id,
        user_id = %auth.user_id,
        model_count = models.len(),
        "Discovered models from OpenAI-compatible endpoint"
    );

    Ok(models)
}

/// Server function to test connection to an OpenAI-compatible endpoint.
#[server]
pub async fn test_openai_connection(
    endpoint_url: String,
    api_key: Option<String>,
) -> Result<bool, ServerFnError> {
    use crate::error::ModelDiscoveryError;
    use crate::server_helpers::get_authenticated_session;
    use std::time::Duration;

    // Authenticate user (must be logged in to test connections)
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for test_openai_connection");
        e.into_server_error()
    })?;

    let models_url = format!("{}/v1/models", endpoint_url.trim_end_matches('/'));

    tracing::debug!(
        user_id = %auth.user_id,
        endpoint = %models_url,
        "Testing connection to OpenAI-compatible endpoint"
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| {
            ModelDiscoveryError::ConnectionFailed {
                endpoint: models_url.clone(),
                reason: e.to_string(),
            }
            .into_server_error()
        })?;

    let mut request = client.get(&models_url);

    // Add API key if provided
    if let Some(key) = api_key
        && !key.is_empty()
    {
        request = request.header("Authorization", format!("Bearer {}", key));
    }

    let response = request.send().await.map_err(|e| {
        tracing::debug!(
            error = %e,
            endpoint = %models_url,
            "Connection test failed"
        );
        if e.is_timeout() {
            ModelDiscoveryError::Timeout {
                endpoint: models_url.clone(),
            }
            .into_server_error()
        } else {
            ModelDiscoveryError::ConnectionFailed {
                endpoint: models_url.clone(),
                reason: e.to_string(),
            }
            .into_server_error()
        }
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::debug!(
            endpoint = %models_url,
            status = %status,
            body = %body,
            "Connection test returned error"
        );
        return Err(ModelDiscoveryError::ConnectionFailed {
            endpoint: models_url,
            reason: format!("HTTP {}: {}", status, body),
        }
        .into_server_error());
    }

    tracing::info!(
        user_id = %auth.user_id,
        endpoint = %models_url,
        "Connection test successful"
    );

    Ok(true)
}

/// Server function to list OpenAI-compatible integrations only.
#[server]
pub async fn list_openai_integrations() -> Result<Vec<IntegrationInfo>, ServerFnError> {
    use crate::db::IntegrationAccountRepository;
    use crate::error::IntegrationError;
    use crate::server_helpers::{get_authenticated_session, get_authz_client, get_db_pool};
    use silver_telegram_authz::{Permission, ResourceType, Subject};
    use silver_telegram_core::IntegrationAccountId;
    use std::str::FromStr;

    // Authenticate user
    let auth = get_authenticated_session().await.map_err(|e| {
        tracing::debug!(error = %e, "Authentication failed for list_openai_integrations");
        e.into_server_error()
    })?;

    let authz_client = get_authz_client();
    let db_pool = get_db_pool();

    // Query SpiceDB for integration IDs the user can view
    let subject = Subject::user(auth.user_id);
    let integration_ids = authz_client
        .lookup_resources(ResourceType::Integration, Permission::View, &subject)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %auth.user_id,
                "Failed to lookup accessible integrations from SpiceDB"
            );
            IntegrationError::AuthorizationError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    // Parse integration IDs
    let integration_ids: Vec<IntegrationAccountId> = integration_ids
        .iter()
        .filter_map(|id| IntegrationAccountId::from_str(id).ok())
        .collect();

    let integration_repo = IntegrationAccountRepository::new(db_pool);
    let integrations = integration_repo
        .list_by_ids(&integration_ids)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %auth.user_id,
                integration_count = integration_ids.len(),
                "Failed to load integrations from database"
            );
            IntegrationError::DatabaseError {
                details: e.to_string(),
            }
            .into_server_error()
        })?;

    // Filter to only openai_compatible integrations
    Ok(integrations
        .into_iter()
        .filter(|i| i.integration_type == "openai_compatible")
        .map(|i| IntegrationInfo {
            id: i.id.to_string(),
            name: i.name,
            integration_type: i.integration_type,
            status: format!("{:?}", i.status).to_lowercase(),
            error_message: i.error_message,
        })
        .collect())
}
