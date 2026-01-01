//! Role and permission types for platform access control.
//!
//! Access to the platform is controlled via OIDC groups. Users must have
//! a user-level group grant to access the platform. Users with an admin-level
//! group grant have additional capabilities.

use serde::{Deserialize, Serialize};

/// Platform access role derived from OIDC group membership.
///
/// The platform uses two levels of access:
/// - `User`: Standard access to personal data and workflows
/// - `Admin`: Additional capabilities for platform oversight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Standard user with access to their own data.
    User,
    /// Administrator with additional oversight capabilities.
    Admin,
}

impl Role {
    /// Returns true if this role has admin privileges.
    #[must_use]
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }
}

/// Set of roles assigned to a user from OIDC groups.
///
/// A user may have multiple roles. Having Admin implies having User access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleSet {
    roles: Vec<Role>,
}

impl RoleSet {
    /// Creates an empty role set (no access).
    #[must_use]
    pub fn none() -> Self {
        Self { roles: Vec::new() }
    }

    /// Creates a role set with user access only.
    #[must_use]
    pub fn user() -> Self {
        Self {
            roles: vec![Role::User],
        }
    }

    /// Creates a role set with admin access (implies user access).
    #[must_use]
    pub fn admin() -> Self {
        Self {
            roles: vec![Role::User, Role::Admin],
        }
    }

    /// Creates a role set from a list of OIDC group names.
    ///
    /// Maps OIDC group names to platform roles based on configuration.
    #[must_use]
    pub fn from_groups(groups: &[String], user_group: &str, admin_group: &str) -> Self {
        let mut roles = Vec::new();

        // Check for user access
        if groups.iter().any(|g| g == user_group) {
            roles.push(Role::User);
        }

        // Check for admin access (also grants user access if not already granted)
        if groups.iter().any(|g| g == admin_group) {
            if !roles.contains(&Role::User) {
                roles.push(Role::User);
            }
            roles.push(Role::Admin);
        }

        Self { roles }
    }

    /// Returns true if the user has any access to the platform.
    #[must_use]
    pub fn has_access(&self) -> bool {
        self.roles.contains(&Role::User)
    }

    /// Returns true if the user has admin access.
    #[must_use]
    pub fn is_admin(&self) -> bool {
        self.roles.contains(&Role::Admin)
    }

    /// Returns the roles as a slice.
    #[must_use]
    pub fn roles(&self) -> &[Role] {
        &self.roles
    }
}

impl Default for RoleSet {
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_is_admin() {
        assert!(!Role::User.is_admin());
        assert!(Role::Admin.is_admin());
    }

    #[test]
    fn role_set_none_has_no_access() {
        let roles = RoleSet::none();
        assert!(!roles.has_access());
        assert!(!roles.is_admin());
        assert!(roles.roles().is_empty());
    }

    #[test]
    fn role_set_user_has_access_not_admin() {
        let roles = RoleSet::user();
        assert!(roles.has_access());
        assert!(!roles.is_admin());
        assert_eq!(roles.roles(), &[Role::User]);
    }

    #[test]
    fn role_set_admin_has_both_roles() {
        let roles = RoleSet::admin();
        assert!(roles.has_access());
        assert!(roles.is_admin());
        assert!(roles.roles().contains(&Role::User));
        assert!(roles.roles().contains(&Role::Admin));
    }

    #[test]
    fn from_groups_no_matching_groups() {
        let groups = vec!["other-group".to_string(), "unrelated".to_string()];
        let roles = RoleSet::from_groups(&groups, "platform-users", "platform-admins");
        assert!(!roles.has_access());
        assert!(!roles.is_admin());
    }

    #[test]
    fn from_groups_user_group_only() {
        let groups = vec!["platform-users".to_string(), "other".to_string()];
        let roles = RoleSet::from_groups(&groups, "platform-users", "platform-admins");
        assert!(roles.has_access());
        assert!(!roles.is_admin());
    }

    #[test]
    fn from_groups_admin_group_only() {
        // Admin group should also grant user access
        let groups = vec!["platform-admins".to_string()];
        let roles = RoleSet::from_groups(&groups, "platform-users", "platform-admins");
        assert!(roles.has_access());
        assert!(roles.is_admin());
    }

    #[test]
    fn from_groups_both_groups() {
        let groups = vec!["platform-users".to_string(), "platform-admins".to_string()];
        let roles = RoleSet::from_groups(&groups, "platform-users", "platform-admins");
        assert!(roles.has_access());
        assert!(roles.is_admin());
        // Should not have duplicate User role
        assert_eq!(
            roles.roles().iter().filter(|r| **r == Role::User).count(),
            1
        );
    }

    #[test]
    fn role_set_serialization_roundtrip() {
        let roles = RoleSet::admin();
        let json = serde_json::to_string(&roles).expect("serialize");
        let parsed: RoleSet = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roles, parsed);
    }

    #[test]
    fn role_serialization_format() {
        let json = serde_json::to_string(&Role::Admin).expect("serialize");
        assert_eq!(json, "\"admin\"");

        let json = serde_json::to_string(&Role::User).expect("serialize");
        assert_eq!(json, "\"user\"");
    }
}
