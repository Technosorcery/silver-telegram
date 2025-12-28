# Integrations

Connecting external services (data sources and sinks) for the assistant to access.

## Background

### Context

The assistant needs access to external services to be useful - email accounts to triage, calendars to check for scheduling, feeds to monitor. Integrations are the connections to these services, storing credentials and connection details.

### Audience

- **Users**: Configure their personal integrations (their email accounts, their calendars)

### Problem Statements

- Users need to connect their external services to the platform
- Different services require different authentication methods (OAuth, username/password, API tokens, URLs)
- Users need to manage (edit, delete) integrations over time
- Deleting an integration used by workflows could break those workflows

## Hypothesis

By providing a straightforward interface to connect external services with appropriate credential handling per service type, users can give the assistant access to the data sources and sinks it needs to be useful.

## Success Criteria

- User can connect an email account (IMAP or Gmail)
- User can connect a calendar feed
- User can see which integrations exist and manage them
- User is warned before deleting an integration that would affect workflows

## Requirements

- **Integration list**:
  - View all configured integrations
  - Each shows: name/label, type, status (connected/error)
  - Actions: edit, delete

- **Create integration**:
  - Select from available integration types
  - Search/filter if list is long
  - Type-specific configuration flow (OAuth redirect for Gmail, form for IMAP credentials, URL for feeds)
  - User provides name/label to distinguish multiple integrations of same type

- **Edit integration**:
  - Change name/label
  - Re-authenticate (OAuth types)
  - Replace credentials (username/password, API token types)
  - Change URL (feed types)

- **Delete integration**:
  - If integration is referenced by workflows, show warning with list of affected workflows
  - User can proceed or cancel

- **MVP integration types**:
  - IMAP (email read/write) - username, password, server, port
  - Gmail (email read/write) - OAuth flow
  - Calendar feeds (read) - URL (iCal/CalDAV)

- **Integration properties**:
  - Type
  - Name/label (user-provided)
  - Credentials/connection details (type-specific)
  - Status (connected, error, etc.)

## Non-requirements

- No shared integrations (MVP is personal only; sharing via SpiceDB comes later)
- No polling frequency configuration (that's per-workflow, not per-integration)
- No Pushover or other notification sinks (email serves as output for MVP)
- No RSS/general feed monitoring (calendar feeds only for MVP)

## Tradeoffs and concerns

*Especially from engineering, what hard decisions will we have to make in order to implement this solution? What future problems might we have to solve because we chose to implement this?*
