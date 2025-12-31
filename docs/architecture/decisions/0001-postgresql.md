# ADR-001: PostgreSQL as Primary Database

**Status**: Accepted

## Context

Need a database for workflow definitions, execution state, conversation history, credentials, and user data. Must support future multi-user scaling and concurrent writes.

## Decision

Use PostgreSQL with SQLx as the Rust driver.

**Deployment**: Container sidecar via Docker Compose (Postgres container alongside the application).

## Rationale

- Concurrent write handling for multi-user scenarios
- JSONB for flexible document storage where schema evolution is needed
- Mature ecosystem with excellent SQLx support
- Compile-time query checking via SQLx

## Consequences

- Requires Postgres container in deployment (not embedded like SQLite)
- More operational complexity than SQLite, but standard Docker Compose pattern
- Connection pooling needed for production use
