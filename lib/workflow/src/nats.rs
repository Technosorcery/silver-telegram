//! NATS integration for workflow execution.
//!
//! Per ADR-006:
//! - Event sourcing via NATS JetStream
//! - Node outputs stored in NATS Object Store
//! - Job queue semantics for orchestrator assignment
//!
//! This module provides NATS-backed implementations of:
//! - `EventStore`: JetStream-based event persistence
//! - `ObjectStore`: NATS Object Store for node outputs

use crate::envelope::Envelope;
use crate::execution::ExecutionEvent;
use crate::orchestrator::{EventStore, EventStoreError, WorkItem};
use crate::worker::{ObjectStore, ObjectStoreError};
use async_nats::jetstream;
use async_nats::jetstream::object_store;
use async_trait::async_trait;
use silver_telegram_core::WorkflowRunId;
use std::sync::Arc;
use ulid::Ulid;

/// Subject prefix for workflow run events.
const RUN_EVENTS_SUBJECT_PREFIX: &str = "workflow.run";

/// Subject for work items.
const WORK_ITEMS_SUBJECT: &str = "workflow.work";

/// Stream name for workflow events.
const EVENTS_STREAM_NAME: &str = "WORKFLOW_EVENTS";

/// Stream name for work items.
const WORK_STREAM_NAME: &str = "WORKFLOW_WORK";

/// Object store bucket name for node outputs.
const OUTPUTS_BUCKET_NAME: &str = "workflow-outputs";

/// Configuration for NATS-based workflow execution.
#[derive(Debug, Clone)]
pub struct NatsConfig {
    /// NATS server URL.
    pub url: String,
    /// Stream name for events (defaults to WORKFLOW_EVENTS).
    pub events_stream_name: Option<String>,
    /// Stream name for work items (defaults to WORKFLOW_WORK).
    pub work_stream_name: Option<String>,
    /// Object store bucket name (defaults to workflow-outputs).
    pub outputs_bucket_name: Option<String>,
}

impl NatsConfig {
    /// Creates a new config with the given NATS URL.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            events_stream_name: None,
            work_stream_name: None,
            outputs_bucket_name: None,
        }
    }

    fn events_stream(&self) -> &str {
        self.events_stream_name
            .as_deref()
            .unwrap_or(EVENTS_STREAM_NAME)
    }

    fn work_stream(&self) -> &str {
        self.work_stream_name.as_deref().unwrap_or(WORK_STREAM_NAME)
    }

    fn outputs_bucket(&self) -> &str {
        self.outputs_bucket_name
            .as_deref()
            .unwrap_or(OUTPUTS_BUCKET_NAME)
    }
}

/// NATS JetStream-based event store.
///
/// Events are published to subjects like `workflow.run.<run_id>`.
/// Each run has its own subject for easy replay.
pub struct NatsEventStore {
    jetstream: Arc<jetstream::Context>,
    config: NatsConfig,
}

impl NatsEventStore {
    /// Creates a new NATS event store.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or stream setup fails.
    pub async fn new(config: NatsConfig) -> Result<Self, EventStoreError> {
        let client = async_nats::connect(&config.url).await.map_err(|e| {
            EventStoreError::ConnectionFailed {
                message: e.to_string(),
            }
        })?;

        let jetstream = async_nats::jetstream::new(client);

        // Ensure streams exist
        Self::ensure_streams(&jetstream, &config).await?;

        Ok(Self {
            jetstream: Arc::new(jetstream),
            config,
        })
    }

    /// Ensures the required streams exist.
    async fn ensure_streams(
        jetstream: &jetstream::Context,
        config: &NatsConfig,
    ) -> Result<(), EventStoreError> {
        // Events stream
        let events_stream_config = jetstream::stream::Config {
            name: config.events_stream().to_string(),
            subjects: vec![format!("{RUN_EVENTS_SUBJECT_PREFIX}.>")],
            storage: jetstream::stream::StorageType::File,
            retention: jetstream::stream::RetentionPolicy::Limits,
            ..Default::default()
        };

        jetstream
            .get_or_create_stream(events_stream_config)
            .await
            .map_err(|e| EventStoreError::ConnectionFailed {
                message: format!("failed to create events stream: {e}"),
            })?;

        // Work stream
        let work_stream_config = jetstream::stream::Config {
            name: config.work_stream().to_string(),
            subjects: vec![format!("{WORK_ITEMS_SUBJECT}.>")],
            storage: jetstream::stream::StorageType::File,
            retention: jetstream::stream::RetentionPolicy::WorkQueue,
            ..Default::default()
        };

        jetstream
            .get_or_create_stream(work_stream_config)
            .await
            .map_err(|e| EventStoreError::ConnectionFailed {
                message: format!("failed to create work stream: {e}"),
            })?;

        Ok(())
    }

    /// Returns the subject for a run's events.
    fn run_subject(run_id: WorkflowRunId) -> String {
        format!("{RUN_EVENTS_SUBJECT_PREFIX}.{run_id}")
    }

    /// Returns the subject for work items.
    fn work_subject() -> String {
        format!("{WORK_ITEMS_SUBJECT}.items")
    }
}

#[async_trait]
impl EventStore for NatsEventStore {
    async fn publish(&self, event: Envelope<ExecutionEvent>) -> Result<(), EventStoreError> {
        let subject = Self::run_subject(event.payload.run_id());
        let bytes = event
            .to_json_bytes()
            .map_err(|e| EventStoreError::PublishFailed {
                message: format!("failed to serialize event: {e}"),
            })?;

        self.jetstream
            .publish(subject, bytes.into())
            .await
            .map_err(|e| EventStoreError::PublishFailed {
                message: e.to_string(),
            })?
            .await
            .map_err(|e| EventStoreError::PublishFailed {
                message: e.to_string(),
            })?;

        Ok(())
    }

    async fn load_events(
        &self,
        run_id: WorkflowRunId,
    ) -> Result<Vec<ExecutionEvent>, EventStoreError> {
        let stream = self
            .jetstream
            .get_stream(self.config.events_stream())
            .await
            .map_err(|e| EventStoreError::LoadFailed {
                message: format!("failed to get stream: {e}"),
            })?;

        let subject = Self::run_subject(run_id);

        // Get messages from the stream for this subject
        let consumer_config = jetstream::consumer::pull::Config {
            filter_subject: subject,
            deliver_policy: jetstream::consumer::DeliverPolicy::All,
            ..Default::default()
        };

        let consumer = stream.create_consumer(consumer_config).await.map_err(|e| {
            EventStoreError::LoadFailed {
                message: format!("failed to create consumer: {e}"),
            }
        })?;

        let mut events = Vec::new();
        let mut messages = consumer
            .messages()
            .await
            .map_err(|e| EventStoreError::LoadFailed {
                message: format!("failed to get messages: {e}"),
            })?;

        use futures::StreamExt;
        while let Ok(Some(message)) =
            tokio::time::timeout(std::time::Duration::from_millis(100), messages.next()).await
        {
            let message = message.map_err(|e| EventStoreError::LoadFailed {
                message: e.to_string(),
            })?;

            let envelope: Envelope<ExecutionEvent> = Envelope::from_json_bytes(&message.payload)
                .map_err(|e| EventStoreError::LoadFailed {
                    message: format!("failed to deserialize event: {e}"),
                })?;

            events.push(envelope.into_payload());

            message
                .ack()
                .await
                .map_err(|e| EventStoreError::LoadFailed {
                    message: format!("failed to ack message: {e}"),
                })?;
        }

        // Clean up the ephemeral consumer
        drop(messages);

        Ok(events)
    }

    async fn publish_work_item(&self, item: Envelope<WorkItem>) -> Result<(), EventStoreError> {
        let subject = Self::work_subject();
        let bytes = serde_json::to_vec(&item).map_err(|e| EventStoreError::PublishFailed {
            message: format!("failed to serialize work item: {e}"),
        })?;

        self.jetstream
            .publish(subject, bytes.into())
            .await
            .map_err(|e| EventStoreError::PublishFailed {
                message: e.to_string(),
            })?
            .await
            .map_err(|e| EventStoreError::PublishFailed {
                message: e.to_string(),
            })?;

        Ok(())
    }
}

/// NATS Object Store-based output storage.
///
/// Node outputs are stored with auto-generated keys.
pub struct NatsObjectStore {
    store: object_store::ObjectStore,
}

impl NatsObjectStore {
    /// Creates a new NATS object store.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or bucket setup fails.
    pub async fn new(config: &NatsConfig) -> Result<Self, ObjectStoreError> {
        let client =
            async_nats::connect(&config.url)
                .await
                .map_err(|e| ObjectStoreError::StoreFailed {
                    message: format!("failed to connect: {e}"),
                })?;

        let jetstream = async_nats::jetstream::new(client);

        // Create or get the object store bucket
        let store = jetstream
            .create_object_store(object_store::Config {
                bucket: config.outputs_bucket().to_string(),
                ..Default::default()
            })
            .await
            .map_err(|e| ObjectStoreError::StoreFailed {
                message: format!("failed to create object store: {e}"),
            })?;

        Ok(Self { store })
    }

    /// Generates a unique key for an object.
    fn generate_key() -> String {
        format!("output_{}", Ulid::new())
    }
}

#[async_trait]
impl ObjectStore for NatsObjectStore {
    async fn put(&self, data: &[u8]) -> Result<String, ObjectStoreError> {
        let key = Self::generate_key();

        // Use key as &str which implements Into<ObjectMetadata>
        self.store
            .put(key.as_str(), &mut std::io::Cursor::new(data))
            .await
            .map_err(|e| ObjectStoreError::StoreFailed {
                message: e.to_string(),
            })?;

        Ok(key)
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, ObjectStoreError> {
        let mut result = self.store.get(key).await.map_err(|e| {
            if e.to_string().contains("not found") {
                ObjectStoreError::NotFound {
                    key: key.to_string(),
                }
            } else {
                ObjectStoreError::RetrieveFailed {
                    message: e.to_string(),
                }
            }
        })?;

        use tokio::io::AsyncReadExt;
        let mut data = Vec::new();
        result
            .read_to_end(&mut data)
            .await
            .map_err(|e| ObjectStoreError::RetrieveFailed {
                message: e.to_string(),
            })?;

        Ok(data)
    }

    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        self.store
            .delete(key)
            .await
            .map_err(|e| ObjectStoreError::DeleteFailed {
                message: e.to_string(),
            })?;

        Ok(())
    }
}

/// Creates both event store and object store from the same config.
///
/// This is a convenience function for setting up the full NATS infrastructure.
///
/// # Errors
///
/// Returns an error if connection or setup fails.
pub async fn create_nats_stores(
    config: &NatsConfig,
) -> Result<(NatsEventStore, NatsObjectStore), NatsSetupError> {
    let event_store = NatsEventStore::new(config.clone())
        .await
        .map_err(NatsSetupError::EventStore)?;

    let object_store = NatsObjectStore::new(config)
        .await
        .map_err(NatsSetupError::ObjectStore)?;

    Ok((event_store, object_store))
}

/// Errors from NATS setup.
#[derive(Debug)]
pub enum NatsSetupError {
    /// Event store setup failed.
    EventStore(EventStoreError),
    /// Object store setup failed.
    ObjectStore(ObjectStoreError),
}

impl std::fmt::Display for NatsSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventStore(e) => write!(f, "event store setup failed: {e}"),
            Self::ObjectStore(e) => write!(f, "object store setup failed: {e}"),
        }
    }
}

impl std::error::Error for NatsSetupError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nats_config_defaults() {
        let config = NatsConfig::new("nats://localhost:4222");

        assert_eq!(config.events_stream(), EVENTS_STREAM_NAME);
        assert_eq!(config.work_stream(), WORK_STREAM_NAME);
        assert_eq!(config.outputs_bucket(), OUTPUTS_BUCKET_NAME);
    }

    #[test]
    fn nats_config_custom() {
        let config = NatsConfig {
            url: "nats://localhost:4222".to_string(),
            events_stream_name: Some("CUSTOM_EVENTS".to_string()),
            work_stream_name: Some("CUSTOM_WORK".to_string()),
            outputs_bucket_name: Some("custom-outputs".to_string()),
        };

        assert_eq!(config.events_stream(), "CUSTOM_EVENTS");
        assert_eq!(config.work_stream(), "CUSTOM_WORK");
        assert_eq!(config.outputs_bucket(), "custom-outputs");
    }

    #[test]
    fn run_subject_format() {
        let run_id = WorkflowRunId::new();
        let subject = NatsEventStore::run_subject(run_id);
        assert!(subject.starts_with("workflow.run."));
    }

    #[test]
    fn key_generation() {
        let key1 = NatsObjectStore::generate_key();
        let key2 = NatsObjectStore::generate_key();

        assert!(key1.starts_with("output_"));
        assert!(key2.starts_with("output_"));
        assert_ne!(key1, key2); // Keys should be unique
    }
}
