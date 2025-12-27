//! Context store for conversation facts and history.
//!
//! Per PRD 8.1:
//! - Conversation history: 90 days rolling
//! - Facts: Until contradicted or manually deleted
//! - Corrections/feedback: Permanent
//! - Workflow execution history: User-configurable (default 90 days)

use crate::error::ContextError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use silver_telegram_core::{ConversationSessionId, UserId};
use ulid::Ulid;

/// Unique identifier for a context fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FactId(Ulid);

impl FactId {
    /// Creates a new fact ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for FactId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fact_{}", self.0)
    }
}

/// The source of a fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactSource {
    /// User explicitly stated this fact.
    Explicit,
    /// System inferred this fact from conversation.
    Inferred,
    /// Fact was extracted from a workflow execution.
    WorkflowExtracted,
}

/// A fact extracted from conversation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFact {
    /// Unique identifier.
    pub id: FactId,
    /// The user this fact belongs to.
    pub user_id: UserId,
    /// Fact category (e.g., "preference", "identity", "constraint").
    pub category: String,
    /// The fact key (e.g., "timezone", "name", "dietary_restrictions").
    pub key: String,
    /// The fact value.
    pub value: JsonValue,
    /// How this fact was learned.
    pub source: FactSource,
    /// The session where this fact was learned.
    pub source_session_id: Option<ConversationSessionId>,
    /// When this fact was created.
    pub created_at: DateTime<Utc>,
    /// When this fact was last updated.
    pub updated_at: DateTime<Utc>,
    /// Whether this fact is part of the "core" context (always included).
    pub is_core: bool,
    /// Confidence level (0.0 - 1.0) for inferred facts.
    pub confidence: Option<f64>,
}

impl ContextFact {
    /// Creates a new explicit fact.
    #[must_use]
    pub fn explicit(
        user_id: UserId,
        category: impl Into<String>,
        key: impl Into<String>,
        value: JsonValue,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: FactId::new(),
            user_id,
            category: category.into(),
            key: key.into(),
            value,
            source: FactSource::Explicit,
            source_session_id: None,
            created_at: now,
            updated_at: now,
            is_core: true, // Explicit facts are core by default
            confidence: None,
        }
    }

    /// Creates a new inferred fact.
    #[must_use]
    pub fn inferred(
        user_id: UserId,
        category: impl Into<String>,
        key: impl Into<String>,
        value: JsonValue,
        confidence: f64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: FactId::new(),
            user_id,
            category: category.into(),
            key: key.into(),
            value,
            source: FactSource::Inferred,
            source_session_id: None,
            created_at: now,
            updated_at: now,
            is_core: false, // Inferred facts are not core by default
            confidence: Some(confidence),
        }
    }

    /// Sets the source session.
    #[must_use]
    pub fn with_source_session(mut self, session_id: ConversationSessionId) -> Self {
        self.source_session_id = Some(session_id);
        self
    }

    /// Marks this fact as core.
    #[must_use]
    pub fn as_core(mut self) -> Self {
        self.is_core = true;
        self
    }

    /// Updates the fact value.
    pub fn update_value(&mut self, value: JsonValue) {
        self.value = value;
        self.updated_at = Utc::now();
    }
}

/// Query parameters for retrieving facts.
#[derive(Debug, Clone, Default)]
pub struct FactQuery {
    /// Filter by user.
    pub user_id: Option<UserId>,
    /// Filter by category.
    pub category: Option<String>,
    /// Filter by key pattern.
    pub key_pattern: Option<String>,
    /// Include only core facts.
    pub core_only: bool,
    /// Include only explicit facts.
    pub explicit_only: bool,
    /// Minimum confidence for inferred facts.
    pub min_confidence: Option<f64>,
}

impl FactQuery {
    /// Creates a query for all facts of a user.
    #[must_use]
    pub fn for_user(user_id: UserId) -> Self {
        Self {
            user_id: Some(user_id),
            ..Default::default()
        }
    }

    /// Filters by category.
    #[must_use]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Filters to core facts only.
    #[must_use]
    pub fn core_only(mut self) -> Self {
        self.core_only = true;
        self
    }
}

/// Trait for context storage.
pub trait ContextStore: Send + Sync {
    /// Stores or updates a fact.
    fn store_fact(
        &self,
        fact: ContextFact,
    ) -> impl std::future::Future<Output = Result<FactId, ContextError>> + Send;

    /// Gets a fact by ID.
    fn get_fact(
        &self,
        id: FactId,
    ) -> impl std::future::Future<Output = Result<ContextFact, ContextError>> + Send;

    /// Queries facts.
    fn query_facts(
        &self,
        query: FactQuery,
    ) -> impl std::future::Future<Output = Result<Vec<ContextFact>, ContextError>> + Send;

    /// Deletes a fact.
    fn delete_fact(
        &self,
        id: FactId,
    ) -> impl std::future::Future<Output = Result<(), ContextError>> + Send;

    /// Gets all core facts for a user (for context injection).
    fn get_core_facts(
        &self,
        user_id: UserId,
    ) -> impl std::future::Future<Output = Result<Vec<ContextFact>, ContextError>> + Send;

    /// Searches facts by semantic similarity (for retrieval).
    fn search_facts(
        &self,
        user_id: UserId,
        query: &str,
        limit: usize,
    ) -> impl std::future::Future<Output = Result<Vec<ContextFact>, ContextError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_fact_creation() {
        let user_id = UserId::new();
        let fact = ContextFact::explicit(
            user_id,
            "preference",
            "timezone",
            serde_json::json!("America/New_York"),
        );

        assert_eq!(fact.source, FactSource::Explicit);
        assert!(fact.is_core);
        assert!(fact.confidence.is_none());
    }

    #[test]
    fn inferred_fact_creation() {
        let user_id = UserId::new();
        let fact = ContextFact::inferred(
            user_id,
            "preference",
            "coffee_preference",
            serde_json::json!("black"),
            0.75,
        );

        assert_eq!(fact.source, FactSource::Inferred);
        assert!(!fact.is_core);
        assert_eq!(fact.confidence, Some(0.75));
    }

    #[test]
    fn fact_update() {
        let user_id = UserId::new();
        let mut fact = ContextFact::explicit(
            user_id,
            "preference",
            "color",
            serde_json::json!("blue"),
        );

        let original_updated = fact.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        fact.update_value(serde_json::json!("green"));

        assert_eq!(fact.value, serde_json::json!("green"));
        assert!(fact.updated_at > original_updated);
    }

    #[test]
    fn fact_query_builder() {
        let user_id = UserId::new();
        let query = FactQuery::for_user(user_id)
            .with_category("preference")
            .core_only();

        assert_eq!(query.user_id, Some(user_id));
        assert_eq!(query.category, Some("preference".to_string()));
        assert!(query.core_only);
    }

    #[test]
    fn fact_serde_roundtrip() {
        let fact = ContextFact::explicit(
            UserId::new(),
            "identity",
            "name",
            serde_json::json!("Alice"),
        );

        let json = serde_json::to_string(&fact).expect("serialize");
        let parsed: ContextFact = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(fact.id, parsed.id);
        assert_eq!(fact.key, parsed.key);
    }
}
