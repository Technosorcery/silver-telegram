# ADR-005: Workflow Representation as petgraph with JSONB Storage

**Status**: Accepted

## Context

Need to define how workflows (graphs of nodes with inputs/outputs) are represented, stored, and how triggers are indexed for efficient execution.

## Decisions

1. **Graph structure**: `petgraph::DiGraph<Node, EdgeWeight>` where edge weights contain port routing (source_port, destination_port)

2. **Port typing**: Structural/schema-based using JSON Schema. Connections valid if schemas compatible. Input ports have a required flag; workflow validation fails if required inputs lack incoming edges.

3. **Node categories**: Trigger, AI Layer, Integration, Transform, Control Flow, Memory, Output

4. **Triggers**: Nodes in graph (source of truth) but denormalized to indexed triggers table for execution efficiency. Reconciled on workflow save.

5. **Storage**: Workflow metadata in columns, graph serialized to JSONB. Triggers table indexed by: cron expression (schedule), webhook path (webhook), event type + integration account (event).

6. **IDs**: ULIDs throughout

## Deferred

- Expression language for transforms/dynamic config (requirements established, no viable Rust impl identified yet)

## Rationale

- petgraph is mature, well-documented Rust graph library
- JSONB allows flexible schema evolution for graph structure
- Denormalized triggers avoid scanning all workflows on every trigger event
- JSON Schema provides structural typing without custom type system

## Consequences

- Must serialize/deserialize petgraph to JSON (serde support exists)
- Trigger reconciliation logic needed on every workflow save
- Expression language decision blocks Transform node implementation
