# AI Node Configuration

Configuring LLM providers at the platform level and selecting models for AI nodes in workflows via auto-discovery.

## Background

### Context

silver-telegram uses AI nodes as the fundamental primitive for all AI-powered operations in workflows (classify, generate, summarize, extract, etc.). These AI nodes need to execute against actual LLM providers—either local (Ollama) or cloud-based.

Currently, the architecture defines an "LLM Backend" component with "provider abstraction," but there's no specification for how users configure which providers are available or how workflow authors select which provider/model an AI node should use.

Without this capability, AI nodes cannot function—there's no way to specify where they should send their prompts.

### Audience

- **Technical Users**: Configure LLM provider integrations (Ollama endpoints, API keys for cloud providers)
- **Workflow Authors**: Select which provider/model to use for AI nodes when building workflows

### Problem Statements

- Users have no way to configure LLM providers (Ollama, cloud APIs) at the platform level
- Workflow authors have no way to specify which model an AI node should use
- There's no mechanism to discover available models from a configured provider
- Multiple AI nodes that should use the same model require repetitive configuration if model selection is per-node

## Hypothesis

By treating LLM providers as integrations and introducing an OpenAIModel node that connects to AI nodes, users will be able to:

1. Configure their preferred LLM providers once at the platform level
2. Easily select from auto-discovered models when building workflows
3. Change the model for multiple AI nodes by modifying a single OpenAIModel node

This follows the existing integration pattern (familiar to users) while keeping workflow graphs explicit about which model powers each AI operation.

## Success Criteria

- User can add an OpenAI-compatible integration instance with an endpoint URL
- User can view auto-discovered models from a configured instance when editing a workflow
- User can create an OpenAIModel node in a workflow and connect it to one or more AI nodes
- Workflow validation fails if an AI node has no connected model node
- Changing an OpenAIModel node's selected model updates all connected AI nodes
- Model discovery completes within reasonable time (< 5 seconds for typical Ollama instance)

## Requirements

### Platform-Level: LLM Provider Integration

**Integration type: `openai_compatible`**

| Aspect | Requirement |
|--------|-------------|
| **Configuration fields** | Endpoint URL (required), display name (required), API key (optional) |
| **Multiple instances** | Allowed (local Ollama, cloud OpenAI, other compatible endpoints) |
| **Connection test** | Validate endpoint responds to model list API |
| **Protocol** | OpenAI API (`/v1/models`, `/v1/chat/completions`) |

**Model discovery endpoint:**

| Aspect | Requirement |
|--------|-------------|
| **API** | `GET /v1/models` (OpenAI-compatible) |
| **Response handling** | Parse model list, extract model IDs and names |
| **Error handling** | Surface connection errors clearly to user |

### Workflow-Level: Model Nodes

**Protocol-based node types**

Model node types are based on API protocol, not provider brand. Providers using the same API protocol share a node type.

**v1 node type: `OpenAIModel`**

| Aspect | Requirement |
|--------|-------------|
| **Category** | Configuration (or new category if needed) |
| **Purpose** | Specifies which OpenAI-compatible instance and model to use |
| **Works with** | Any OpenAI-compatible API (Ollama, OpenAI, LocalAI, vLLM, etc.) |

**Node configuration:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `integration_id` | reference | yes | Which OpenAI-compatible integration instance |
| `model_id` | string | yes | Which model (auto-discovered from instance) |

**Node ports:**

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `model` | output | `ModelReference` | Connection point for AI nodes |

**`ModelReference` type:**

```
{
  integration_id: string,
  model_id: string
}
```

**AI Node changes:**

| Aspect | Requirement |
|--------|-------------|
| **New input port** | `model` (type: `ModelReference`, required) |
| **Accepts connection from** | Any protocol-specific model node (`OpenAIModel`, future `AnthropicModel`, etc.) |
| **Validation** | Workflow invalid if AI node's `model` port has no incoming edge |

### Model Selection UX (Workflow Editor)

| Aspect | Requirement |
|--------|-------------|
| **Integration picker** | Dropdown of configured `openai_compatible` integrations |
| **Model picker** | Dropdown populated by on-demand discovery from selected integration |
| **Discovery trigger** | When user selects an integration, or clicks refresh |
| **Loading state** | Show loading indicator during model discovery |
| **Error state** | Show error message if discovery fails (with retry option) |
| **Caching** | Cache discovered models for duration of editing session |

### Workflow Execution

| Aspect | Requirement |
|--------|-------------|
| **Model resolution** | At execution time, OpenAIModel node outputs the provider endpoint + model ID |
| **AI node execution** | AI node uses the connected model node's output to make LLM API calls |
| **Missing model** | If referenced model no longer exists on provider, fail with clear error |

### General Capabilities (Already Implemented)

| Capability | Status | Notes |
|------------|--------|-------|
| **Node renaming** | Already implemented | All nodes have a `name`/`label` field editable in the config panel |

## Non-requirements

- **No temperature/max_tokens/top_p settings** — deferred beyond v1
- **No cost controls or budget limits** — deferred
- **No platform-wide default model** — model selection is explicit per workflow
- **No workflow-level default model** — each AI node must have explicit model connection
- **No non-OpenAI-compatible providers** — Anthropic, etc. deferred beyond v1
- **No continuous/background model discovery** — on-demand only
- **No model capability metadata** — no filtering by context window, vision support, etc.

## Tradeoffs and concerns

*Especially from engineering, what hard decisions will we have to make in order to implement this solution? What future problems might we have to solve because we chose to implement this?*
