//! SpiceDB client for authorization operations.

use crate::error::AuthzError;
use crate::types::{Permission, Relationship, Resource, ResourceType, Subject};
use rootcause::prelude::Report;
use spicedb_client::SpicedbClient;
use spicedb_grpc::authzed::api::v1::{
    CheckPermissionRequest, Consistency, DeleteRelationshipsRequest, LookupResourcesRequest,
    ObjectReference, RelationshipFilter, RelationshipUpdate, SubjectReference,
    WriteRelationshipsRequest, check_permission_response::Permissionship, relationship_update,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

/// SpiceDB authorization client wrapper.
///
/// This wrapper handles the lifetime constraints of the underlying SpiceDB client
/// by maintaining a persistent connection that's protected by a mutex.
#[derive(Clone)]
pub struct AuthzClient {
    inner: Arc<Mutex<SpicedbClient>>,
}

impl AuthzClient {
    /// Creates a new authorization client.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The SpiceDB gRPC endpoint (e.g., "http://localhost:50051")
    /// * `preshared_key` - The preshared key for authentication
    ///
    /// Note: The endpoint and preshared_key are leaked to satisfy the 'static
    /// lifetime requirements of the underlying gRPC client. This is intentional
    /// as the authorization client is expected to live for the duration of the app.
    pub async fn new(endpoint: String, preshared_key: String) -> Result<Self, Report<AuthzError>> {
        // Leak the strings to get 'static references
        // This is safe because AuthzClient is meant to be long-lived
        let endpoint: &'static str = Box::leak(endpoint.into_boxed_str());
        let preshared_key: &'static str = Box::leak(preshared_key.into_boxed_str());

        let client = SpicedbClient::from_url_and_preshared_key(endpoint, preshared_key)
            .await
            .map_err(|e| AuthzError::ConnectionFailed {
                details: e.to_string(),
            })?;

        Ok(Self {
            inner: Arc::new(Mutex::new(client)),
        })
    }

    /// Checks if a subject has a permission on a resource.
    #[instrument(skip(self), fields(resource = %resource.resource_type, permission = %permission))]
    pub async fn check_permission(
        &self,
        resource: &Resource,
        permission: Permission,
        subject: &Subject,
    ) -> Result<bool, Report<AuthzError>> {
        let request = CheckPermissionRequest {
            resource: Some(ObjectReference {
                object_type: resource.resource_type.as_str().to_string(),
                object_id: resource.id.clone(),
            }),
            permission: permission.as_str().to_string(),
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: subject.subject_type.clone(),
                    object_id: subject.id.clone(),
                }),
                optional_relation: String::new(),
            }),
            consistency: Some(Consistency {
                requirement: Some(
                    spicedb_grpc::authzed::api::v1::consistency::Requirement::FullyConsistent(true),
                ),
            }),
            ..Default::default()
        };

        let mut client = self.inner.lock().await;
        let response =
            client
                .check_permission(request)
                .await
                .map_err(|e| AuthzError::RequestFailed {
                    details: e.to_string(),
                })?;

        let permissionship = response.permissionship();
        let has_permission = permissionship == Permissionship::HasPermission;

        debug!(has_permission, "permission check result");

        Ok(has_permission)
    }

    /// Checks permission and returns an error if denied.
    pub async fn require_permission(
        &self,
        resource: &Resource,
        permission: Permission,
        subject: &Subject,
    ) -> Result<(), Report<AuthzError>> {
        let allowed = self.check_permission(resource, permission, subject).await?;
        if !allowed {
            return Err(AuthzError::PermissionDenied {
                resource: format!("{}:{}", resource.resource_type, resource.id),
                permission: permission.to_string(),
            }
            .into());
        }
        Ok(())
    }

    /// Writes a relationship to SpiceDB.
    #[instrument(skip(self), fields(resource = %relationship.resource.resource_type, relation = %relationship.relation))]
    pub async fn write_relationship(
        &self,
        relationship: &Relationship,
    ) -> Result<(), Report<AuthzError>> {
        let update = RelationshipUpdate {
            operation: relationship_update::Operation::Touch as i32,
            relationship: Some(spicedb_grpc::authzed::api::v1::Relationship {
                resource: Some(ObjectReference {
                    object_type: relationship.resource.resource_type.as_str().to_string(),
                    object_id: relationship.resource.id.clone(),
                }),
                relation: relationship.relation.clone(),
                subject: Some(SubjectReference {
                    object: Some(ObjectReference {
                        object_type: relationship.subject.subject_type.clone(),
                        object_id: relationship.subject.id.clone(),
                    }),
                    optional_relation: String::new(),
                }),
                optional_caveat: None,
            }),
        };

        let request = WriteRelationshipsRequest {
            updates: vec![update],
            ..Default::default()
        };

        let mut client = self.inner.lock().await;
        client
            .write_relationships(request)
            .await
            .map_err(|e| AuthzError::RequestFailed {
                details: e.to_string(),
            })?;

        debug!("relationship written");
        Ok(())
    }

    /// Deletes relationships matching a filter.
    #[instrument(skip(self), fields(resource = %resource.resource_type))]
    pub async fn delete_relationships(
        &self,
        resource: &Resource,
        relation: Option<&str>,
    ) -> Result<(), Report<AuthzError>> {
        let request = DeleteRelationshipsRequest {
            relationship_filter: Some(RelationshipFilter {
                resource_type: resource.resource_type.as_str().to_string(),
                optional_resource_id: resource.id.clone(),
                optional_relation: relation.unwrap_or_default().to_string(),
                optional_subject_filter: None,
                optional_resource_id_prefix: String::new(),
            }),
            ..Default::default()
        };

        let mut client = self.inner.lock().await;
        client
            .delete_relationships(request)
            .await
            .map_err(|e| AuthzError::RequestFailed {
                details: e.to_string(),
            })?;

        debug!("relationships deleted");
        Ok(())
    }

    /// Writes the authorization schema to SpiceDB.
    ///
    /// This should be called on server startup to ensure the schema is loaded.
    #[instrument(skip(self, schema))]
    pub async fn write_schema(&self, schema: &str) -> Result<(), Report<AuthzError>> {
        let mut client = self.inner.lock().await;
        client
            .write_schema(schema)
            .await
            .map_err(|e| AuthzError::RequestFailed {
                details: e.to_string(),
            })?;

        debug!("schema written");
        Ok(())
    }

    /// Looks up resource IDs that a subject has a permission on.
    #[instrument(skip(self), fields(resource_type = %resource_type, permission = %permission))]
    pub async fn lookup_resources(
        &self,
        resource_type: ResourceType,
        permission: Permission,
        subject: &Subject,
    ) -> Result<Vec<String>, Report<AuthzError>> {
        use tokio_stream::StreamExt;

        let request = LookupResourcesRequest {
            resource_object_type: resource_type.as_str().to_string(),
            permission: permission.as_str().to_string(),
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: subject.subject_type.clone(),
                    object_id: subject.id.clone(),
                }),
                optional_relation: String::new(),
            }),
            consistency: Some(Consistency {
                requirement: Some(
                    spicedb_grpc::authzed::api::v1::consistency::Requirement::FullyConsistent(true),
                ),
            }),
            ..Default::default()
        };

        let mut client = self.inner.lock().await;
        let mut response =
            client
                .lookup_resources(request)
                .await
                .map_err(|e| AuthzError::RequestFailed {
                    details: e.to_string(),
                })?;

        // Collect all resource IDs from the streaming response
        let mut ids = Vec::new();
        while let Some(result) = response.next().await {
            match result {
                Ok(r) => ids.push(r.resource_object_id),
                Err(e) => {
                    return Err(AuthzError::RequestFailed {
                        details: e.to_string(),
                    }
                    .into());
                }
            }
        }

        debug!(count = ids.len(), "lookup resources result");
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use silver_telegram_core::{UserId, WorkflowId};

    #[test]
    fn test_resource_creation() {
        let wf_id = WorkflowId::new();
        let resource = Resource::workflow(wf_id);
        assert_eq!(resource.resource_type.as_str(), "workflow");
    }

    #[test]
    fn test_relationship_creation() {
        let wf_id = WorkflowId::new();
        let user_id = UserId::new();
        let rel = Relationship::workflow_owner(wf_id, user_id);
        assert_eq!(rel.relation, "owner");
    }
}
