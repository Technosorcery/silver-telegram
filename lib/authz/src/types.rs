//! Authorization types for SpiceDB integration.

use silver_telegram_core::{IntegrationAccountId, UserId, WorkflowId};
use std::fmt;

/// Resource types in the authorization model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// A workflow resource.
    Workflow,
    /// An integration account resource.
    Integration,
    /// The platform itself (for admin permissions).
    Platform,
}

impl ResourceType {
    /// Returns the SpiceDB type name.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::Integration => "integration",
            Self::Platform => "platform",
        }
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A resource in the authorization model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    /// The type of resource.
    pub resource_type: ResourceType,
    /// The resource ID.
    pub id: String,
}

impl Resource {
    /// Creates a new resource.
    #[must_use]
    pub fn new(resource_type: ResourceType, id: impl Into<String>) -> Self {
        Self {
            resource_type,
            id: id.into(),
        }
    }

    /// Creates a workflow resource.
    #[must_use]
    pub fn workflow(id: WorkflowId) -> Self {
        Self::new(ResourceType::Workflow, id.to_string())
    }

    /// Creates an integration resource.
    #[must_use]
    pub fn integration(id: IntegrationAccountId) -> Self {
        Self::new(ResourceType::Integration, id.to_string())
    }

    /// Creates a platform resource.
    #[must_use]
    pub fn platform() -> Self {
        Self::new(ResourceType::Platform, "main")
    }
}

/// A subject (actor) in the authorization model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    /// Subject type (always "user" for now).
    pub subject_type: String,
    /// Subject ID.
    pub id: String,
}

impl Subject {
    /// Creates a new user subject.
    #[must_use]
    pub fn user(id: UserId) -> Self {
        Self {
            subject_type: "user".to_string(),
            id: id.to_string(),
        }
    }
}

/// A relationship between a resource and a subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    /// The resource.
    pub resource: Resource,
    /// The relation name (e.g., "owner", "admin").
    pub relation: String,
    /// The subject.
    pub subject: Subject,
}

impl Relationship {
    /// Creates a new relationship.
    #[must_use]
    pub fn new(resource: Resource, relation: impl Into<String>, subject: Subject) -> Self {
        Self {
            resource,
            relation: relation.into(),
            subject,
        }
    }

    /// Creates an owner relationship for a workflow.
    #[must_use]
    pub fn workflow_owner(workflow_id: WorkflowId, user_id: UserId) -> Self {
        Self::new(
            Resource::workflow(workflow_id),
            "owner",
            Subject::user(user_id),
        )
    }

    /// Creates an owner relationship for an integration.
    #[must_use]
    pub fn integration_owner(integration_id: IntegrationAccountId, user_id: UserId) -> Self {
        Self::new(
            Resource::integration(integration_id),
            "owner",
            Subject::user(user_id),
        )
    }

    /// Creates an admin relationship for the platform.
    #[must_use]
    pub fn platform_admin(user_id: UserId) -> Self {
        Self::new(Resource::platform(), "admin", Subject::user(user_id))
    }

    /// Creates a member relationship for the platform.
    #[must_use]
    pub fn platform_member(user_id: UserId) -> Self {
        Self::new(Resource::platform(), "member", Subject::user(user_id))
    }
}

/// Permission to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// View a resource.
    View,
    /// Edit a resource.
    Edit,
    /// Delete a resource.
    Delete,
    /// Execute a workflow.
    Execute,
    /// Use an integration.
    Use,
    /// Administer the platform.
    Administer,
    /// Access the platform.
    Access,
}

impl Permission {
    /// Returns the SpiceDB permission name.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::View => "view",
            Self::Edit => "edit",
            Self::Delete => "delete",
            Self::Execute => "execute",
            Self::Use => "use",
            Self::Administer => "administer",
            Self::Access => "access",
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
