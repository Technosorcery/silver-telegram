# CLAUDE.md

This file provides guidance to Claude Code when working on silver-telegram.

## Project Overview

**silver-telegram** is an autonomous personal assistant platform that combines conversational AI with autonomous workflows. It provides a natural language interface to interact with integrations (email, calendar, tasks) and can graduate repeated conversation patterns into autonomous workflows.

**Key Concepts:**

- **Conversational Mode**: Primary interaction mode where users make ad-hoc requests
- **Workflows**: Explicit, inspectable automation for recurring patterns
- **Graduation**: Turning repeated conversational patterns into autonomous workflows
- **AI Primitives**: Bounded AI operations (classify, extract, generate, summarize, etc.)

See `docs/PRD.md` for complete product requirements.

## Tech Stack

- **Language**: Rust (edition 2024)
- **Web Framework**: Leptos (full-stack SSR + WASM)
- **Server**: Axum
- **Error Handling**: rootcause
- **Database**: SQLite with SQLx (planned)

## Development Commands

### Starting Development Server

```bash
cargo leptos watch
```

Access at http://localhost:3000

### Testing

```bash
# Run all tests
cargo test

# Run tests with SSR features
cargo test --features ssr
```

### Building

```bash
# Development build
cargo leptos build

# Release build with optimized WASM
cargo leptos build --release

# Server binary only
cargo build --bin silver-telegram-server --no-default-features --features=ssr
```

### Linting

```bash
# Run clippy (strict mode - zero warnings allowed)
cargo clippy -- -D warnings

# With SSR features
cargo clippy --features ssr -- -D warnings
```

## Code Quality Standards

### Clippy Linting Policy

**CRITICAL: All code must be clippy-lint-free with zero warnings.**

- **NEVER add `#[allow(...)]` attributes** without explicit authorization
- **ALL clippy violations must be addressed at their root cause**
- **Authorization required**: Any `#[allow(...)]` must be explicitly approved
- **Exception**: `#[allow(non_snake_case)]` is permitted at crate level for Leptos components

### Test-Driven Development

- **ALWAYS write the test first** - No exceptions
- **Watch it fail** - Verify test fails before implementation
- **Minimal implementation** - Write just enough code to pass
- **Refactor** - Clean up while keeping tests green

### Type-Driven Error Handling

Using rootcause for error handling:

- **Domain Errors**: Business logic violations with domain-specific names
  - Examples: `WorkflowError`, `IntegrationError`, `ConversationError`
  - **NEVER use generic names** like `DomainError` or `Error`
- **Use `.context()` with typed error variants**, not string literals
- **No `.unwrap()`** - Use `.expect()` only in tests

Example pattern:

```rust
use rootcause::prelude::{Report, ResultExt};

#[derive(Debug)]
pub enum WorkflowError {
    NotFound { id: String },
    InvalidState { from: String, to: String },
}

fn get_workflow(id: &str) -> Result<Workflow, Report<WorkflowError>> {
    db.find(id)
        .context(WorkflowError::NotFound { id: id.to_string() })?
}
```

## Project Structure

```
silver-telegram/
├── Cargo.toml              # Workspace root
├── rust-toolchain.toml     # Pins Rust version
├── docs/
│   └── PRD.md              # Product requirements
├── lib/
│   └── core/               # Core domain types and errors
│       └── src/
│           ├── lib.rs
│           └── error.rs
└── bin/
    └── server/             # Leptos web application
        └── src/
            ├── main.rs     # Axum server entry
            ├── lib.rs      # WASM hydration entry
            └── app.rs      # Leptos App component
```

### Adding New Crates

- **Supporting libraries**: `lib/<crate-name>/`
- **Services/binaries**: `bin/<crate-name>/`

## Leptos Architecture

This project uses Leptos, a full-stack Rust framework that compiles to:

1. **Server-side (SSR)**: Axum handlers for initial page rendering
2. **Client-side (WASM)**: Interactive UI that hydrates in the browser

**Key Pattern**: Server Functions (`#[server]` macro)

- Functions marked with `#[server]` run on the server
- Leptos generates client-side RPC stubs automatically
- Database access is server-only (via `#[cfg(feature = "ssr")]`)

**Feature Flags**:

- `ssr`: Server-side rendering (Axum, database access)
- `hydrate`: Client-side hydration (WASM)

## Commit Guidelines

- Write in present tense: "Add feature" not "Added feature"
- Focus on WHY not WHAT (the diff shows what changed)
- Keep first line under 50 characters
- Use detailed body for complex changes

## Key Dependencies

- **leptos** (0.8.x): Full-stack reactive framework
- **leptos_router**: Client-side routing
- **leptos_axum**: Server integration
- **axum** (0.8.x): HTTP server
- **rootcause**: Error handling with structured reports
- **tokio**: Async runtime

## Important Constraints

- **Feature flags**: Code split by `ssr` (server) and `hydrate` (client)
- **OIDC authentication**: Supported from the start; granular permissions beyond "logged in or not" come later
- **Self-hosted**: Designed for personal infrastructure
- **FSL License**: Source-available with non-compete restriction

## Product Design Reference

See `docs/PRD.md` for:

- Use cases and workflows
- Platform capabilities
- Integration framework design
- AI primitives specification
- Non-functional requirements
