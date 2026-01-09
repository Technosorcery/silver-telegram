//! Workflow worker for executing nodes.
//!
//! Per ADR-006:
//! - Workers: Execute nodes, publish completion/failure events
//! - Clean separation: orchestrator handles graph logic, workers handle execution
//! - All workers have same capabilities (capability-based routing deferred)
//!
//! The worker:
//! 1. Receives work items from the queue
//! 2. Executes the node
//! 3. Stores output to Object Store
//! 4. Publishes completion/failure result

use crate::node::Node;
use crate::orchestrator::{WorkItem, WorkItemResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Trait for object storage operations.
///
/// Per ADR-006, node outputs are stored in NATS Object Store.
/// This abstraction allows testing without NATS.
#[async_trait]
pub trait ObjectStore: Send + Sync {
    /// Stores data and returns the key.
    async fn put(&self, data: &[u8]) -> Result<String, ObjectStoreError>;

    /// Retrieves data by key.
    async fn get(&self, key: &str) -> Result<Vec<u8>, ObjectStoreError>;

    /// Deletes data by key.
    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError>;
}

/// Errors from object store operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectStoreError {
    /// Failed to store data.
    StoreFailed { message: String },
    /// Key not found.
    NotFound { key: String },
    /// Failed to retrieve data.
    RetrieveFailed { message: String },
    /// Failed to delete data.
    DeleteFailed { message: String },
}

impl std::fmt::Display for ObjectStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StoreFailed { message } => write!(f, "object store put failed: {message}"),
            Self::NotFound { key } => write!(f, "object not found: {key}"),
            Self::RetrieveFailed { message } => write!(f, "object store get failed: {message}"),
            Self::DeleteFailed { message } => write!(f, "object store delete failed: {message}"),
        }
    }
}

impl std::error::Error for ObjectStoreError {}

/// Trait for node execution.
///
/// This abstraction allows testing the worker without actual node implementations.
/// In production, this would dispatch to integration connectors, AI primitives, etc.
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Executes a node with the given inputs.
    ///
    /// Returns the output as JSON.
    async fn execute(
        &self,
        node: &Node,
        inputs: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, NodeExecutionError>;
}

/// Errors from node execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeExecutionError {
    /// Input validation failed.
    InvalidInput { message: String },
    /// Execution failed.
    ExecutionFailed { message: String },
    /// Node type not supported.
    UnsupportedNodeType { node_type: String },
    /// External service error.
    ExternalServiceError { service: String, message: String },
    /// Timeout.
    Timeout,
}

impl std::fmt::Display for NodeExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { message } => write!(f, "invalid input: {message}"),
            Self::ExecutionFailed { message } => write!(f, "execution failed: {message}"),
            Self::UnsupportedNodeType { node_type } => {
                write!(f, "unsupported node type: {node_type}")
            }
            Self::ExternalServiceError { service, message } => {
                write!(f, "external service error ({service}): {message}")
            }
            Self::Timeout => write!(f, "execution timed out"),
        }
    }
}

impl std::error::Error for NodeExecutionError {}

/// Errors that can occur during worker operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerError {
    /// Object store error.
    ObjectStore(ObjectStoreError),
    /// Node execution error.
    Execution(NodeExecutionError),
    /// Node not found in workflow.
    NodeNotFound { node_id: String },
    /// Failed to deserialize input.
    DeserializationFailed { message: String },
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ObjectStore(e) => write!(f, "object store error: {e}"),
            Self::Execution(e) => write!(f, "execution error: {e}"),
            Self::NodeNotFound { node_id } => write!(f, "node not found: {node_id}"),
            Self::DeserializationFailed { message } => {
                write!(f, "deserialization failed: {message}")
            }
        }
    }
}

impl std::error::Error for WorkerError {}

impl From<ObjectStoreError> for WorkerError {
    fn from(e: ObjectStoreError) -> Self {
        Self::ObjectStore(e)
    }
}

impl From<NodeExecutionError> for WorkerError {
    fn from(e: NodeExecutionError) -> Self {
        Self::Execution(e)
    }
}

/// The workflow worker.
///
/// Executes individual nodes and reports results.
pub struct Worker<O: ObjectStore, E: NodeExecutor> {
    object_store: O,
    executor: E,
}

impl<O: ObjectStore, E: NodeExecutor> Worker<O, E> {
    /// Creates a new worker.
    pub fn new(object_store: O, executor: E) -> Self {
        Self {
            object_store,
            executor,
        }
    }

    /// Processes a work item.
    ///
    /// 1. Retrieves inputs from object store
    /// 2. Executes the node
    /// 3. Stores output to object store
    /// 4. Returns the result
    pub async fn process(&self, work_item: WorkItem, node: &Node) -> WorkItemResult {
        match self.execute_node(work_item.clone(), node).await {
            Ok(output_key) => WorkItemResult::Completed {
                run_id: work_item.run_id,
                node_id: work_item.node_id,
                output_key,
            },
            Err(e) => WorkItemResult::Failed {
                run_id: work_item.run_id,
                node_id: work_item.node_id,
                error: e.to_string(),
            },
        }
    }

    /// Executes a node and returns the output key.
    async fn execute_node(&self, work_item: WorkItem, node: &Node) -> Result<String, WorkerError> {
        // Retrieve inputs from object store
        let inputs = self.retrieve_inputs(&work_item.inputs).await?;

        // Execute the node
        let output = self.executor.execute(node, inputs).await?;

        // Store output to object store
        let output_bytes =
            serde_json::to_vec(&output).map_err(|e| WorkerError::DeserializationFailed {
                message: e.to_string(),
            })?;
        let output_key = self.object_store.put(&output_bytes).await?;

        Ok(output_key)
    }

    /// Retrieves inputs from object store.
    async fn retrieve_inputs(
        &self,
        input_keys: &HashMap<String, String>,
    ) -> Result<HashMap<String, JsonValue>, WorkerError> {
        let mut inputs = HashMap::new();

        for (port_name, key) in input_keys {
            let bytes = self.object_store.get(key).await?;
            let value: JsonValue =
                serde_json::from_slice(&bytes).map_err(|e| WorkerError::DeserializationFailed {
                    message: e.to_string(),
                })?;
            inputs.insert(port_name.clone(), value);
        }

        Ok(inputs)
    }
}

/// A simple executor that echoes inputs as output (for testing).
pub struct EchoExecutor;

#[async_trait]
impl NodeExecutor for EchoExecutor {
    async fn execute(
        &self,
        _node: &Node,
        inputs: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, NodeExecutionError> {
        Ok(JsonValue::Object(inputs.into_iter().collect()))
    }
}

/// A mock executor that can be configured to succeed or fail.
pub struct MockExecutor {
    /// If set, all executions will fail with this error.
    pub fail_with: Option<NodeExecutionError>,
    /// The output to return on success.
    pub output: JsonValue,
}

impl MockExecutor {
    /// Creates a mock executor that succeeds with the given output.
    #[must_use]
    pub fn succeeding(output: JsonValue) -> Self {
        Self {
            fail_with: None,
            output,
        }
    }

    /// Creates a mock executor that fails with the given error.
    #[must_use]
    pub fn failing(error: NodeExecutionError) -> Self {
        Self {
            fail_with: Some(error),
            output: JsonValue::Null,
        }
    }
}

#[async_trait]
impl NodeExecutor for MockExecutor {
    async fn execute(
        &self,
        _node: &Node,
        _inputs: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, NodeExecutionError> {
        match &self.fail_with {
            Some(e) => Err(e.clone()),
            None => Ok(self.output.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{AiLayerNodeConfig, NodeConfig};
    use silver_telegram_core::WorkflowRunId;
    use std::sync::{Arc, Mutex};

    /// In-memory object store for testing.
    struct InMemoryObjectStore {
        data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        counter: Arc<Mutex<u64>>,
    }

    impl InMemoryObjectStore {
        fn new() -> Self {
            Self {
                data: Arc::new(Mutex::new(HashMap::new())),
                counter: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait]
    impl ObjectStore for InMemoryObjectStore {
        async fn put(&self, data: &[u8]) -> Result<String, ObjectStoreError> {
            let mut counter = self.counter.lock().unwrap();
            *counter += 1;
            let key = format!("obj_{counter}");
            self.data.lock().unwrap().insert(key.clone(), data.to_vec());
            Ok(key)
        }

        async fn get(&self, key: &str) -> Result<Vec<u8>, ObjectStoreError> {
            self.data
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| ObjectStoreError::NotFound {
                    key: key.to_string(),
                })
        }

        async fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
            self.data.lock().unwrap().remove(key);
            Ok(())
        }
    }

    fn create_ai_node() -> Node {
        Node::new(
            "AI",
            NodeConfig::AiLayer(AiLayerNodeConfig::Generate {
                instructions: "Test".to_string(),
            }),
        )
    }

    #[tokio::test]
    async fn worker_processes_work_item_successfully() {
        let object_store = InMemoryObjectStore::new();

        // Pre-populate an input
        let input_key = object_store
            .put(&serde_json::to_vec(&serde_json::json!({"data": "test"})).unwrap())
            .await
            .unwrap();

        let executor = MockExecutor::succeeding(serde_json::json!({"result": "success"}));
        let worker = Worker::new(object_store, executor);

        let node = create_ai_node();
        let work_item = WorkItem {
            run_id: WorkflowRunId::new(),
            node_id: node.id,
            inputs: [("context".to_string(), input_key)].into_iter().collect(),
        };

        let result = worker.process(work_item.clone(), &node).await;

        match result {
            WorkItemResult::Completed {
                run_id,
                node_id,
                output_key,
            } => {
                assert_eq!(run_id, work_item.run_id);
                assert_eq!(node_id, work_item.node_id);
                assert!(!output_key.is_empty());
            }
            WorkItemResult::Failed { error, .. } => {
                panic!("expected success, got failure: {error}");
            }
        }
    }

    #[tokio::test]
    async fn worker_handles_execution_failure() {
        let object_store = InMemoryObjectStore::new();
        let executor = MockExecutor::failing(NodeExecutionError::ExecutionFailed {
            message: "test error".to_string(),
        });
        let worker = Worker::new(object_store, executor);

        let node = create_ai_node();
        let work_item = WorkItem {
            run_id: WorkflowRunId::new(),
            node_id: node.id,
            inputs: HashMap::new(),
        };

        let result = worker.process(work_item.clone(), &node).await;

        match result {
            WorkItemResult::Failed {
                run_id,
                node_id,
                error,
            } => {
                assert_eq!(run_id, work_item.run_id);
                assert_eq!(node_id, work_item.node_id);
                assert!(error.contains("test error"));
            }
            WorkItemResult::Completed { .. } => {
                panic!("expected failure, got success");
            }
        }
    }

    #[tokio::test]
    async fn worker_handles_missing_input() {
        let object_store = InMemoryObjectStore::new();
        let executor = MockExecutor::succeeding(serde_json::json!({}));
        let worker = Worker::new(object_store, executor);

        let node = create_ai_node();
        let work_item = WorkItem {
            run_id: WorkflowRunId::new(),
            node_id: node.id,
            inputs: [("context".to_string(), "nonexistent_key".to_string())]
                .into_iter()
                .collect(),
        };

        let result = worker.process(work_item.clone(), &node).await;

        match result {
            WorkItemResult::Failed { error, .. } => {
                assert!(error.contains("not found"));
            }
            WorkItemResult::Completed { .. } => {
                panic!("expected failure due to missing input");
            }
        }
    }

    #[tokio::test]
    async fn echo_executor_echoes_inputs() {
        let executor = EchoExecutor;
        let node = create_ai_node();

        let inputs: HashMap<String, JsonValue> = [
            ("a".to_string(), serde_json::json!("value_a")),
            ("b".to_string(), serde_json::json!(123)),
        ]
        .into_iter()
        .collect();

        let result = executor.execute(&node, inputs.clone()).await.unwrap();

        assert_eq!(result["a"], "value_a");
        assert_eq!(result["b"], 123);
    }

    #[tokio::test]
    async fn worker_stores_output_in_object_store() {
        let object_store = InMemoryObjectStore::new();
        let executor = MockExecutor::succeeding(serde_json::json!({"output": "data"}));
        let worker = Worker::new(object_store, executor);

        let node = create_ai_node();
        let work_item = WorkItem {
            run_id: WorkflowRunId::new(),
            node_id: node.id,
            inputs: HashMap::new(),
        };

        let result = worker.process(work_item, &node).await;

        if let WorkItemResult::Completed { output_key, .. } = result {
            // Verify we can retrieve the output
            let stored = worker.object_store.get(&output_key).await.unwrap();
            let value: JsonValue = serde_json::from_slice(&stored).unwrap();
            assert_eq!(value["output"], "data");
        } else {
            panic!("expected success");
        }
    }
}
