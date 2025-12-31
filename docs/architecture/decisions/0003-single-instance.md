# ADR-003: Single Instance with Scaling-Compatible Patterns

**Status**: Accepted

## Context

Need to define deployment topology - single-node vs distributed, single instance vs multiple.

## Decision

Single application instance for now, designed to allow horizontal scaling later.

**Constraints on implementation**:
- No in-memory session state (use database-backed sessions)
- No in-process locks for cross-request coordination
- Scheduler must support single-writer or leader election pattern
- Event handling must be durable, not purely in-memory

## Rationale

- Home lab / self-hosted use case doesn't need horizontal scaling initially
- Avoiding anti-patterns now prevents costly rewrites later
- Simpler operations for single instance deployment

## Consequences

- Some implementation patterns are ruled out (e.g., in-memory caches for session state)
- Event bus decision must consider durability requirement
