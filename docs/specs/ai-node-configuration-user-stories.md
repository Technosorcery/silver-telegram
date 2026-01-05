# AI Node Configuration User Stories

User stories for the [AI Node Configuration](ai-node-configuration.md) spec.

## LLM Provider Integration

**As a** user,
**I want to** add an OpenAI-compatible LLM provider integration,
**So that** my workflows can use AI capabilities from that provider.

---

**As a** user,
**I want to** configure just an endpoint URL for local providers like Ollama,
**So that** I can connect without unnecessary API key configuration.

---

**As a** user,
**I want to** optionally provide an API key for cloud providers,
**So that** I can authenticate with services that require it.

---

**As a** user,
**I want to** test the connection when adding or editing an LLM provider integration,
**So that** I know the endpoint is reachable before saving.

---

**As a** user,
**I want to** add multiple LLM provider integrations,
**So that** I can use different providers for different workflows (e.g., local Ollama for testing, cloud for production).

---

**As a** user,
**I want to** see LLM provider integrations in my integrations list alongside email and calendar,
**So that** I can manage all my connections in one place.

---

## Model Selection in Workflows

**As a** user,
**I want to** add an OpenAIModel node to my workflow,
**So that** I can specify which LLM provider and model my AI nodes should use.

---

**As a** user,
**I want to** select from my configured LLM provider integrations in the model node,
**So that** I can choose which provider to use.

---

**As a** user,
**I want to** see available models auto-discovered from the selected provider,
**So that** I don't have to manually type model names.

---

**As a** user,
**I want to** see a loading indicator while models are being discovered,
**So that** I know the system is working.

---

**As a** user,
**I want to** see a clear error message if model discovery fails,
**So that** I can troubleshoot connection issues.

---

**As a** user,
**I want to** retry model discovery if it fails,
**So that** I can recover from transient errors.

---

**As a** user,
**I want to** connect an OpenAIModel node to one or more AI nodes,
**So that** those AI nodes know which model to use for execution.

---

**As a** user,
**I want to** connect one OpenAIModel node to multiple AI nodes,
**So that** I can change the model for all of them by editing one node.

---

## Workflow Validation

**As a** user,
**I want** workflow validation to fail if an AI node has no connected model node,
**So that** I don't deploy a workflow that can't execute.

---

**As a** user,
**I want to** see a clear validation error indicating which AI node is missing a model connection,
**So that** I know exactly what to fix.

---

## Workflow Execution

**As a** user,
**I want** my workflow to fail with a clear error if the selected model no longer exists on the provider,
**So that** I can update the model selection.

---
