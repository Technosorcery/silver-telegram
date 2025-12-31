# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the silver-telegram project.

## What is an ADR?

An ADR captures an important architectural decision made along with its context and consequences. ADRs are immutable once accepted - if a decision changes, a new ADR supersedes the old one.

## ADR Format

Each ADR follows this structure:

```markdown
# ADR-NNN: Title

**Status**: Proposed | Accepted | Deprecated | Superseded by ADR-XXX

## Context

What is the issue that we're seeing that is motivating this decision?

## Decision

What is the change that we're proposing and/or doing?

## Rationale

Why is this the right decision? What alternatives were considered?

## Consequences

What becomes easier or more difficult because of this decision?
```

## Creating a New ADR

1. Copy an existing ADR as a template
2. Use the next available number: `NNNN-short-title.md`
3. Fill in all sections
4. Add an entry to the index in `../ARCHITECTURE.md`
5. Submit for review

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-postgresql.md) | PostgreSQL as Primary Database | Accepted |
| [0002](0002-spicedb.md) | SpiceDB for Relationship-Based Authorization | Accepted |
| [0003](0003-single-instance.md) | Single Instance with Scaling-Compatible Patterns | Accepted |
| [0004](0004-nats-jetstream.md) | NATS + JetStream for Event Bus | Accepted |
| [0005](0005-workflow-representation.md) | Workflow Representation as petgraph with JSONB Storage | Accepted |
| [0006](0006-workflow-execution.md) | Workflow Execution Model | Accepted |
| [0007](0007-configuration-management.md) | Centralized Configuration Management | Accepted |
