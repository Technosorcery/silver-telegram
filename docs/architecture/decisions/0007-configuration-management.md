# ADR-007: Centralized Configuration Management

**Status**: Accepted

## Context

Configuration values (database URLs, OIDC settings, timeouts, cookie names, etc.) must be managed consistently. Scattered `std::env::var()` calls and duplicated magic constants lead to:
- Inconsistent defaults across the codebase
- Difficulty understanding what configuration a service requires
- Hard-to-find configuration bugs
- No single place to see all configuration options

## Decision

Centralize configuration loading in service binaries using the `config` crate.

### Pattern

1. **Library crates** define config structs for their needs:
   ```rust
   // lib/platform-access/src/config.rs
   #[derive(Debug, Deserialize)]
   pub struct OidcConfig {
       pub issuer_url: String,
       pub client_id: String,
       // ...
   }
   ```

2. **Service binaries** compose library configs into a service config:
   ```rust
   // bin/server/src/config.rs
   #[derive(Debug, Deserialize)]
   pub struct ServerConfig {
       pub bind_address: SocketAddr,
       pub database_url: String,
       pub oidc: OidcConfig,
       pub session: SessionConfig,
       // ...
   }
   ```

3. **Service binaries** use the `config` crate to load from environment, files, etc.:
   ```rust
   let config = Config::builder()
       .add_source(Environment::default())
       .build()?
       .try_deserialize::<ServerConfig>()?;
   ```

### Rules

- No `std::env::var()` calls outside service config loading
- No scattered `const` values for configurable settings - defaults belong in config structs via `#[serde(default)]`
- Library crates receive config, never read environment directly
- Each service documents its configuration in a single place

## Rationale

- Single source of truth for each service's configuration
- Library crates remain environment-agnostic and testable
- The `config` crate provides layered configuration (files, env, defaults)
- Strongly typed configuration catches errors at startup

## Consequences

- All services depend on the `config` crate
- Configuration structs must be kept in sync with documentation
- Refactoring configuration requires updating the config struct, not hunting for env var reads
