-- Create workflows table for storing workflow definitions
-- Workflows are user-defined automations with triggers, AI nodes, and integrations

CREATE TABLE workflows (
    -- Workflow ID (ULID stored as text with prefix)
    id TEXT PRIMARY KEY,

    -- Human-readable name for the workflow
    -- Note: Ownership is tracked in SpiceDB, not via user_id column (see ADR-002)
    name TEXT NOT NULL,

    -- Optional description of what the workflow does
    description TEXT,

    -- Whether the workflow is enabled (disabled workflows don't run on schedule)
    enabled BOOLEAN NOT NULL DEFAULT true,

    -- Tags for organization (stored as JSON array)
    tags JSONB NOT NULL DEFAULT '[]',

    -- The workflow graph definition (nodes and edges as JSONB)
    -- This is the source of truth for the workflow structure
    graph_data JSONB NOT NULL DEFAULT '{"nodes": [], "edges": []}',

    -- When the workflow was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the workflow was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for finding enabled workflows
CREATE INDEX workflows_enabled_idx ON workflows (enabled) WHERE enabled = true;

-- Create triggers table for denormalized trigger lookup
-- Triggers are extracted from workflow graphs for efficient scheduling
CREATE TABLE triggers (
    -- Trigger ID (ULID)
    id TEXT PRIMARY KEY,

    -- Reference to the workflow this trigger belongs to
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,

    -- Node ID within the workflow graph
    node_id TEXT NOT NULL,

    -- Type of trigger ('schedule', 'webhook', 'integration_event', 'manual')
    trigger_type TEXT NOT NULL,

    -- Trigger-specific configuration (JSON)
    -- For schedule: {"cron": "0 7 * * *", "timezone": "America/New_York"}
    -- For webhook: {"path": "/hooks/my-workflow"}
    -- For integration_event: {"integration_id": "...", "event_type": "..."}
    config_data JSONB NOT NULL DEFAULT '{}',

    -- Whether the trigger is active (matches workflow enabled state)
    active BOOLEAN NOT NULL DEFAULT true,

    -- When the trigger was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the trigger was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Unique constraint on workflow + node
    CONSTRAINT triggers_workflow_node_unique UNIQUE (workflow_id, node_id)
);

-- Index for finding active schedule triggers
CREATE INDEX triggers_schedule_active_idx ON triggers (trigger_type, active)
    WHERE trigger_type = 'schedule' AND active = true;

-- Index for finding webhook triggers by path
CREATE INDEX triggers_webhook_path_idx ON triggers ((config_data->>'path'))
    WHERE trigger_type = 'webhook';

-- Index for workflow lookup
CREATE INDEX triggers_workflow_id_idx ON triggers (workflow_id);

-- Create workflow_memory table for cross-run state
-- Memory is AI-managed opaque data persisted across workflow runs
CREATE TABLE workflow_memory (
    -- Memory ID (ULID)
    id TEXT PRIMARY KEY,

    -- Reference to the workflow
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,

    -- The memory content (opaque bytes, managed by AI)
    content BYTEA NOT NULL DEFAULT '',

    -- Version number for optimistic concurrency
    version INTEGER NOT NULL DEFAULT 1,

    -- When the memory was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the memory was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure one memory record per workflow
    CONSTRAINT workflow_memory_unique UNIQUE (workflow_id)
);

-- Index for looking up memory by workflow
CREATE INDEX workflow_memory_workflow_id_idx ON workflow_memory (workflow_id);
