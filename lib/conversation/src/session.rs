//! Conversation session management.
//!
//! Sessions track active conversations, maintaining message history
//! and extracted context.

use crate::error::SessionError;
use crate::message::Message;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::{ConversationSessionId, UserId};

/// The state of a conversation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Session is active and accepting messages.
    Active,
    /// Session is in workflow authoring mode.
    Authoring,
    /// Session has ended.
    Ended,
}

impl SessionState {
    /// Returns true if the session can accept messages.
    #[must_use]
    pub fn can_accept_messages(&self) -> bool {
        matches!(self, Self::Active | Self::Authoring)
    }

    /// Returns true if the session has ended.
    #[must_use]
    pub fn is_ended(&self) -> bool {
        matches!(self, Self::Ended)
    }
}

/// A conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier.
    pub id: ConversationSessionId,
    /// The user who owns this session.
    pub user_id: UserId,
    /// Session state.
    pub state: SessionState,
    /// Messages in this session.
    pub messages: Vec<Message>,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session was last active.
    pub last_active_at: DateTime<Utc>,
    /// Session metadata.
    pub metadata: SessionMetadata,
}

/// Session metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Optional session title (generated from first message).
    pub title: Option<String>,
}

impl Session {
    /// Creates a new session for a user.
    #[must_use]
    pub fn new(user_id: UserId) -> Self {
        let now = Utc::now();
        Self {
            id: ConversationSessionId::new(),
            user_id,
            state: SessionState::Active,
            messages: Vec::new(),
            created_at: now,
            last_active_at: now,
            metadata: SessionMetadata::default(),
        }
    }

    /// Adds a message to the session.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.last_active_at = Utc::now();
    }

    /// Enters authoring mode.
    pub fn enter_authoring_mode(&mut self) {
        self.state = SessionState::Authoring;
        self.last_active_at = Utc::now();
    }

    /// Exits authoring mode.
    pub fn exit_authoring_mode(&mut self) {
        self.state = SessionState::Active;
        self.last_active_at = Utc::now();
    }

    /// Ends the session.
    pub fn end(&mut self) {
        self.state = SessionState::Ended;
    }

    /// Returns the number of messages.
    #[must_use]
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Returns the last message, if any.
    #[must_use]
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Returns messages within a time window.
    pub fn messages_since(&self, since: DateTime<Utc>) -> impl Iterator<Item = &Message> {
        self.messages.iter().filter(move |m| m.timestamp >= since)
    }

    /// Generates a title from the first user message.
    pub fn generate_title(&mut self) {
        if self.metadata.title.is_some() {
            return;
        }

        // Find first user message
        let first_user_msg = self
            .messages
            .iter()
            .find(|m| m.role == crate::message::MessageRole::User);

        if let Some(msg) = first_user_msg {
            // Take first 50 chars as title
            let title = if msg.content.len() > 50 {
                format!("{}...", &msg.content[..47])
            } else {
                msg.content.clone()
            };
            self.metadata.title = Some(title);
        }
    }
}

/// Trait for session storage.
#[async_trait]
pub trait SessionManager: Send + Sync {
    /// Creates a new session.
    async fn create_session(&self, user_id: UserId) -> Result<Session, SessionError>;

    /// Gets a session by ID.
    async fn get_session(&self, id: ConversationSessionId) -> Result<Session, SessionError>;

    /// Updates a session.
    async fn update_session(&self, session: Session) -> Result<(), SessionError>;

    /// Lists sessions for a user.
    async fn list_sessions(
        &self,
        user_id: UserId,
        include_ended: bool,
    ) -> Result<Vec<Session>, SessionError>;

    /// Deletes a session.
    async fn delete_session(&self, id: ConversationSessionId) -> Result<(), SessionError>;

    /// Gets expired sessions (for cleanup).
    async fn get_expired_sessions(
        &self,
        older_than: DateTime<Utc>,
    ) -> Result<Vec<ConversationSessionId>, SessionError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageRole;

    #[test]
    fn session_creation() {
        let user_id = UserId::new();
        let session = Session::new(user_id);

        assert_eq!(session.user_id, user_id);
        assert_eq!(session.state, SessionState::Active);
        assert!(session.messages.is_empty());
    }

    #[test]
    fn session_add_message() {
        let mut session = Session::new(UserId::new());
        let message = Message::new(MessageRole::User, "Hello!");

        session.add_message(message);

        assert_eq!(session.message_count(), 1);
        assert_eq!(session.last_message().unwrap().content, "Hello!");
    }

    #[test]
    fn session_authoring_mode() {
        let mut session = Session::new(UserId::new());
        assert!(session.state.can_accept_messages());

        session.enter_authoring_mode();
        assert_eq!(session.state, SessionState::Authoring);
        assert!(session.state.can_accept_messages());

        session.exit_authoring_mode();
        assert_eq!(session.state, SessionState::Active);
    }

    #[test]
    fn session_end() {
        let mut session = Session::new(UserId::new());
        session.end();

        assert!(session.state.is_ended());
        assert!(!session.state.can_accept_messages());
    }

    #[test]
    fn session_generate_title() {
        let mut session = Session::new(UserId::new());
        session.add_message(Message::new(MessageRole::User, "What's the weather today?"));
        session.add_message(Message::new(
            MessageRole::Assistant,
            "I can help with that.",
        ));

        session.generate_title();

        assert_eq!(
            session.metadata.title,
            Some("What's the weather today?".to_string())
        );
    }

    #[test]
    fn session_serde_roundtrip() {
        let mut session = Session::new(UserId::new());
        session.add_message(Message::new(MessageRole::User, "Test"));

        let json = serde_json::to_string(&session).expect("serialize");
        let parsed: Session = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(session.id, parsed.id);
        assert_eq!(session.message_count(), parsed.message_count());
    }
}
