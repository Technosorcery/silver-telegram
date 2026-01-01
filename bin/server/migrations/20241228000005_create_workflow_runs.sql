-- Create workflow_runs table for execution history
-- Each run represents a single execution of a workflow

CREATE TABLE workflow_runs (
    -- Run ID (ULID stored as text with prefix)
    id TEXT PRIMARY KEY,

    -- Reference to the workflow being executed
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,

    -- Reference to the trigger that initiated this run (optional for manual runs)
    trigger_id TEXT REFERENCES triggers(id) ON DELETE SET NULL,

    -- Current execution state
    -- 'queued': Waiting for an orchestrator
    -- 'running': Actively executing
    -- 'completed': Finished successfully
    -- 'failed': Finished with error
    -- 'cancelled': Cancelled by user or system
    state TEXT NOT NULL DEFAULT 'queued',

    -- When the run was queued
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the run started executing
    started_at TIMESTAMPTZ,

    -- When the run finished (completed, failed, or cancelled)
    finished_at TIMESTAMPTZ,

    -- Input data that triggered the run (JSON)
    input_data JSONB,

    -- Final output of the run if completed (JSON)
    output_data JSONB,

    -- Error message if failed
    error_message TEXT,

    -- Duration in milliseconds (computed on completion)
    duration_ms BIGINT
);

-- Index for looking up runs by workflow
CREATE INDEX workflow_runs_workflow_id_idx ON workflow_runs (workflow_id);

-- Index for finding runs by state
CREATE INDEX workflow_runs_state_idx ON workflow_runs (state);

-- Index for finding recent runs (for listings)
CREATE INDEX workflow_runs_queued_at_idx ON workflow_runs (queued_at DESC);

-- Index for finding running workflows (for cancellation)
CREATE INDEX workflow_runs_running_idx ON workflow_runs (state)
    WHERE state IN ('queued', 'running');

-- Create node_executions table for per-node execution records
-- Each record represents the execution of a single node within a run
CREATE TABLE node_executions (
    -- Execution ID (ULID)
    id TEXT PRIMARY KEY,

    -- Reference to the workflow run
    run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,

    -- Node ID within the workflow graph
    node_id TEXT NOT NULL,

    -- Current execution state
    -- 'pending': Waiting for predecessors
    -- 'ready': All predecessors complete, ready to execute
    -- 'running': Currently executing
    -- 'completed': Finished successfully
    -- 'failed': Finished with error
    -- 'skipped': Skipped (e.g., branch not taken)
    state TEXT NOT NULL DEFAULT 'pending',

    -- When execution started
    started_at TIMESTAMPTZ,

    -- When execution finished
    finished_at TIMESTAMPTZ,

    -- Input data received (JSON)
    input_data JSONB,

    -- Output key for NATS Object Store (large outputs stored there)
    output_key TEXT,

    -- Error message if failed
    error_message TEXT,

    -- Duration in milliseconds
    duration_ms BIGINT,

    -- Unique constraint on run + node
    CONSTRAINT node_executions_run_node_unique UNIQUE (run_id, node_id)
);

-- Index for looking up executions by run
CREATE INDEX node_executions_run_id_idx ON node_executions (run_id);

-- Index for finding executions by state (for the orchestrator)
CREATE INDEX node_executions_state_idx ON node_executions (state)
    WHERE state IN ('pending', 'ready', 'running');

-- Create decision_traces table for AI decision logging
-- Records the reasoning behind AI node decisions
CREATE TABLE decision_traces (
    -- Trace ID (ULID)
    id TEXT PRIMARY KEY,

    -- Reference to the node execution
    node_execution_id TEXT NOT NULL REFERENCES node_executions(id) ON DELETE CASCADE,

    -- Sequence number for ordering traces within an execution
    sequence INTEGER NOT NULL DEFAULT 0,

    -- Type of trace ('llm_call', 'tool_call', 'decision')
    trace_type TEXT NOT NULL,

    -- Trace data (JSON)
    -- For llm_call: {"prompt": "...", "response": "...", "model": "...", "tokens": {...}}
    -- For tool_call: {"tool": "...", "input": {...}, "output": {...}}
    -- For decision: {"options": [...], "chosen": "...", "reasoning": "..."}
    trace_data JSONB NOT NULL,

    -- When this trace was recorded
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for looking up traces by execution
CREATE INDEX decision_traces_execution_id_idx ON decision_traces (node_execution_id);

-- Index for ordering traces
CREATE INDEX decision_traces_sequence_idx ON decision_traces (node_execution_id, sequence);
