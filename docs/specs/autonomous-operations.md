# Autonomous Operations

Workflows that run without user involvement, triggered by schedules.

## Background

### Context

The core value of the assistant is doing work autonomously - monitoring email, preparing briefings, running on a schedule. Users set up workflows once, and they run without further involvement.

### Audience

- **Users**: Create and manage workflows that run on their behalf

### Problem Statements

- Users need to define what the assistant should do autonomously
- Workflows need to run on schedules without user initiation
- Users need visibility into what ran, when, and whether it succeeded
- Users need to be able to debug when things go wrong

## Hypothesis

By providing a visual workflow editor where users connect triggers to AI nodes with tool access, users can define autonomous operations that run on schedule and accomplish tasks they would otherwise do manually.

## Success Criteria

- User can create a workflow that runs on a schedule
- Workflow can access email (read and write) via connected tools
- Workflow can access calendar feeds via connected tools
- Workflow can chain AI nodes for context management
- Workflow can persist and retrieve state across runs
- User can see execution history with errors
- User can enable/disable workflows without deleting them
- User can tune workflow behavior by editing prompts and memory

## Requirements

- **Workflow list**:
  - View all user's workflows
  - Each shows: name, enabled/disabled toggle, last run (datetime, duration, success/failure)
  - Actions: edit, delete (with confirmation)

- **Workflow editor** (visual node-based):
  - Add/remove/connect nodes
  - Configure node properties
  - Connect node outputs to downstream node inputs
  - View and edit raw workflow memory content (for tuning/correcting assistant behavior)

- **MVP node types**:
  - **Trigger node**: Schedule-based (cron syntax), fires connected downstream nodes
  - **AI node**: Prompt configuration, uses connected tools and context inputs, produces output that can feed downstream nodes
  - **Tool node** (read or read+write): Provides an integration as callable tools to a connected AI node; read+write allows actions (send email, update workflow memory, etc.)
  - **Data injection node**: Outputs entire data source content as context to a connected AI node (useful for workflow memory, simple HTTP responses, RSS feeds)

- **Workflow structure**:
  - Trigger fires → downstream nodes execute
  - AI nodes can chain: output of one AI node feeds as input to another
  - AI nodes use connected tools to investigate and act
  - AI nodes receive injected context from data injection nodes
  - AI nodes handle conditional logic internally (no explicit branching nodes for MVP)
  - Chaining enables separation of concerns (e.g., first AI identifies what to process, second AI processes it)

- **Data sources available to nodes**:
  - User-configured integrations (IMAP, Gmail, calendar feeds)
  - Workflow memory (per-workflow persistent state)

- **Execution history**:
  - View past runs for a workflow
  - Each run shows: datetime, duration, success/failure, errors if any
  - Ability to drill into run details for debugging

- **Enable/disable**:
  - Toggle workflow on/off without deleting
  - Disabled workflows don't run on schedule

## Non-requirements

- No event-based triggers (only schedule for MVP)
- No explicit branching/conditional nodes (AI handles logic)
- No agent-suggested workflows (manual creation only for MVP)
- No meta-workflow for automation suggestions (post-MVP)
- No natural language workflow creation (post-MVP)

## Tradeoffs and concerns

*Especially from engineering, what hard decisions will we have to make in order to implement this solution? What future problems might we have to solve because we chose to implement this?*
