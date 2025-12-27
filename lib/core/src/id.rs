//! Strongly-typed ID types for domain entities.
//!
//! All IDs use ULID (Universally Unique Lexicographically Sortable Identifier) format,
//! providing both uniqueness and temporal ordering.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use ulid::Ulid;

/// Error returned when parsing an ID from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIdError {
    /// The type of ID that failed to parse.
    pub id_type: &'static str,
    /// The reason for the parse failure.
    pub reason: String,
}

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to parse {}: {}", self.id_type, self.reason)
    }
}

impl std::error::Error for ParseIdError {}

/// Macro to generate a strongly-typed ID wrapper around ULID.
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $prefix:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Ulid);

        impl $name {
            /// Creates a new ID with a randomly generated ULID.
            #[must_use]
            pub fn new() -> Self {
                Self(Ulid::new())
            }

            /// Creates an ID from a ULID.
            #[must_use]
            pub const fn from_ulid(ulid: Ulid) -> Self {
                Self(ulid)
            }

            /// Returns the underlying ULID.
            #[must_use]
            pub const fn as_ulid(&self) -> Ulid {
                self.0
            }

            /// Returns the prefix used for display formatting.
            #[must_use]
            pub const fn prefix() -> &'static str {
                $prefix
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}_{}", $prefix, self.0)
            }
        }

        impl FromStr for $name {
            type Err = ParseIdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                // Try with prefix first
                let prefix_with_underscore = concat!($prefix, "_");
                let ulid_str = if let Some(stripped) = s.strip_prefix(prefix_with_underscore) {
                    stripped
                } else {
                    // Try parsing as raw ULID
                    s
                };

                Ulid::from_str(ulid_str)
                    .map(Self)
                    .map_err(|e| ParseIdError {
                        id_type: stringify!($name),
                        reason: e.to_string(),
                    })
            }
        }

        impl From<Ulid> for $name {
            fn from(ulid: Ulid) -> Self {
                Self(ulid)
            }
        }

        impl From<$name> for Ulid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

define_id!(
    /// Unique identifier for a user.
    UserId,
    "usr"
);

define_id!(
    /// Unique identifier for a workflow definition.
    WorkflowId,
    "wf"
);

define_id!(
    /// Unique identifier for a single execution (run) of a workflow.
    WorkflowRunId,
    "run"
);

define_id!(
    /// Unique identifier for a conversation session.
    ConversationSessionId,
    "sess"
);

define_id!(
    /// Unique identifier for a message within a conversation.
    MessageId,
    "msg"
);

define_id!(
    /// Unique identifier for an integration account.
    IntegrationAccountId,
    "int"
);

define_id!(
    /// Unique identifier for a stored credential.
    CredentialId,
    "cred"
);

define_id!(
    /// Unique identifier for a trigger.
    TriggerId,
    "trg"
);

define_id!(
    /// Unique identifier for a node execution record within a workflow run.
    NodeExecutionId,
    "nexec"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_id_display_format() {
        let id = UserId::new();
        let display = id.to_string();
        assert!(display.starts_with("usr_"));
    }

    #[test]
    fn workflow_id_display_format() {
        let id = WorkflowId::new();
        let display = id.to_string();
        assert!(display.starts_with("wf_"));
    }

    #[test]
    fn parse_with_prefix() {
        let id = WorkflowId::new();
        let display = id.to_string();
        let parsed: WorkflowId = display.parse().expect("should parse");
        assert_eq!(id, parsed);
    }

    #[test]
    fn parse_without_prefix() {
        let ulid = Ulid::new();
        let id: WorkflowId = ulid.to_string().parse().expect("should parse");
        assert_eq!(id.as_ulid(), ulid);
    }

    #[test]
    fn parse_invalid_ulid() {
        let result: Result<WorkflowId, _> = "not_a_ulid".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.id_type, "WorkflowId");
    }

    #[test]
    fn id_equality() {
        let ulid = Ulid::new();
        let id1 = UserId::from_ulid(ulid);
        let id2 = UserId::from_ulid(ulid);
        assert_eq!(id1, id2);
    }

    #[test]
    fn id_hash() {
        use std::collections::HashSet;

        let id1 = WorkflowId::new();
        let id2 = WorkflowId::new();

        let mut set = HashSet::new();
        set.insert(id1);
        set.insert(id2);
        set.insert(id1); // duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn id_serde_roundtrip() {
        let id = WorkflowRunId::new();
        let json = serde_json::to_string(&id).expect("serialize");
        let parsed: WorkflowRunId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, parsed);
    }
}
