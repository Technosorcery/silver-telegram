# Brainstorming Session: Workflow Execution Model

**Created:** 2025-12-25 10:00
**Status:** Complete

## Quick Reference

### Requirements & Constraints
- Primary goal: Simplicity (robustness, resumability, low latency flow from simplicity)
- Output: Updates to PRD and ARCHITECTURE.md (not implementation plan)
- Must work with existing decisions: NATS + JetStream (ADR-004), petgraph + JSONB (ADR-005)

### Alternatives Explored
[To be updated as alternatives are discussed]

### Key Decisions
- **Persistence granularity**: Per-node completion. Persist output after each node completes. Resume from last completed node on crash recovery.
- **Parallel synchronization**: Implicit barrier. Nodes with multiple incoming edges wait for ALL predecessors to complete before executing.
- **Failure handling**: Partial completion. Failed nodes block downstream, independent branches continue. Run ends showing what succeeded/failed.

### Execution Algorithm (Remaining Work Graph)
Insight from user: Use a "remaining work" subgraph derived from the workflow:
1. Start with full workflow graph
2. Remove nodes that have completed (edges to them disappear)
3. Failed nodes get a self-edge (never become ready again, block downstream)
4. Nodes with no incoming edges are ready for execution
5. When no nodes have 0 incoming edges AND no nodes executing → run complete

Reference: Similar pattern to DependentValueGraph in systeminit/si

- **NATS integration**: Full event sourcing. All state changes (run started, node started, node completed, run finished) go through NATS. State is reconstructed from the event stream.
- **Executor model**: Single orchestrator per run + worker pool. Orchestrator determines ready nodes and publishes work items. Workers execute nodes and publish completion/failure events. Orchestrator advances the graph.
- **Retry policy**: No automatic retries. Failed nodes are marked failed immediately. User can manually retry the run or intervene. Keeps orchestrator simple; retries can be added later if needed.
- **Orchestrator assignment**: Standard job queue semantics. Trigger fires → job queued → available orchestrator dequeues (implicit claim). JetStream ack semantics handle crash recovery: unacked job redelivers, new orchestrator reconstructs from event stream for that run.
- **Worker routing**: Deferred. All workers have same capabilities for now. Simple NATS job queue - workers pull from single work queue. Capability-based routing added later when needed.
- **Node output storage**: NATS Object Store. Worker writes output to Object Store, publishes completion event with key/reference. Keeps PostgreSQL for relational data only.
- **Serialization versioning**: All serialized data includes a version header/envelope - events, workflow definitions (JSONB), node outputs, anything persisted. Recipients/readers can handle multiple versions, enabling schema evolution, in-place upgrades, and rolling deployments.

### Current Design State
[To be updated as design elements are validated]

### Open Questions
- ~~Execution granularity~~ → Per-node completion
- ~~How parallel branches coordinate~~ → Implicit barrier
- ~~Failure/retry semantics~~ → Partial completion with remaining work graph
- ~~NATS integration pattern~~ → Full event sourcing
- ~~Executor concurrency model~~ → Orchestrator + worker pool
- ~~Retry semantics~~ → No automatic retries
- ~~Orchestrator lifecycle~~ → Job queue semantics; JetStream ack for crash recovery
- ~~Worker assignment pattern~~ → Deferred; all workers same capabilities for now
- ~~Node output storage~~ → NATS Object Store with key in completion event
- ~~Event schema design~~ → Versioned envelopes for all serialized data (schema evolution, rolling deployments)

---

## Chronological Log

### [Phase 1 - 10:00] Initial Understanding
- Context: Designing execution model for workflow engine
- Existing foundation: petgraph for graph structure, NATS for events, PostgreSQL for persistence
- User clarified: This is architecture design, not implementation planning
- Primary goal: Simplicity first - other qualities flow from it
- [10:05] Selected per-node completion persistence - enables crash recovery without full restart
- [10:07] Selected implicit barrier synchronization - simplest rule, every multi-input node waits for all predecessors
- [10:12] Selected partial completion for failures - user insight: not actually more complex. Failed nodes get self-edge in remaining work graph, naturally blocking downstream. Algorithm: remove completed nodes, self-edge failed nodes, 0-incoming-edge nodes are ready. Reference: DependentValueGraph pattern from si repo.
- [10:15] Selected full event sourcing via NATS. All state changes flow through NATS, state reconstructed from event stream.
- [10:18] Selected orchestrator + worker pool model. Clean separation: orchestrator handles graph logic, workers handle execution.
- [10:20] Selected no automatic retries. Failed is failed. Simplicity; retries can be layered on later if needed.
- [10:23] User correction: standard job queue semantics, not Kafka-style. Trigger → job queued → orchestrator dequeues (claim). JetStream ack handles crash recovery naturally.
- [10:26] User clarification: Worker assignment is similar pattern but with capability requirements. AI nodes need GPU/model access, fetch nodes need network access, etc.
- [10:28] Explored hybrid worker routing options. Researched Hatchet (worker affinity labels) and RabbitMQ (no native subset matching).
- [10:45] Decision: Defer capability routing. All workers have same capabilities for now. Simple NATS job queue. YAGNI - solve when needed.
- [10:48] Node output storage: NATS Object Store, not PostgreSQL. Keep PG for relational data; Object Store for blob/output storage. Completion event contains key/reference.
- [10:52] Serialization versioning: All serialized data (events, workflow definitions, node outputs, anything persisted) includes version header/envelope. Enables schema evolution, in-place upgrades, and rolling deployments.
- [11:00] Architecture document updated with ADR-006: Workflow Execution Model. Updated Section 3.1 (NATS description), Section 6.7 (open questions), Section 12.2 (pending decisions and PRD mapping).
