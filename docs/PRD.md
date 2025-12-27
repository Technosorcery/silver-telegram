# Autonomous Personal Assistant Platform

## Product Requirements Document

**Codename**: silver-telegram

**License**: FSL (Functional Source License)

- Source-available with non-compete restriction
- Self-hosting for personal/internal use unrestricted, with no artificial feature limits
- Converts to Apache 2.0 two years after each release
- Honest about being source-available, not “open source”

-----

## 1. Problem Statement

### 1.1 The Gap in Existing Solutions

Current self-hosted automation and AI agent platforms fall into two categories:

**Workflow Automation Platforms** (n8n, Activepieces, Windmill, Huginn)

- Provide scheduling, triggers, and integrations
- AI capabilities bolted on as “just another node”
- Every task requires defining a workflow upfront
- No natural language interaction for ad-hoc requests

**AI Chat Interfaces** (Open WebUI, Dify, ChatGPT)

- Natural conversation for ad-hoc requests
- No persistent autonomous operation
- Integrations limited or absent
- Can’t “set it and forget it”

**What’s Missing**: A platform where you can have a conversation with an AI assistant that has access to your digital infrastructure (email, calendar, tasks, web), *and* can graduate repeated patterns into autonomous workflows that run without your involvement.

### 1.2 Core Model

**Conversational mode is primary.** You talk to the assistant. It has access to your integrations (email, calendar, etc.), can use tools (search, APIs), and applies AI capabilities (classification, summarization, generation) on demand. No workflow definition required.

**Workflows are for graduating patterns.** When you notice you keep asking for the same kind of thing, or want something to run autonomously on a schedule or trigger, you create a workflow. Workflows are explicit, inspectable, and predictable.

**The assistant can help author workflows.** Describe what you want automated, refine through conversation, get an inspectable artifact that runs on its own.

**A meta-workflow can suggest automations.** Since agents don’t have inherent memory or “deja vu,” pattern recognition is itself an explicit scheduled workflow that reviews conversation history and proposes candidates for automation.

This gives you:

- **Low friction for ad-hoc requests**: Just ask
- **Predictability for recurring patterns**: Explicit workflows you can inspect
- **Graduation path**: Easy to turn conversations into automations
- **Transparency**: Even the “learning” (pattern recognition) is an inspectable workflow

-----

## 2. Product Vision

A self-hosted conversational AI assistant with deep access to your digital infrastructure, capable of handling ad-hoc requests immediately and graduating repeated patterns into autonomous workflows.

**One-sentence pitch**: "A personal AI assistant that can access your email, calendar, and tools, and can automate the things you keep asking for."

-----

## 3. User Personas

### 3.1 Primary: Technical Professional

- Comfortable with self-hosting, Docker, configuration files
- Values being able to inspect and understand what their automations do
- Wants AI to help with the hard parts (classification, generation) but not make unpredictable decisions
- Has specific workflows in mind but doesn’t want to build from scratch

### 3.2 Secondary: Power User

- Can follow technical instructions but not a developer
- Wants to describe what they need and have the system help build it
- Needs to be able to review and approve what gets created
- May not be able to write code but can understand logic when explained

-----

## 4. Example Use Cases

These illustrate both conversational (ad-hoc) and workflow (recurring/autonomous) modes.

### 4.1 Conversational Mode Examples

These are ad-hoc requests handled through natural conversation. No workflow is created; the assistant uses its available capabilities directly.

#### Travel Research

**User**: “I want to visit Iceland in March. When are flights cheapest?”

**Assistant behavior**:

- Uses search/API tools to find flight pricing patterns
- Presents findings conversationally
- Can follow up: “What about hotels?” / “Any events happening then?”

**Capabilities used**: Tool invocation (search, flight APIs), generation (summarization)

-----

#### Calendar Query

**User**: “What’s on my calendar tomorrow? Anything I should prep for?”

**Assistant behavior**:

- Fetches calendar via integration
- Identifies meetings, notes any with attached agendas
- Suggests prep based on meeting types

**Capabilities used**: Integration (calendar), generation (analysis and suggestions)

-----

#### Lunch Recommendations

**User**: “I’m meeting Sarah for lunch downtown. What are some good options?”

**Assistant behavior**:

- Could use search for restaurant options
- If it has context about Sarah’s preferences (from prior conversation or explicit info), factors that in
- Presents options with reasoning

**Capabilities used**: Tool invocation (search), optional context retrieval

-----

#### Quick Lookup

**User**: “Did I get any emails from Acme Corp this week?”

**Assistant behavior**:

- Queries email via integration
- Summarizes findings

**Capabilities used**: Integration (email), generation (summarization)

-----

### 4.2 Workflow Mode Examples

These are recurring patterns that benefit from explicit automation. The user (possibly with assistant help) defines a workflow that runs autonomously.

#### Daily Briefing

**User intent**: “Every morning, prepare a briefing with my calendar, important emails, and due tasks.”

**Resulting workflow**:

1. **Trigger**: Schedule (7:00 AM, user’s timezone)
1. **Fetch**: Calendar events for today (personal + work calendars)
1. **Fetch**: Emails flagged or from priority senders
1. **Fetch**: Tasks due today or overdue
1. **Generate**: Summary combining all inputs with prioritization
1. **Deliver**: Send via email / notification

**Why a workflow**: Runs daily without user initiation; same pattern every time.

-----

#### Misdirected Email Handler

**User intent**: “When I get an email meant for someone else (people confuse me with the CEO), draft a polite redirect.”

**Resulting workflow**:

1. **Trigger**: New email arrives
1. **Classify**: Is this misdirected? (Categories: intended-for-me, misdirected, uncertain)
1. **Branch**:
- If misdirected (high confidence): Generate draft response → Save to Drafts → Notify user
- If uncertain: Notify user for review
- If intended-for-me: No action
1. **Log**: Record classification for later review/feedback

**Why a workflow**: Event-driven; should happen automatically without user checking each email.

-----

#### Interest-Based News Monitoring

**User intent**: “Watch for news about topics I care about and send me a digest, but alert me immediately if something major happens.”

**Resulting workflow**:

1. **Trigger**: Schedule (every 30 minutes)
1. **Fetch**: New items from RSS feeds, configured APIs
1. **For each item**:
- **Classify**: Relevance to interest profile (high/medium/low/none)
- **Classify**: Urgency (breaking/routine)
- **Deduplicate**: Skip if substantially similar to recent item
1. **Branch**:
- High relevance + breaking: Immediate notification
- High/medium relevance + routine: Add to digest queue
1. **Trigger**: Schedule (6:00 PM daily)
1. **Generate**: Compile digest from queue
1. **Deliver**: Send digest

**Why a workflow**: Continuous monitoring; different actions based on urgency; batched delivery.

-----

#### Cross-Calendar Date Night Finder

**User intent**: "Alert me when both my calendar and my spouse's calendar are free on a weekend evening. Potential date night opportunity."

**Resulting workflow**:

1. **Trigger**: Schedule (daily) or Calendar change event
1. **Fetch**: My calendar (next 2 weeks, weekend evenings)
1. **Fetch**: Spouse’s calendar (same window)
1. **Analyze**: Find overlapping free slots
1. **Deduplicate**: Skip if already notified about this slot
1. **Branch**: If new slots found → Notify
1. **Log**: Record notified slots

**Why a workflow**: Proactive detection; runs without user asking.

-----

### 4.3 Graduation Example: Conversation to Workflow

**Initial conversation**:

- User: “What’s on my calendar tomorrow?”
- User (next day): “Show me tomorrow’s calendar”
- User (next day): “Calendar for tomorrow?”

**Meta-workflow detects pattern** (see 4.4):

- “You’ve asked for tomorrow’s calendar 5 times in the past 2 weeks, usually in the evening. Would you like a daily briefing sent at 8 PM?”

**User agrees**, workflow is created:

1. **Trigger**: Schedule (8:00 PM daily)
1. **Fetch**: Calendar for next day
1. **Generate**: Brief summary
1. **Deliver**: Notification

**Graduation benefit**: User no longer has to remember to ask.

-----

### 4.4 Meta-Workflow: Workflow Suggestion

This is itself a workflow, making the “pattern recognition” explicit and inspectable.

**Purpose**: Review conversation history, identify repeated request patterns, propose workflow candidates.

**Workflow**:

1. **Trigger**: Schedule (weekly, Sunday evening)
1. **Fetch**: Conversation logs from past week
1. **Extract**: Request patterns. What did the user ask for repeatedly?
1. **Classify**: Which patterns are candidates for automation?
- Criteria: frequency, predictability, benefit from autonomous operation
1. **Filter**: Exclude patterns that already have workflows
1. **Generate**: Workflow proposals for promising candidates
1. **Deliver**: Present suggestions to user

**User control**:

- Can adjust what counts as “worth automating”
- Can disable this workflow entirely
- Can review and reject suggestions
- Can modify suggested workflows before deploying

**Transparency**: This isn't magic "learning". It's an explicit workflow operating on conversation data. User can inspect it, adjust the thresholds, or turn it off.

-----

### 4.5 Coordination Example: Multi-Step Research

**User initiates**: “Help me plan a trip to Japan in April.”

**Assistant uses coordination** (not a predefined workflow; this is conversational mode with sub-workflow invocation):

1. Invoke flight search sub-workflow → collect options
1. Invoke hotel search sub-workflow → collect options
1. Invoke event/festival lookup → collect relevant events
1. Synthesize findings, present to user
1. User refines: “I prefer Kyoto over Tokyo”
1. Re-run relevant sub-workflows with constraints
1. Present updated options

**Why not a predefined workflow**:

- One-off task, not recurring
- Requires interactive refinement
- Exploratory, not predictable

**Why sub-workflows are useful**:

- Reusable components (flight search, hotel search)
- Parallel execution
- Can be invoked from conversation or from other workflows

-----

## 5. Platform Capabilities

### 5.1 Conversational Interface

The primary interaction mode.

|Capability                 |Description                                                        |
|---------------------------|-------------------------------------------------------------------|
|**Natural Language Input** |User asks questions or makes requests in plain language            |
|**Context Maintenance**    |Conversation history maintained within session                     |
|**Integration Access**     |Assistant can query user’s connected services on demand            |
|**Tool Invocation**        |Assistant can use search, APIs, and other tools                    |
|**AI Primitives on Demand**|Classification, summarization, generation available as capabilities|
|**Workflow Invocation**    |Can trigger existing workflows from conversation                   |
|**Authoring Mode**         |Can switch to workflow authoring when user wants to automate       |

### 5.2 Workflows

For recurring patterns that should run autonomously.

|Capability             |Description                                                          |
|-----------------------|---------------------------------------------------------------------|
|**Workflow Definition**|Declare a sequence of steps with branching, loops, and error handling|
|**Workflow State**     |Persist execution state for long-running or resumable workflows      |
|**Workflow Inspection**|View the definition in readable form; understand what it will do     |
|**Execution History**  |See past runs, inputs, outputs, and decisions at each step           |

### 5.3 Triggers

What causes workflows to execute.

|Capability          |Description                                                   |
|--------------------|--------------------------------------------------------------|
|**Schedule**        |Cron-style time-based triggers with timezone support          |
|**Event**           |React to external events (new email, calendar change, webhook)|
|**Condition**       |Trigger when a monitored condition becomes true               |
|**Manual**          |User-initiated execution (from UI or conversation)            |
|**Sub-workflow**    |Triggered by another workflow or conversational request       |
|**Missed Execution**|Configurable: skip, run immediately, or run at next window    |

### 5.4 AI Primitives

AI-powered operations available both in conversation and as workflow steps.

|Primitive      |Input                       |Output               |Example Use                        |
|---------------|----------------------------|---------------------|-----------------------------------|
|**Classify**   |Content + categories        |Category + confidence|"Is this email misdirected?"       |
|**Extract**    |Content + schema            |Structured data      |"Pull out dates, locations, budget"|
|**Generate**   |Context + instructions      |Text                 |"Write a polite redirect email"    |
|**Summarize**  |Content + constraints       |Condensed text       |"Summarize these 10 emails"        |
|**Score**      |Content + criteria          |Numeric score        |"How relevant to my interests?"    |
|**Deduplicate**|Item + recent items         |Is duplicate (bool)  |"Have I seen this already?"        |
|**Decide**     |Context + options + criteria|Selected option      |"Which response is best?"          |
|**Coordinate** |Goal + available tools      |Final result         |"Plan this trip" (multi-step research)|

**Note on primitives**: At the implementation level, most primitives above (Classify, Extract, Generate, Summarize, Score, Deduplicate, Decide) are variations of a single LLM call with different prompts and output schemas. **Coordinate** is architecturally distinct: it's an LLM-driven execution loop where the model decides what actions to take, executes them, evaluates results, and repeats until the goal is achieved.

### 5.5 Integration Framework

Access to external services, available to both conversation and workflows.

|Capability            |Description                                                              |
|----------------------|-------------------------------------------------------------------------|
|**Service Connectors**|Pre-built integrations for common services (email, calendar, tasks, etc.)|
|**Protocol Support**  |IMAP, JMAP, CalDAV, REST, GraphQL, webhooks                              |
|**Authentication**    |OAuth flows, token refresh, secure credential storage                    |
|**Multi-Account**     |Multiple accounts of same service type                                   |
|**Read and Write**    |Bidirectional: fetch data and take actions                               |
|**Rate Limiting**     |Respect external API constraints                                         |
|**Custom Connectors** |Define new integrations without modifying core platform                  |

### 5.6 Workflow Authoring

How workflows get created.

|Capability                  |Description                                                      |
|----------------------------|-----------------------------------------------------------------|
|**Conversational Authoring**|Describe intent in natural language, refine through dialogue     |
|**Example-Based Refinement**|Provide examples of inputs and desired outputs                   |
|**Template Library**        |Start from common patterns, customize                            |
|**Test Execution**          |Run workflow with sample data before deploying                   |
|**Graduation Prompts**      |Meta-workflow suggests automations based on conversation patterns|

### 5.7 Human-in-the-Loop

Controls for when automation should pause for human input.

|Capability            |Description                                   |
|----------------------|----------------------------------------------|
|**Approval Steps**    |Pause execution pending human approval        |
|**Review Queues**     |Present outputs for review before final action|
|**Feedback Capture**  |Accept/reject/modify signals on outputs       |
|**Confidence Routing**|Different paths based on AI confidence level  |

### 5.8 Observability

Understanding what happened and why.

|Capability              |Description                                           |
|------------------------|------------------------------------------------------|
|**Execution Logs**      |What happened, when, with what inputs/outputs         |
|**Decision Trace**      |For AI primitives: why this classification/generation?|
|**Error Reporting**     |Clear errors with actionable information              |
|**Metrics**             |Success rate, latency, resource usage                 |
|**Alerting**            |Notify on failures or anomalies                       |
|**Conversation History**|Searchable log of past conversations                  |

### 5.9 Learning and Improvement

How the system gets better over time (all explicit, not magic).

|Capability                |Description                                                  |
|--------------------------|-------------------------------------------------------------|
|**Feedback Collection**   |Store user feedback on AI primitive outputs                  |
|**Conversation Analysis** |Meta-workflow to identify automation candidates              |
|**Refinement Suggestions**|"You've rejected 5 classifications like this. Want to adjust?"|
|**Model Improvement Path**|Mechanism to incorporate feedback into model behavior        |
|**A/B Comparison**        |Test workflow variations against each other                  |

-----

## 6. Non-Functional Requirements

### 6.1 Deployment

|Requirement           |Description                                             |
|----------------------|--------------------------------------------------------|
|**Self-Hosted**       |Runs entirely on user-controlled infrastructure         |
|**Single-Node Viable**|Must work on single server (home lab use case)          |
|**Containerized**     |Standard container deployment                           |
|**Offline Capable**   |Core functionality works without internet (local models)|
|**Resource Efficient**|Reasonable footprint when idle                          |

### 6.2 Privacy and Security

|Requirement            |Description                                     |
|-----------------------|------------------------------------------------|
|**Data Locality**      |All data stays on user infrastructure by default|
|**Credential Security**|Encrypted storage, no plaintext credentials     |
|**Minimal Permissions**|Request only needed access scopes               |
|**No Telemetry**       |No data sent to platform developers             |
|**Audit Logging**      |Track all access to sensitive data              |

### 6.3 Reliability

|Requirement              |Description                                    |
|-------------------------|-----------------------------------------------|
|**Crash Recovery**       |Automatic restart with state preservation      |
|**Idempotent Operations**|Safe to retry failed operations                |
|**Graceful Degradation** |Partial functionality if components unavailable|
|**Queue Durability**     |Pending tasks survive restarts                 |
|**Health Checking**      |Self-monitoring with alerting                  |

### 6.4 Extensibility

|Requirement            |Description                                 |
|-----------------------|--------------------------------------------|
|**Custom Connectors**  |Add new service integrations                |
|**Custom Primitives**  |Define new AI primitives beyond built-in set|
|**Plugin Architecture**|Clean extension points                      |
|**API-First**          |All functionality accessible via API        |

### 6.5 Usability

|Requirement               |Description                                  |
|--------------------------|---------------------------------------------|
|**Sensible Defaults**     |Works out of box for common cases            |
|**Progressive Disclosure**|Simple things simple, complex things possible|
|**Documentation**         |Comprehensive, example-rich documentation    |
|**Actionable Errors**     |Error messages that tell you what to do      |

-----

## 7. Key Differentiators from Existing Solutions

### 7.1 vs. Workflow Platforms (n8n, Activepieces, Windmill)

|They Provide                           |This Platform Adds                                                   |
|---------------------------------------|---------------------------------------------------------------------|
|Visual/code workflow builder           |Conversational interface for ad-hoc requests                         |
|AI as just another node                |AI primitives as first-class vocabulary                              |
|Every task requires workflow definition|Ad-hoc requests handled immediately, workflows for recurring patterns|
|No authoring assistance                |Conversational workflow creation and graduation prompts              |

### 7.2 vs. Chat Interfaces (Open WebUI, ChatGPT)

|They Provide           |This Platform Adds                               |
|-----------------------|-------------------------------------------------|
|Conversational AI      |Deep integration with personal infrastructure    |
|Limited/no integrations|Email, calendar, tasks, and extensible connectors|
|Session-based only     |Autonomous workflows that run without user       |
|No pattern graduation  |Repeated requests can become automations         |

### 7.3 vs. Agent Frameworks (LangGraph, CrewAI)

|They Provide                   |This Platform Adds                 |
|-------------------------------|-----------------------------------|
|Sophisticated agent reasoning  |Explicit, inspectable workflows    |
|Requires external orchestration|Built-in scheduling and triggers   |
|Open-ended autonomy            |Bounded AI within defined structure|
|Code-only definition           |Conversational authoring           |

### 7.4 vs. AI Platforms (Dify)

|They Provide             |This Platform Adds                |
|-------------------------|----------------------------------|
|Chat-centric interaction |Both chat and autonomous operation|
|Conversational AI        |Workflow graduation path          |
|Limited integration story|Deep integration framework        |
|No workflow coordination |Sub-workflow composition          |

-----

## 8. Open Questions

### 8.1 Conversational Context

**Status**: Decided (v1)

#### Decision

**Categories of context**:

| Category | Description | Source label |
|----------|-------------|--------------|
| **Conversation history** | Raw messages exchanged | N/A |
| **Facts** | Structured knowledge learned from conversation | `explicit` (user stated) or `inferred` |
| **Corrections/feedback** | User overrides and refinements | N/A |
| **Workflow execution history** | What ran, when, inputs/outputs | Queryable from conversation, not injected |

**Retention policy**:

| Category | Retention | Rationale |
|----------|-----------|-----------|
| Conversation history | 90 days rolling | Enough for meta-workflow pattern detection; patterns graduate to workflows/facts |
| Facts | Until contradicted or manually deleted | Stable truths; outlive source conversation |
| Corrections/feedback | Permanent | Training signal; small volume; high value |
| Workflow execution history | User-configurable (default 90 days) | Audit/debugging |

**Cross-session surfacing**:

- **Hybrid approach**: Core context always injected into LLM calls; everything else retrieved on-demand via semantic search
- **Core definition (v1)**: Explicitly marked facts only. User says "remember that..." → core. No auto-promotion.

#### Deferred: Future promotion strategies

**Recency-based promotion** (strawman):
- Facts referenced in last N sessions auto-promote to core
- Open question: What constitutes "referenced"? Retrieved ≠ referenced.
- Options to explore: explicit citation, user confirmation, heuristics

**Typed category promotion** (alternative):
- Certain fact types always core: identity, constraints, active projects
- Open question: Context-dependent relevance. Allergies are core for lunch plans but irrelevant for scheduling online meetings.
- Implies retrieval might need task/intent awareness, not just semantic similarity

#### Key insight

> Relevance is context-dependent. A fact being "important" isn't binary; it depends on what the user is trying to do. Future work on smart retrieval should consider intent, not just semantic similarity.

### 8.2 Workflow Representation

**Question**: What artifact results from workflow authoring?

Options:

- **Code**: Full programming language, maximum flexibility, requires dev skills to edit
- **Declarative config**: YAML/JSON/TOML, readable but limited expressiveness
- **Visual graph**: GUI-editable, accessible but harder to version control
- **DSL**: Purpose-built language, balanced but another thing to learn
- **Hybrid**: High-level structure in config, complex logic in code

Considerations:

- Must be inspectable (user can understand what it does)
- Authoring agent needs to produce it
- User may need to edit it manually

### 8.3 Graduation Criteria

**Status**: Working framework (needs refinement before implementation)

#### Core insight

Graduation criteria are interdependent. "Frequency", "predictability", and "autonomy benefit" can't be evaluated independently. A pattern is graduatable based on a holistic assessment.

#### Automation flavors

Graduation produces different workflow types depending on the pattern:

| Flavor | Trigger | Inputs | Invoker | Example |
|--------|---------|--------|---------|---------|
| **Autonomous** | Schedule/event | Derived from trigger or stable | System | Daily briefing at 7am |
| **Invoked workflow** | Explicit call | Parameterized | User, agent, or other workflow | "Research trip to {destination}" |
| **Suggested action** | System detects opportunity | Derived from context | System prompts, user confirms | "Free evening detected. Date night?" |

#### Detection signals (draft)

| Pattern signal | Suggests |
|----------------|----------|
| Same time of day / day of week | Autonomous candidate (schedule trigger) |
| Same process, varying inputs | Invoked workflow candidate (parameterized) |
| Same trigger event, same response | Autonomous candidate (event trigger) |
| Consistent process but requires clarification | Not yet automatable |

#### Graduation decision flow (draft)

1. **Is the process stable?** Same steps, same integrations across instances
2. **Are inputs predictable?**
   - Yes → autonomous candidate
   - No, but parameterizable → invoked workflow
   - No, requires conversation → not ready for graduation
3. **Is there a trigger pattern?**
   - Temporal regularity → schedule trigger
   - Event correlation → event trigger
   - Ad-hoc → manual/callable trigger

#### Open for implementation planning

- Specific thresholds (N occurrences in M days)
- Semantic similarity measurement for "same request"
- Disqualification heuristics
- User configurability of thresholds

### 8.4 AI Primitive Boundaries

**Status**: Decided

**Decision**: Per-node configuration. Each node instance specifies its own constraint level rather than having a platform-wide policy.

For example, "Classify" can be configured per-use as:

- **Constrained**: Pick from exactly these categories
- **Semi-constrained**: Pick from these categories or suggest a new one
- **Unconstrained**: Determine appropriate categories

This allows workflows to choose the appropriate trade-off for each use case:

- More constraint = more predictable, easier to debug
- Less constraint = more capable, more flexible

The constraint level is part of the node's configuration, not a global setting.

### 8.5 Workflow Execution Patterns

**Status**: Partially decided (sequential, parallel decided; conditional TBD)

These are patterns the workflow engine handles for static workflow graphs (distinct from the Coordinate AI primitive, which handles dynamic LLM-driven orchestration).

#### Decided patterns

| Pattern | Mechanism | Vec<T> handling |
|---------|-----------|-----------------|
| **Sequential** | Linear graph edges | Passed as-is |
| **Graph parallel** | Multiple outgoing edges (no FanOut) | Copied as-is to each downstream node |
| **FanOut parallel** | FanOut node iterates over items | Exploded into individual items |
| **Combined** | FanOut + multiple outgoing edges | Each item sent to all downstream nodes |

#### FanOut node

- **Inputs**: Waits for all inputs (barrier behavior), flattens arrays into single collection
- **Validation**: Each input item must be compatible with schemas of ALL direct children
- **Execution**: Each item processed through all downstream paths in parallel

#### FanIn node

- **Optional**: FanOut doesn't require a paired FanIn (valid to fan out for side effects only)
- **Scope**: Edge from FanIn to its corresponding FanOut defines scope (no ID matching needed)
- **Validation**: All inputs must have non-empty common schema intersection
- **Output**: `Vec<CommonSchema>`

#### Validation model

| Stage | Check |
|-------|-------|
| **Construction** | Port schema compatibility between connected nodes |
| **Runtime** | Each node validates its output against its declared output schema |

Runtime validation at the source (node output) rather than destination (node input) makes debugging clearer: "Node X failed to satisfy its output schema."

#### Open

- **Conditional**: Branch node with predicate per outgoing edge (not yet discussed)

### 8.6 State and Memory

**Status**: Decided (v1)

#### Workflow memory

Workflows can persist state across runs via **workflow memory** - an opaque store that AI agents manage.

**Mechanism:**

- **Load Workflow Memory** node: Retrieves stored memory, feeds into downstream AI node context
- **Record Workflow Memory** node: AI rewrites memory based on current state and update instructions

**Design decisions:**

| Aspect | Decision |
|--------|----------|
| Scope | Per-workflow ID |
| Format | Opaque bytes; AI determines and maintains structure |
| Update semantics | Full rewrite (AI curates what to keep) |
| Size limit | Platform-enforced; small enough to always fit in LLM context |
| Atomicity | Each Record node is a transaction (completes → persisted) |
| Checkpointing | Multiple Record nodes in a workflow enable intermediate saves |

**Record Workflow Memory node:**

| | Name | Type | Description |
|--|------|------|-------------|
| config | `update_instructions` | string | Prompt guiding how AI should maintain memory |
| input | `workflow_output` | any | Output from upstream nodes to inform update |
| output | `memory` | bytes | Updated memory (also persisted) |

Note: Current memory is loaded implicitly by the node, not a user-wired input.

**Example uses:**

- **Deduplication**: Memory stores identifiers of previously processed items; AI checks incoming items against memory
- **Notification tracking**: Memory records what user has been notified about to avoid repeats
- **Accumulated context**: Memory builds up relevant facts across runs for richer AI decisions

#### Deferred

- File-like tooling for large memories (if "fits in context" limit proves insufficient)
- Memory versioning / rollback

### 8.7 Feedback Granularity

**Status**: Decided

#### Decision

All explicit feedback levels available, none required:

| Level | Example | When useful |
|-------|---------|-------------|
| **Per-output** | "This classification was wrong" | Correcting specific AI decisions |
| **Per-interaction** | "That answer was helpful" | Rating conversational responses |
| **Per-workflow-run** | "This run succeeded/failed" | Evaluating overall workflow behavior |

Users can provide feedback at whatever granularity makes sense in the moment. No feedback level is mandatory.

#### Implicit feedback rejected

Implicit feedback (inferring from user behavior) is not supported:

- **Inaction is ambiguous**: User ignoring a notification could mean disagreement, or they're busy, or they didn't see it
- **Action requires external visibility**: "User edited the draft" happens in email clients or other systems the platform doesn't control

### 8.8 Learning Mechanisms

**Status**: Decided

#### Mechanism

The meta-workflow reviews interaction history (which includes feedback) and suggests changes. All suggestions are explicit and require user approval.

This is the same meta-workflow described in Section 4.4 (Workflow Suggestion). Graduation is one learning outcome among several.

#### Possible outcomes

| Outcome | Description | Example suggestion |
|---------|-------------|-------------------|
| **New workflow** | Graduation of repeated pattern | "You ask for tomorrow's calendar every evening. Create a daily briefing?" |
| **Workflow structure** | Add, remove, or reorder nodes | "Your news workflow misses sports. Add a sports feed node?" |
| **Prompt refinement** | Change instructions to AI nodes | "Your classifier keeps missing newsletters. Refine the prompt?" |
| **Threshold adjustment** | Change confidence routing | "You've approved 5 'uncertain' classifications. Lower the threshold?" |
| **Model training** | Recommend fine-tuning | "You have enough feedback to train a custom classifier. Proceed?" |

#### Key principle

All learning is explicit and user-controlled. The meta-workflow detects patterns and suggests changes; the user reviews and approves. No automatic model updates or silent behavior changes.

### 8.9 Multi-User Considerations

**Status**: Decided (foundational architecture)

#### Decision

Multi-user authorization uses SpiceDB (Zanzibar-style relationship-based authorization). See [ADR-002](architecture/ARCHITECTURE.md#adr-002-spicedb-for-relationship-based-authorization) for full rationale.

#### How it addresses the considerations

| Consideration | Approach |
|---------------|----------|
| **Shared vs. personal workflows** | Relationships in SpiceDB: `workflow:123#viewer@user:spouse` |
| **Shared vs. personal integrations** | Same pattern: `integration:family-calendar#user@user:spouse` |
| **Privacy between users** | Default isolation; sharing requires explicit relationship |
| **Conflicting preferences** | Per-user facts/preferences; shared resources have owner who resolves conflicts |

#### Deferred

Detailed permission model (what granular permissions exist beyond owner/viewer) deferred until needed.

-----

## 9. Success Metrics

### 9.1 Conversational Effectiveness

- Request completion rate (did the assistant accomplish what was asked?)
- Turns to resolution (fewer = better understanding)
- Integration coverage (how many services connected and used)
- Tool invocation success rate

### 9.2 Workflow Graduation

- Graduation rate (conversations → workflows)
- User acceptance of graduation suggestions
- Time from pattern emergence to workflow creation

### 9.3 Workflow Execution

- Workflow completion rate
- Error rate by primitive type
- Human intervention rate (lower = better for trusted workflows)

### 9.4 Learning Effectiveness

- Classification accuracy over time (should improve)
- User override frequency (should decrease)
- Feedback volume (engagement signal)
- Meta-workflow suggestion acceptance rate

### 9.5 Reliability

- Uptime
- Missed scheduled executions
- Mean time to recovery
- Data loss incidents (target: zero)

-----

## 10. Out of Scope (v1)

- **Granular permissions model**: OIDC authentication from the start, but permissions beyond "logged in or not" come later
- **Mobile apps**: Web/API access only
- **Voice interface**: Text-based interaction
- **Real-time collaboration**: Async operation model
- **Marketplace**: No workflow/connector sharing infrastructure
- **Hosted offering**: Self-hosted only

-----

## Appendix A: Existing Platform Limitations Summary

|Platform    |Key Limitation                                             |
|------------|-----------------------------------------------------------|
|n8n         |Fair-code license; no authoring assistance; AI bolted on   |
|Activepieces|No native local LLM; no learning; no authoring assistance  |
|Windmill    |AGPL + proprietary; SMTP-only email; enterprise lock-in    |
|Huginn      |Unmaintained since 2022; no AI capabilities                |
|Temporal    |Enterprise complexity; no AI-native features               |
|LangGraph   |No scheduling; no integrations; library not platform       |
|CrewAI      |No scheduling; no integrations; multi-agent focus only     |
|Dify        |Chat-centric; no autonomous operation; limited coordination|

-----

## Appendix B: Glossary

|Term                   |Definition                                                                                       |
|-----------------------|-------------------------------------------------------------------------------------------------|
|**Conversational Mode**|The primary interaction mode. User asks questions, assistant responds using available capabilities.|
|**Workflow**           |An explicit, inspectable sequence of steps that runs autonomously on triggers                    |
|**AI Primitive**       |A bounded AI operation (classify, extract, generate, etc.) usable in conversation or workflows   |
|**Trigger**            |An event or condition that causes a workflow to execute                                          |
|**Connector**          |An integration with an external service (email, calendar, etc.)                                  |
|**Coordinate**         |AI primitive: LLM-driven execution loop that decides what actions to take, executes, evaluates, and repeats|
|**Graduation**         |The process of turning a repeated conversational pattern into an autonomous workflow             |
|**Meta-workflow**      |A workflow that operates on the platform itself (e.g., analyzing conversation patterns)          |
|**Feedback**           |User signal about AI primitive output quality                                                    |
|**Authoring Agent**    |The AI capability that helps users create and refine workflows                                   |
|**Human-in-the-Loop**  |Pausing workflow execution for human review/approval                                             |
