# Technical Architecture

## silver-telegram

**Version**: 0.1.0 (Draft)
**Status**: Initial architecture documentation with many open decisions

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [System Context (C4 Level 1)](#2-system-context-c4-level-1)
3. [Container Diagram (C4 Level 2)](#3-container-diagram-c4-level-2)
4. [Component Diagrams (C4 Level 3)](#4-component-diagrams-c4-level-3)
5. [Data Architecture](#5-data-architecture)
6. [Workflow Definition Format](#6-workflow-definition-format)
7. [Security Architecture](#7-security-architecture)
8. [Deployment Architecture](#8-deployment-architecture)
9. [API Design](#9-api-design)
10. [Observability](#10-observability)
11. [Crate Structure](#11-crate-structure)
12. [Architecture Decision Records](#12-architecture-decision-records)

---

## 1. Introduction

### 1.1 Purpose

This document describes the technical architecture of silver-telegram, an autonomous personal assistant platform that combines conversational AI with autonomous workflows.

For product requirements, use cases, and detailed capability descriptions, see [PRD.md](../PRD.md).

### 1.2 Audience

- Developers implementing the platform
- Future contributors understanding the system design
- Users wanting to understand how the system works

### 1.3 How to Read C4 Diagrams

This document uses the [C4 model](https://c4model.com/) for visualizing architecture:

- **Level 1 - System Context**: Shows the system as a box surrounded by its users and external systems it interacts with
- **Level 2 - Container**: Zooms into the system to show major deployable units (applications, databases, etc.)
- **Level 3 - Component**: Zooms into a container to show its internal components

Diagrams use [Mermaid](https://mermaid.js.org/) syntax for GitHub rendering.

### 1.4 Document Status

This is a living document. Many architectural decisions are marked as **OPEN** pending design sessions. See [Section 12: Architecture Decision Records](#12-architecture-decision-records) for the status of key decisions.

---

## 2. System Context (C4 Level 1)

The system context shows silver-telegram in relation to its users and external systems.

```mermaid
flowchart TB
    subgraph Users
        TU[Technical User]
        PU[Power User]
    end

    ST[silver-telegram<br/>Autonomous Personal Assistant]

    subgraph External Systems
        EMAIL[Email Providers<br/>IMAP / JMAP / SMTP]
        CAL[Calendar Services<br/>CalDAV / APIs]
        TASKS[Task Managers<br/>Various APIs]
        LLM[LLM Providers<br/>Local or Cloud]
        WH[Webhook Sources<br/>External Events]
    end

    TU -->|configures, inspects, converses| ST
    PU -->|converses, reviews automations| ST

    ST <-->|read/send email| EMAIL
    ST <-->|query/create events| CAL
    ST <-->|manage tasks| TASKS
    ST -->|AI primitive execution| LLM
    WH -->|trigger workflows| ST
```

### 2.1 Users

| User | Description |
|------|-------------|
| **Technical User** | Self-hosts the platform, configures integrations, inspects and modifies workflows, comfortable with configuration files |
| **Power User** | Interacts conversationally, describes automation needs, reviews and approves what gets created |

### 2.2 External Systems

| System | Protocols | Purpose |
|--------|-----------|---------|
| **Email Providers** | IMAP, JMAP, SMTP | Read and send email |
| **Calendar Services** | CalDAV, REST APIs | Query and manage calendar events |
| **Task Managers** | Various REST APIs | Task management integration |
| **LLM Providers** | HTTP APIs | Execute AI primitives (local via Ollama or cloud APIs) |
| **Webhook Sources** | HTTP webhooks | External event triggers for workflows |

---

## 3. Container Diagram (C4 Level 2)

The container diagram shows the major deployable/runnable components within silver-telegram.

```mermaid
flowchart TB
    subgraph Users
        U[User]
    end

    subgraph silver-telegram
        WEB[Web Application<br/>Leptos / Axum<br/>SSR + WASM SPA]

        CONV[Conversation Service<br/>Context management<br/>AI orchestration]

        WF[Workflow Engine<br/>Execution<br/>State machines]

        INT[Integration Adapters<br/>Protocol adapters<br/>IMAP, CalDAV, REST]

        SCHED[Scheduler<br/>Cron triggers<br/>Missed execution handling]

        DB[(PostgreSQL<br/>SQLx driver)]

        SPICE[SpiceDB<br/>Authorization<br/>Relationships]

        NATS[NATS + JetStream<br/>Event bus<br/>Durable messaging]
    end

    subgraph External
        EXT[External Services]
    end

    U -->|HTTPS| WEB
    WEB --> CONV
    WEB --> WF
    CONV --> WF
    CONV --> DB
    WF --> INT
    WF --> DB
    SCHED --> WF
    SCHED --> EVENTS
    EVENTS --> WF
    INT --> EXT
```

### 3.1 Container Descriptions

| Container | Technology | Responsibility |
|-----------|------------|----------------|
| **Web Application** | Leptos/Axum (Rust) | User interface via SSR + WASM hydration |
| **Conversation Service** | Rust | Manages conversation sessions, context, AI primitive orchestration |
| **Workflow Engine** | Rust | Executes workflows, manages state machines, handles step execution |
| **Integration Adapters** | Rust | Protocol-specific adapters for external services |
| **Scheduler** | Rust | Manages scheduled triggers, handles missed executions |
| **Database** | PostgreSQL (SQLx) | Persistent storage for workflows, state, history, credentials |
| **SpiceDB** | SpiceDB (Zanzibar) | Relationship-based authorization, permission checks |
| **NATS** | NATS + JetStream | Event bus, durable messaging, pub/sub |

---

## 4. Component Diagrams (C4 Level 3)

### 4.1 Workflow Engine Components

```mermaid
flowchart TB
    subgraph Workflow Engine
        DEF[Definition Store<br/>Workflow definitions<br/>with versioning]

        EXEC[Workflow Executor<br/>Step-by-step execution]

        SM[State Machine<br/>Execution state<br/>tracking per run]

        NR[Node Registry<br/>Available node types]

        TM[Trigger Manager<br/>Maps triggers<br/>to workflows]

        HIST[Execution History<br/>Audit trail<br/>Decision traces]
    end

    TM --> EXEC
    EXEC --> DEF
    EXEC --> SM
    EXEC --> NR
    EXEC --> HIST
```

| Component | Responsibility |
|-----------|----------------|
| **Definition Store** | Persists workflow definitions with version history |
| **Workflow Executor** | Executes workflows step-by-step, advancing through nodes |
| **State Machine** | Tracks execution state for each workflow run |
| **Node Registry** | Registry of available node types (AI layer, integrations, control flow) |
| **Trigger Manager** | Connects triggers (schedule, event, manual) to workflows |
| **Execution History** | Audit trail of all executions, inputs, outputs, decisions |

### 4.2 Conversation Service Components

```mermaid
flowchart TB
    subgraph Conversation Service
        SM[Session Manager<br/>Active session lifecycle]

        CS[Context Store<br/>Conversation history<br/>Extracted facts]

        AI[AI Layer Interface<br/>LLM Call and Coordinate]

        TR[Tool Registry<br/>Available tools]

        AC[Authoring Controller<br/>Workflow creation mode]
    end

    SM --> CS
    SM --> AI
    SM --> TR
    AC --> SM
```

| Component | Responsibility |
|-----------|----------------|
| **Session Manager** | Manages active conversation session lifecycle |
| **Context Store** | Stores conversation history and extracted context |
| **AI Layer Interface** | Interface to AI primitives (LLM Call, Coordinate) |
| **Tool Registry** | Registry of tools available during conversation |
| **Authoring Controller** | Handles workflow creation from conversation |

#### Context Persistence Strategy (PRD 8.1 - Decided)

**Categories**: Conversation history, Facts (with explicit/inferred source), Corrections/feedback, Workflow execution history (queryable).

**Retention**: Conversation 90 days; Facts until contradicted; Corrections permanent; Workflow runs configurable (default 90 days).

**Surfacing**: Hybrid. Explicitly marked facts always in context; everything else via semantic retrieval on-demand.

### 4.3 Integration Framework Components

```mermaid
flowchart TB
    subgraph Integration Adapters
        CT[Connector Trait<br/>Common interface]

        IMAP[IMAP Adapter<br/>Email read/write]

        CALDAV[CalDAV Adapter<br/>Calendar operations]

        REST[REST Adapter<br/>Generic REST client]

        VAULT[Credential Vault<br/>Encrypted storage]

        RL[Rate Limiter<br/>Per-integration throttling]
    end

    IMAP -.->|implements| CT
    CALDAV -.->|implements| CT
    REST -.->|implements| CT
    CT --> VAULT
    CT --> RL
```

| Component | Responsibility |
|-----------|----------------|
| **Connector Trait** | Common interface all integrations implement |
| **IMAP Adapter** | Email read/write via IMAP protocol |
| **CalDAV Adapter** | Calendar operations via CalDAV |
| **REST Adapter** | Generic REST client for arbitrary APIs |
| **Credential Vault** | Encrypted storage for integration credentials |
| **Rate Limiter** | Per-integration rate limiting to respect external API constraints |

### 4.4 AI Layer Components

```mermaid
flowchart TB
    subgraph AI Layer
        subgraph Primitives
            CALL[LLM Call<br/>Single-shot inference]
            COORD[Coordinate<br/>LLM-driven execution loop]
        end

        LLM[LLM Backend<br/>Provider abstraction]

        PROMPTS[Prompt Registry<br/>Versioned templates]

        SCHEMA[Output Schema<br/>Structured output handling]

        FB[Feedback Store<br/>User feedback tracking]
    end

    CALL --> LLM
    CALL --> PROMPTS
    CALL --> SCHEMA
    COORD --> CALL
    COORD --> FB
```

#### Fundamental Primitives

There are two architecturally distinct AI primitives:

| Primitive | Nature | Description |
|-----------|--------|-------------|
| **LLM Call** | Single-shot inference | Context + prompt + optional output schema → output |
| **Coordinate** | LLM-driven loop | Goal → (decide actions → execute → evaluate) → repeat until done |

**LLM Call** is stateless, single-shot inference. The LLM receives context and a prompt, optionally constrained by an output schema, and produces a response.

**Coordinate** is an iterative process where the LLM controls execution flow:
1. LLM evaluates context and goal
2. LLM decides what operations to run (and how many)
3. Operations execute (tools, sub-workflows, integrations)
4. LLM evaluates results
5. LLM decides: done, or more operations needed?
6. Repeat until done

The key distinction: in Coordinate, the LLM decides the control flow dynamically, not just producing output. It can invoke multiple rounds of operations, reacting to results of previous rounds.

#### Workflow Node Types

The PRD describes several user-facing operations (Classify, Generate, Summarize, Extract, Score, Deduplicate, Decide). These are **workflow node types**, not distinct primitives. They are all built on **LLM Call** with different configurations:

| Node Type | Implementation |
|-----------|----------------|
| **Classify** | LLM Call with prompt asking to categorize, output schema = {category, confidence} |
| **Generate** | LLM Call with free-form output |
| **Summarize** | LLM Call with prompt asking for condensation |
| **Extract** | LLM Call with prompt asking to pull out data, output schema = user-defined structure |
| **Score** | LLM Call with output schema = numeric value |
| **Deduplicate** | LLM Call with output schema = boolean |
| **Decide** | LLM Call with prompt asking to select, output schema = one of the provided options |

These node types provide semantic clarity in workflow definitions and can have specialized prompt templates, but they share the same underlying LLM Call primitive.

#### Supporting Components

| Component | Responsibility |
|-----------|----------------|
| **LLM Backend** | Provider abstraction (local Ollama, cloud APIs) |
| **Prompt Registry** | Versioned prompt templates for node types |
| **Output Schema** | Structured output handling (JSON schema constraints) |
| **Feedback Store** | User feedback on AI outputs for improvement |

---

## 5. Data Architecture

### 5.1 Core Entities

```mermaid
erDiagram
    USER ||--o{ CONVERSATION_SESSION : has
    USER ||--o{ WORKFLOW : owns
    USER ||--o{ INTEGRATION_ACCOUNT : configures

    CONVERSATION_SESSION ||--o{ MESSAGE : contains
    CONVERSATION_SESSION ||--o{ CONTEXT_FACT : extracts

    WORKFLOW ||--o{ WORKFLOW_VERSION : has
    WORKFLOW ||--o{ WORKFLOW_RUN : executes
    WORKFLOW ||--o{ TRIGGER : has
    WORKFLOW ||--o| WORKFLOW_MEMORY : has

    WORKFLOW_RUN ||--o{ NODE_EXECUTION : contains
    NODE_EXECUTION ||--o{ DECISION_TRACE : logs

    INTEGRATION_ACCOUNT ||--o{ CREDENTIAL : stores

    LLM_INVOCATION ||--o{ FEEDBACK : receives
```

### 5.2 Entity Descriptions

| Entity | Description |
|--------|-------------|
| **User** | Platform user account |
| **Conversation Session** | A conversation instance with message history |
| **Message** | Individual message in a conversation |
| **Context Fact** | Extracted fact or preference from conversation |
| **Workflow** | A defined automation |
| **Workflow Version** | Version history for a workflow |
| **Workflow Run** | Single execution of a workflow |
| **Workflow Memory** | Opaque cross-run state managed by AI agents within a workflow |
| **Trigger** | What initiates a workflow (schedule, event, manual) |
| **Node Execution** | Execution record for a single node in a run |
| **Decision Trace** | Explanation of LLM decision (for Coordinate loops and complex calls) |
| **Integration Account** | Configured external service connection |
| **Credential** | Encrypted credential for an integration |
| **LLM Invocation** | Record of an LLM Call or Coordinate execution |
| **Feedback** | User feedback on LLM output |

### 5.3 Authorization Model

Per ADR-002, authorization uses SpiceDB (Zanzibar-style relationships):

- **Resources don't have `user_id` columns** - ownership and permissions are relationships in SpiceDB
- **Permission checks via SpiceDB API** - "Can user X do action Y on resource Z?"
- **Relationships stored separately** - e.g., `workflow:123#owner@user:alice`

This decouples authorization from the data model, allowing flexible sharing without schema changes.

### 5.4 Workflow Memory

Workflows can persist opaque state across runs via **workflow memory**. See [Section 6.9](#69-workflow-memory-nodes) for node descriptions and [PRD 8.6](../PRD.md#86-state-and-memory) for design decisions.

---

## 6. Workflow Definition Format

### 6.1 Graph Model

Workflows are directed graphs using **petgraph** (`DiGraph`).

| Element | Description |
|---------|-------------|
| **Node** | A workflow step with a type, configuration, and named input/output ports |
| **Edge** | Connection between nodes, carrying port routing (source port → destination port) |
| **Port** | Named connection point with a JSON Schema defining accepted/produced data |

Edges connect specific ports, not just nodes. This enables nodes with multiple inputs/outputs to route data explicitly.

### 6.2 Node Categories

| Category | Examples | Notes |
|----------|----------|-------|
| **Trigger** | Schedule, Webhook, IntegrationEvent | Entry points; denormalized for execution |
| **AI Layer** | LLM Call, Coordinate | See [Section 4.4](#44-ai-layer-components) |
| **Integration** | email.fetch, calendar.list | Protocol-specific actions |
| **Transform** | Expression-based data manipulation | For structured data |
| **Control Flow** | Branch, Loop, Parallel, Join | Graph structure |
| **Memory** | Load Workflow Memory, Record Workflow Memory | Cross-run state for AI agents |
| **Output** | Notify, Log, HTTP Response | Terminal actions |

### 6.3 Port Typing

**Structural/schema-based**: Ports have JSON Schemas. Connections are valid if schemas are compatible.

| Port type | Properties |
|-----------|------------|
| **Input** | Name, JSON Schema, required flag |
| **Output** | Name, JSON Schema |

Input ports may be marked as required. Workflow validation fails if a required input has no incoming edge.

### 6.4 Triggers

**Triggers are nodes** in the graph (source of truth for users), but **denormalized to a triggers table** for execution efficiency:

- On workflow save: reconcile triggers table (insert new, update modified, delete removed)
- On trigger fire: query triggers table (indexed), load workflow, start at trigger node

### 6.5 Storage

**Workflows**: Metadata stored in columns (name, version, timestamps); graph structure serialized as JSONB. This allows flexible schema evolution for the graph while keeping queryable metadata in columns.

**Triggers**: Denormalized from graph into a separate table for efficient lookup. Indexed by trigger type:

| Trigger type | Index key(s) |
|--------------|--------------|
| Schedule | cron expression |
| Webhook | webhook path |
| Event | event type + integration account |

Triggers cascade delete when their workflow is deleted.

### 6.6 Expression Language

> **DEFERRED**: Expression language for transforms and dynamic config not yet selected.

**Requirements established:**
- JSON transformation capability
- Conditional expressions
- String templating with data interpolation
- Must have viable Rust implementation

**Candidates considered:** JSONata (Rust lib incomplete), CEL (more validation-focused), Tera/Jinja (less powerful transforms).

### 6.7 Open Questions

- **Execution model**: How engine traverses graph, handles parallel paths, manages state
- **Versioning**: How workflow changes are tracked, rollback, draft vs published
- **Workflow run state**: State management during execution

### 6.8 Workflow Execution Patterns

The workflow engine (not the Coordinate AI primitive) handles these static execution patterns:

| Pattern | Mechanism | Vec<T> handling |
|---------|-----------|-----------------|
| **Sequential** | Linear graph edges | Passed as-is |
| **Graph parallel** | Multiple outgoing edges (no FanOut) | Copied as-is to each downstream node |
| **FanOut parallel** | FanOut node iterates over items | Exploded into individual items |
| **Combined** | FanOut + multiple outgoing edges | Each item sent to all downstream nodes |
| **Conditional** | Branch node with predicate per edge | TBD |

**FanOut node**:
- Waits for all inputs (barrier), flattens arrays into single collection
- Each input item must be compatible with schemas of ALL direct children
- Each item processed through all downstream paths in parallel

**FanIn node**:
- Optional (FanOut doesn't require paired FanIn)
- Scope defined by graph edge from FanIn to corresponding FanOut
- All inputs must have non-empty common schema intersection
- Output: `Vec<CommonSchema>`

**Validation model**:
- Construction time: Port schema compatibility between connected nodes
- Runtime: Each node validates its output against its declared output schema (errors caught at source)

**Distinct from Coordinate**: The Coordinate AI primitive (see [Section 4.4](#44-ai-layer-components)) handles *dynamic* orchestration where the LLM decides at runtime what to execute, how many rounds, and when to stop. Static patterns above are graph structure; Coordinate is LLM-controlled execution.

> **OPEN**: Conditional branching not yet discussed.

### 6.9 Workflow Memory Nodes

Memory nodes enable workflows to persist state across runs, managed by AI agents.

#### Load Workflow Memory

| Aspect | Description |
|--------|-------------|
| **Purpose** | Retrieve persisted memory for use in downstream AI nodes |
| **Configuration** | None (implicitly scoped to current workflow) |
| **Output** | The workflow's stored memory, or empty if none exists |

#### Record Workflow Memory

| Aspect | Description |
|--------|-------------|
| **Purpose** | AI-mediated update of workflow memory based on current context |
| **Configuration** | Update instructions (prompt guiding how AI should maintain memory) |
| **Input** | Workflow output from upstream nodes (accepts any type) |
| **Output** | Updated memory (also persisted to storage) |
| **Implicit input** | Current memory loaded automatically, not user-wired |

**Execution**: Wraps an LLM Call. The AI receives current memory, workflow output, and update instructions, then produces new memory content. On success, memory is persisted as a transaction.

**Checkpointing**: Multiple Record nodes in a workflow enable intermediate saves. If later steps fail, subsequent runs see the last successfully recorded state.

---

## 7. Security Architecture

### 7.1 Authentication

- **OIDC**: OpenID Connect for user authentication
- Single sign-on support with external identity providers

### 7.2 Credential Storage

- All integration credentials encrypted at rest
- No plaintext credentials in configuration or logs
- Credentials isolated per user

### 7.3 Audit Logging

- All access to sensitive data logged
- Workflow executions include full audit trail
- AI primitive decisions traceable

### 7.4 Design Principles

From [PRD Section 6.2](../PRD.md#62-privacy-and-security):

| Requirement | Description |
|-------------|-------------|
| Data Locality | All data stays on user infrastructure by default |
| Credential Security | Encrypted storage, no plaintext credentials |
| Minimal Permissions | Request only needed access scopes |
| No Telemetry | No data sent to platform developers |
| Audit Logging | Track all access to sensitive data |

---

## 8. Deployment Architecture

### 8.1 Topology

**Decision**: Single application instance for now, designed to allow horizontal scaling later.

- All containers (app, Postgres, SpiceDB) run on a single node
- Application code avoids patterns that block future horizontal scaling:
  - No in-memory session state (use database)
  - No in-process locks for cross-request coordination (use database)
  - Scheduler must support single-writer or leader election pattern
  - Event handling must be durable (not purely in-memory)

### 8.2 Design Considerations

From [PRD Section 6.1](../PRD.md#61-deployment):

| Requirement | Description |
|-------------|-------------|
| Self-Hosted | Runs entirely on user-controlled infrastructure |
| Single-Node Viable | Must work on single server (home lab use case) |
| Containerized | Standard container deployment |
| Offline Capable | Core functionality works without internet (local models) |
| Resource Efficient | Reasonable footprint when idle |

### 8.3 Container Sidecar Services

PostgreSQL, SpiceDB, and NATS run as container sidecars alongside the application:

| Service | Role | Dependencies |
|---------|------|--------------|
| **Application** | silver-telegram | Postgres, SpiceDB, NATS |
| **PostgreSQL** | Primary data store | None |
| **SpiceDB** | Authorization | Postgres (as storage backend) |
| **NATS** | Event bus | None (persistent storage volume) |

All services use persistent volumes for data durability.

### 8.4 Open Questions

- Volume and backup strategy

---

## 9. API Design

### 9.1 Web UI API

- **Leptos server functions** provide the API for the web UI
- Automatic RPC stub generation for client-side calls
- Type-safe communication between server and WASM client

**Note**: Leptos server functions are tightly coupled to the frontend's needs. They are not a general-purpose API and should not be treated as one.

### 9.2 General API

> **DEFERRED**: A general API (REST, GraphQL, etc.) for external tooling and scripting is not needed yet.

When needed, considerations from [PRD Section 6.4](../PRD.md#64-extensibility):
- API-first design
- All functionality accessible via API
- Support for external tooling and scripting

### 9.3 Webhook Endpoints

> **TBD**: Design for webhook ingestion for external triggers (when workflow triggers are implemented).

### 9.4 Admin/Observability Endpoints

> **TBD**: Design for administrative and monitoring endpoints.

---

## 10. Observability

### 10.1 Structured Logging

- **`tracing`** crate for structured logging
- Span-based tracing for request flows
- Environment-based log level filtering

### 10.2 Decision Traces

- LLM invocations record reasoning
- Coordinate loops record each iteration's decisions and actions
- Outputs traceable to prompts, context, and output schemas

### 10.3 Health Checks

- Readiness and liveness endpoints
- Integration connectivity status
- Database connection health

### 10.4 Metrics

From [PRD Section 5.8](../PRD.md#58-observability):

| Metric Category | Examples |
|-----------------|----------|
| Execution | Workflow completion rate, error rate, latency |
| AI Layer | LLM call latency, Coordinate iteration counts, output quality signals |
| Resources | Memory usage, database size, queue depth |

---

## 11. Crate Structure

```
silver-telegram/
├── lib/
│   └── core/              # Domain types, errors
│       └── src/
│           ├── lib.rs
│           └── error.rs
└── bin/
    └── server/            # Leptos web application
        └── src/
            ├── main.rs    # Axum server entry
            ├── lib.rs     # WASM hydration entry
            └── app.rs     # Leptos App component
```

---

## 12. Architecture Decision Records

### 12.1 Finalized Decisions

#### ADR-001: PostgreSQL as Primary Database

**Status**: Accepted

**Context**: Need a database for workflow definitions, execution state, conversation history, credentials, and user data. Must support future multi-user scaling and concurrent writes.

**Decision**: Use PostgreSQL with SQLx as the Rust driver.

**Deployment**: Container sidecar via Docker Compose (Postgres container alongside the application).

**Rationale**:
- Concurrent write handling for multi-user scenarios
- JSONB for flexible document storage where schema evolution is needed
- Mature ecosystem with excellent SQLx support
- Compile-time query checking via SQLx

**Consequences**:
- Requires Postgres container in deployment (not embedded like SQLite)
- More operational complexity than SQLite, but standard Docker Compose pattern
- Connection pooling needed for production use

---

#### ADR-002: SpiceDB for Relationship-Based Authorization

**Status**: Accepted

**Context**: Need multi-user authorization that supports:
- User isolation by default
- Household/shared integrations (e.g., family calendar)
- Future sharing of workflows, templates
- Flexible permission model without schema changes per feature

**Decision**: Use SpiceDB (Zanzibar-style) as a sidecar container for relationship-based authorization.

**Deployment**: SpiceDB container in Docker Compose, using Postgres as its storage backend.

**Key concepts**:
- Resources (workflows, integrations, etc.) don't have `user_id` columns
- Authorization relationships stored in SpiceDB: `workflow:123#owner@user:alice`
- Permission checks via SpiceDB API: "Can user X do action Y on resource Z?"
- Permissions flow through relationships (user → group → resource)

**Rationale**:
- Decouples authorization from data model
- Sharing doesn't require altering resource tables
- Consistent permission model across all resources
- Battle-tested Zanzibar semantics

**Consequences**:
- Additional container in deployment
- All permission checks go through SpiceDB API (latency consideration)
- Need to keep SpiceDB relationships in sync with resource lifecycle
- Learning curve for Zanzibar concepts

---

#### ADR-003: Single Instance with Scaling-Compatible Patterns

**Status**: Accepted

**Context**: Need to define deployment topology - single-node vs distributed, single instance vs multiple.

**Decision**: Single application instance for now, designed to allow horizontal scaling later.

**Constraints on implementation**:
- No in-memory session state (use database-backed sessions)
- No in-process locks for cross-request coordination
- Scheduler must support single-writer or leader election pattern
- Event handling must be durable, not purely in-memory

**Rationale**:
- Home lab / self-hosted use case doesn't need horizontal scaling initially
- Avoiding anti-patterns now prevents costly rewrites later
- Simpler operations for single instance deployment

**Consequences**:
- Some implementation patterns are ruled out (e.g., in-memory caches for session state)
- Event bus decision must consider durability requirement

---

#### ADR-004: NATS + JetStream for Event Bus

**Status**: Accepted

**Context**: Need durable event handling for workflow triggers, step completion, integration events, and internal notifications. Per ADR-003, must be durable (not purely in-memory).

**Decision**: Use NATS with JetStream as the event bus.

**Deployment**: NATS container in Docker Compose with JetStream enabled and persistent storage.

**Usage pattern** (event-driven, not RPC):
- Publish events to subjects: `workflow.completed.{id}`, `integration.email.received`
- Consumers subscribe to patterns with durable subscriptions
- JetStream provides persistence, replay, and exactly-once semantics

**Rust client**: `async-nats` crate

**Rationale**:
- Lightweight (~10-20MB footprint)
- Simple pub/sub model naturally discourages RPC-over-bus patterns
- JetStream adds durability without changing the programming model
- Clean async Rust client
- Subject-based routing with wildcards fits event hierarchies

**Consequences**:
- Additional container in deployment
- Need to manage JetStream streams and consumers
- Events must be designed as fire-and-forget, not request/reply

---

#### ADR-005: Workflow Representation as petgraph with JSONB Storage

**Status**: Accepted

**Context**: Need to define how workflows (graphs of nodes with inputs/outputs) are represented, stored, and how triggers are indexed for efficient execution.

**Decisions**:

1. **Graph structure**: `petgraph::DiGraph<Node, EdgeWeight>` where edge weights contain port routing (source_port, destination_port)

2. **Port typing**: Structural/schema-based using JSON Schema. Connections valid if schemas compatible. Input ports have a required flag; workflow validation fails if required inputs lack incoming edges.

3. **Node categories**: Trigger, AI Layer, Integration, Transform, Control Flow, Memory, Output

4. **Triggers**: Nodes in graph (source of truth) but denormalized to indexed triggers table for execution efficiency. Reconciled on workflow save.

5. **Storage**: Workflow metadata in columns, graph serialized to JSONB. Triggers table indexed by: cron expression (schedule), webhook path (webhook), event type + integration account (event).

6. **IDs**: ULIDs throughout

**Deferred**:
- Expression language for transforms/dynamic config (requirements established, no viable Rust impl identified yet)
- Execution model (graph traversal, parallel handling, state)
- Versioning (change tracking, draft vs published)

**Rationale**:
- petgraph is mature, well-documented Rust graph library
- JSONB allows flexible schema evolution for graph structure
- Denormalized triggers avoid scanning all workflows on every trigger event
- JSON Schema provides structural typing without custom type system

**Consequences**:
- Must serialize/deserialize petgraph to JSON (serde support exists)
- Trigger reconciliation logic needed on every workflow save
- Expression language decision blocks Transform node implementation

---

### 12.2 Pending Decisions

| Decision | Status | Notes |
|----------|--------|-------|
| General API design | **DEFERRED** | Not needed yet; Leptos server functions serve frontend only |
| Expression language | **DEFERRED** | Requirements established; no viable Rust impl yet |
| Workflow execution model | **PARTIAL** | Parallel/FanOut decided; conditional, traversal, state TBD |
| Workflow versioning | **OPEN** | Change tracking, rollback, draft vs published |
| Context persistence strategy | **DECIDED** | See PRD 8.1 and Section 4.2 |

### 12.2 PRD Open Questions Mapping

| PRD Section | Question | Status |
|-------------|----------|--------|
| 8.1 | Conversational Context | **DECIDED** - Hybrid surfacing; explicit-only core; 90-day conversation retention |
| 8.2 | Workflow Representation | **DECIDED** - petgraph + JSONB storage (ADR-005) |
| 8.3 | Graduation Criteria | **PARTIAL** - Working framework; needs refinement before implementation |
| 8.4 | AI Primitive Boundaries | **N/A** - per-node configuration |
| 8.5 | Workflow Execution Patterns | **PARTIAL** - Sequential/parallel decided; conditional TBD |
| 8.6 | State and Memory | **DECIDED** - Workflow memory (AI-managed, per-workflow, opaque bytes) |
| 8.7 | Feedback Granularity | **OPEN** |
| 8.8 | Learning Mechanisms | **OPEN** |
| 8.9 | Multi-User | **DECIDED** - SpiceDB for relationship-based authz (ADR-002) |

---

## References

- [Product Requirements Document](../PRD.md)
- [C4 Model](https://c4model.com/)
- [Mermaid Diagrams](https://mermaid.js.org/)
