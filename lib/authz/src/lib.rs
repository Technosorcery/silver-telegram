//! SpiceDB authorization client for silver-telegram.
//!
//! This crate provides relationship-based authorization following ADR-002.
//! Resources don't have user_id columns; authorization flows through SpiceDB relationships.

mod client;
mod error;
mod types;

pub use client::AuthzClient;
pub use error::AuthzError;
pub use types::{Permission, Relationship, Resource, ResourceType, Subject};
