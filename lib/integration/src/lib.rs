//! Integration framework for the silver-telegram platform.
//!
//! This crate provides:
//!
//! - **Connector trait**: Common interface for all integrations
//! - **Credential vault**: Encrypted storage for integration credentials
//! - **Rate limiter**: Per-integration rate limiting

pub mod connector;
pub mod credential;
pub mod rate_limit;

pub use connector::{Connector, ConnectorCapability, ConnectorInfo, Operation, OperationResult};
pub use credential::{Credential, CredentialData, CredentialVault};
pub use rate_limit::{RateLimitConfig, RateLimiter};
