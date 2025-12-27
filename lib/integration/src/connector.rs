//! Connector trait and related types.
//!
//! All integrations implement the Connector trait, providing a uniform
//! interface for external service operations.

use crate::error::ConnectorError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Information about a connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    /// Unique identifier for this connector type.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of the connector.
    pub description: String,
    /// Protocol used (e.g., "imap", "caldav", "rest").
    pub protocol: String,
    /// Available operations.
    pub operations: Vec<OperationInfo>,
    /// Capabilities of this connector.
    pub capabilities: Vec<ConnectorCapability>,
}

/// Information about an available operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationInfo {
    /// Operation name.
    pub name: String,
    /// Description of what the operation does.
    pub description: String,
    /// JSON schema for input parameters.
    pub input_schema: JsonValue,
    /// JSON schema for output.
    pub output_schema: JsonValue,
}

/// Capabilities that a connector may support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorCapability {
    /// Can read data from the service.
    Read,
    /// Can write/create data in the service.
    Write,
    /// Can update existing data.
    Update,
    /// Can delete data.
    Delete,
    /// Can subscribe to events.
    Subscribe,
    /// Supports OAuth authentication.
    Oauth,
    /// Supports API key authentication.
    ApiKey,
    /// Supports basic auth.
    BasicAuth,
}

/// An operation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// The operation name.
    pub name: String,
    /// Operation parameters.
    pub parameters: JsonValue,
}

impl Operation {
    /// Creates a new operation.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: JsonValue::Object(Default::default()),
        }
    }

    /// Adds a parameter.
    #[must_use]
    pub fn with_param(mut self, key: impl Into<String>, value: JsonValue) -> Self {
        if let JsonValue::Object(ref mut map) = self.parameters {
            map.insert(key.into(), value);
        }
        self
    }

    /// Sets all parameters at once.
    #[must_use]
    pub fn with_parameters(mut self, parameters: JsonValue) -> Self {
        self.parameters = parameters;
        self
    }
}

/// The result of an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Output data (if successful).
    pub data: Option<JsonValue>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Metadata about the operation.
    pub metadata: OperationMetadata,
}

/// Metadata about an operation execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationMetadata {
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Number of API calls made.
    pub api_calls: u32,
    /// Rate limit remaining (if applicable).
    pub rate_limit_remaining: Option<u32>,
}

impl OperationResult {
    /// Creates a successful result.
    #[must_use]
    pub fn success(data: JsonValue, metadata: OperationMetadata) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata,
        }
    }

    /// Creates a failed result.
    #[must_use]
    pub fn failure(error: impl Into<String>, metadata: OperationMetadata) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            metadata,
        }
    }
}

/// Trait for integration connectors.
///
/// All integrations must implement this trait to provide a uniform interface
/// for the workflow engine and conversation service.
#[async_trait]
pub trait Connector: Send + Sync {
    /// Returns information about this connector.
    fn info(&self) -> ConnectorInfo;

    /// Executes an operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    async fn execute(&self, operation: Operation) -> Result<OperationResult, ConnectorError>;

    /// Checks if the connection is healthy.
    async fn health_check(&self) -> Result<bool, ConnectorError>;

    /// Returns the list of supported capabilities.
    fn capabilities(&self) -> Vec<ConnectorCapability> {
        self.info().capabilities
    }

    /// Checks if this connector supports a specific capability.
    fn supports(&self, capability: ConnectorCapability) -> bool {
        self.capabilities().contains(&capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_builder() {
        let op = Operation::new("fetch_emails")
            .with_param("folder", serde_json::json!("inbox"))
            .with_param("limit", serde_json::json!(10));

        assert_eq!(op.name, "fetch_emails");
        if let JsonValue::Object(params) = op.parameters {
            assert_eq!(params.get("folder"), Some(&serde_json::json!("inbox")));
            assert_eq!(params.get("limit"), Some(&serde_json::json!(10)));
        } else {
            panic!("parameters should be an object");
        }
    }

    #[test]
    fn operation_result_success() {
        let result = OperationResult::success(
            serde_json::json!({"emails": []}),
            OperationMetadata {
                latency_ms: 150,
                api_calls: 1,
                rate_limit_remaining: Some(99),
            },
        );

        assert!(result.success);
        assert!(result.data.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn operation_result_failure() {
        let result = OperationResult::failure(
            "Connection timeout",
            OperationMetadata::default(),
        );

        assert!(!result.success);
        assert!(result.data.is_none());
        assert_eq!(result.error, Some("Connection timeout".to_string()));
    }

    #[test]
    fn connector_info_serde() {
        let info = ConnectorInfo {
            id: "email_imap".to_string(),
            name: "IMAP Email".to_string(),
            description: "Email access via IMAP protocol".to_string(),
            protocol: "imap".to_string(),
            operations: vec![],
            capabilities: vec![ConnectorCapability::Read, ConnectorCapability::Write],
        };

        let json = serde_json::to_string(&info).expect("serialize");
        let parsed: ConnectorInfo = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(info.id, parsed.id);
        assert_eq!(info.capabilities.len(), 2);
    }
}
