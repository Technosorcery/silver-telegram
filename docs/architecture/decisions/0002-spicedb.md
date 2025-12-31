# ADR-002: SpiceDB for Relationship-Based Authorization

**Status**: Accepted

## Context

Need multi-user authorization that supports:
- User isolation by default
- Household/shared integrations (e.g., family calendar)
- Future sharing of workflows, templates
- Flexible permission model without schema changes per feature

## Decision

Use SpiceDB (Zanzibar-style) as a sidecar container for relationship-based authorization.

**Deployment**: SpiceDB container in Docker Compose, using Postgres as its storage backend.

**Key concepts**:
- Resources (workflows, integrations, etc.) don't have `user_id` columns
- Authorization relationships stored in SpiceDB: `workflow:123#owner@user:alice`
- Permission checks via SpiceDB API: "Can user X do action Y on resource Z?"
- Permissions flow through relationships (user → group → resource)

## Rationale

- Decouples authorization from data model
- Sharing doesn't require altering resource tables
- Consistent permission model across all resources
- Battle-tested Zanzibar semantics

## Consequences

- Additional container in deployment
- All permission checks go through SpiceDB API (latency consideration)
- Need to keep SpiceDB relationships in sync with resource lifecycle
- Learning curve for Zanzibar concepts
