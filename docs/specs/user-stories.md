# MVP User Stories

User stories for the silver-telegram MVP, organized by feature area.

## Platform Access

**As a** user,
**I want to** log in using my identity provider,
**So that** I can securely access my personal assistant.

---

**As a** user,
**I want to** log out of the platform,
**So that** my session is terminated when I'm done.

---

**As a** user,
**I want to** configure my timezone,
**So that** scheduled workflows run at the times I expect.

---

**As a** user,
**I want to** access my integration settings from the user menu,
**So that** I can manage my connected services.

---

**As an** admin,
**I want to** see admin functionality in my menu when I have the appropriate group grant,
**So that** I can perform platform oversight tasks.

---

**As an** admin,
**I want to** trigger or cancel workflows belonging to other users,
**So that** I can help manage the platform when needed.

---

**As a** platform operator,
**I want** users without the appropriate OIDC group to be denied access,
**So that** only authorized users can use the platform.

---

## Integrations

**As a** user,
**I want to** see a list of my configured integrations,
**So that** I know what services are connected to my assistant.

---

**As a** user,
**I want to** add a new integration by selecting from available types,
**So that** I can connect a new service to my assistant.

---

**As a** user,
**I want to** search or filter integration types when adding a new one,
**So that** I can quickly find the type I need if the list is long.

---

**As a** user,
**I want to** give my integration a name or label,
**So that** I can distinguish between multiple integrations of the same type (e.g., "Work Gmail" vs "Personal Gmail").

---

**As a** user,
**I want to** connect an IMAP email account,
**So that** my assistant can read and send email on my behalf.

---

**As a** user,
**I want to** connect a Gmail account via OAuth,
**So that** my assistant can read and send email without storing my password.

---

**As a** user,
**I want to** connect a calendar feed,
**So that** my assistant can see my schedule.

---

**As a** user,
**I want to** edit an existing integration,
**So that** I can rename it, update credentials, re-authenticate, or change the URL.

---

**As a** user,
**I want to** see the status of each integration (connected, error),
**So that** I know if something needs attention.

---

**As a** user,
**I want to** delete an integration I no longer need,
**So that** I can keep my configuration clean.

---

**As a** user,
**I want to** be warned when deleting an integration that is used by workflows,
**So that** I don't accidentally break my automations.

---

## Autonomous Operations

**As a** user,
**I want to** see a list of my workflows,
**So that** I know what automations I have set up.

---

**As a** user,
**I want to** see the last run time, duration, and success/failure status for each workflow,
**So that** I know if my automations are working.

---

**As a** user,
**I want to** create a new workflow,
**So that** I can set up a new automation.

---

**As a** user,
**I want to** visually add and connect nodes in the workflow editor,
**So that** I can define what my automation does.

---

**As a** user,
**I want to** add a schedule trigger to my workflow,
**So that** it runs automatically at specified times.

---

**As a** user,
**I want to** add an AI node and configure its prompt,
**So that** the assistant knows what to do when the workflow runs.

---

**As a** user,
**I want to** connect an integration to an AI node as a read-only tool,
**So that** the AI can query that service.

---

**As a** user,
**I want to** connect an integration to an AI node as a read+write tool,
**So that** the AI can both query and take actions on that service.

---

**As a** user,
**I want to** connect a data source to an AI node as injected context,
**So that** the entire content is available without tool calls.

---

**As a** user,
**I want to** chain AI nodes together,
**So that** I can separate concerns (e.g., one identifies what to process, another processes it).

---

**As a** user,
**I want to** use workflow memory to persist state across runs,
**So that** my workflow can remember what it has done (e.g., avoid duplicate notifications).

---

**As a** user,
**I want to** view and edit workflow memory,
**So that** I can tune or correct my assistant's behavior.

---

**As a** user,
**I want to** enable or disable a workflow without deleting it,
**So that** I can pause automations temporarily.

---

**As a** user,
**I want to** delete a workflow,
**So that** I can remove automations I no longer need.

---

**As a** user,
**I want to** confirm before deleting a workflow,
**So that** I don't accidentally delete something important.

---

**As a** user,
**I want to** view the execution history for a workflow,
**So that** I can see what happened on past runs.

---

**As a** user,
**I want to** see errors when a workflow run fails,
**So that** I can debug and fix issues.

---

**As a** user,
**I want to** drill into execution details for a specific run,
**So that** I can understand what happened step by step.
