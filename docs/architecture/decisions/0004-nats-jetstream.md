# ADR-004: NATS + JetStream for Event Bus

**Status**: Accepted

## Context

Need durable event handling for workflow triggers, step completion, integration events, and internal notifications. Per ADR-003, must be durable (not purely in-memory).

## Decision

Use NATS with JetStream as the event bus.

**Deployment**: NATS container in Docker Compose with JetStream enabled and persistent storage.

**Usage pattern** (event-driven, not RPC):
- Publish events to subjects: `workflow.completed.{id}`, `integration.email.received`
- Consumers subscribe to patterns with durable subscriptions
- JetStream provides persistence, replay, and exactly-once semantics

**Rust client**: `async-nats` crate

## Rationale

- Lightweight (~10-20MB footprint)
- Simple pub/sub model naturally discourages RPC-over-bus patterns
- JetStream adds durability without changing the programming model
- Clean async Rust client
- Subject-based routing with wildcards fits event hierarchies

## Consequences

- Additional container in deployment
- Need to manage JetStream streams and consumers
- Events must be designed as fire-and-forget, not request/reply
