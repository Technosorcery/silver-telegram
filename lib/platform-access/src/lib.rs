//! Platform access, authentication, and authorization for silver-telegram.
//!
//! This crate provides:
//! - User management (`User` type with OIDC integration)
//! - Role-based access control (`Role`, `RoleSet`)
//! - Session management (`Session`, `SessionId`)
//! - Authentication error types
//!
//! # Access Control Model
//!
//! Access to the platform is controlled via OIDC groups:
//! - Users must have a user-level group grant to access the platform
//! - Users with an admin-level group grant have additional oversight capabilities
//!
//! # Example
//!
//! ```
//! use silver_telegram_platform_access::{User, RoleSet, Session, SessionId};
//! use chrono::Duration;
//!
//! // Create a user after OIDC authentication
//! let mut user = User::new(
//!     "auth0|123456".to_string(),
//!     "https://example.auth0.com/".to_string(),
//! );
//! user.set_email(Some("alice@example.com".to_string()));
//! user.set_timezone(Some("America/New_York".to_string()));
//!
//! // Derive roles from OIDC groups
//! let groups = vec!["platform-users".to_string(), "platform-admins".to_string()];
//! let roles = RoleSet::from_groups(&groups, "platform-users", "platform-admins");
//!
//! // Create a session
//! let session = Session::new(
//!     SessionId::new("sess_abc123".to_string()),
//!     user.id(),
//!     roles,
//!     Duration::hours(8),
//! );
//!
//! assert!(session.has_access());
//! assert!(session.is_admin());
//! ```

pub mod auth;
pub mod error;
pub mod oidc;
pub mod role;
pub mod session;
pub mod user;

// Re-export main types at crate root
pub use auth::{
    AuthResult, AuthenticatedUser, CallbackData, CallbackResult, LoginInitiation, OidcClaims,
};
pub use error::{AuthenticationError, AuthorizationError};
pub use oidc::{OidcConfig, OidcConfigBuilder};
pub use role::{Role, RoleSet};
pub use session::{Session, SessionId};
pub use user::User;
