# ADR-006: Workflow Execution Model

**Status**: Accepted

## Context

Need to define how workflows execute: state tracking, parallel coordination, failure handling, and integration with NATS.

## Decisions

1. **Persistence granularity**: Per-node completion. After each node completes, persist its output. On crash recovery, resume from last completed node.

2. **Parallel synchronization**: Implicit barrier. Nodes with multiple incoming edges wait for ALL predecessors to complete before executing.

3. **Failure handling**: Partial completion. Failed nodes block downstream; independent branches continue. Run ends showing what succeeded/failed.

4. **Execution algorithm** (remaining work graph):
   - Start with full workflow graph
   - Remove nodes that have completed
   - Failed nodes get a self-edge (never become ready, block downstream)
   - Nodes with 0 incoming edges are ready for execution
   - When no nodes have 0 incoming edges AND no nodes executing → run complete

   (Pattern: similar to DependentValueGraph in systeminit/si)

5. **Event sourcing**: Full event sourcing via NATS. All state changes (run started, node started, node completed, run finished) published to NATS. State reconstructed from event stream on recovery.

6. **Executor model**: Orchestrator + worker pool.
   - Orchestrator: One per run, determines ready nodes, publishes work items
   - Workers: Execute nodes, publish completion/failure events
   - Clean separation: orchestrator handles graph logic, workers handle execution

7. **Orchestrator assignment**: Job queue semantics. Trigger fires → job queued → available orchestrator dequeues (implicit claim). JetStream ack handles crash recovery: unacked job redelivers, new orchestrator reconstructs from event stream.

8. **Worker routing**: Deferred. All workers have same capabilities for now. Simple NATS work queue. Capability-based routing added when needed.

9. **Retry policy**: No automatic retries. Failed nodes marked failed immediately. User can manually retry. Simplicity first; retries can be layered on later.

10. **Node output storage**: NATS Object Store. Worker writes output to Object Store, publishes completion event with key/reference. Keeps PostgreSQL for relational data only.

11. **Serialization versioning**: All serialized data includes version header/envelope (events, workflow definitions, node outputs). Enables schema evolution, in-place upgrades, and rolling deployments.

## Rationale

- Simplicity as primary goal; robustness flows from simplicity
- Event sourcing provides durability and auditability
- Orchestrator/worker separation enables future scaling
- NATS Object Store avoids bloating PostgreSQL with blob data
- Versioned envelopes enable zero-downtime deployments

## Consequences

- Event stream becomes source of truth for run state
- Must design event schemas carefully (versioned from start)
- NATS Object Store adds storage management consideration
- Capability routing deferred; revisit when worker heterogeneity needed
