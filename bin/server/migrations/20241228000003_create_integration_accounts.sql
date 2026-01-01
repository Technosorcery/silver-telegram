-- Create integration_accounts table for storing user-configured integrations
-- Integrations connect external services (email, calendar, etc.) to the platform

CREATE TABLE integration_accounts (
    -- Internal integration account ID (ULID stored as text with prefix)
    id TEXT PRIMARY KEY,

    -- User-provided name/label for this integration (e.g., "Work Gmail", "Personal Calendar")
    -- Note: Ownership is tracked in SpiceDB, not via user_id column (see ADR-002)
    name TEXT NOT NULL,

    -- Type of integration (e.g., "imap", "gmail", "calendar_feed")
    integration_type TEXT NOT NULL,

    -- Status of the integration
    -- 'connected': Successfully connected and working
    -- 'error': Connection failed or credentials invalid
    -- 'pending': Awaiting OAuth completion or initial connection
    status TEXT NOT NULL DEFAULT 'pending',

    -- Error message if status is 'error'
    error_message TEXT,

    -- When the integration was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the integration was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the integration was last successfully used
    last_used_at TIMESTAMPTZ
);

-- Index for filtering by type
CREATE INDEX integration_accounts_type_idx ON integration_accounts (integration_type);

-- Create credentials table for encrypted credential storage
-- Credentials are stored separately for security (encrypted at rest)
CREATE TABLE credentials (
    -- Credential ID (ULID)
    id TEXT PRIMARY KEY,

    -- Reference to the integration account
    integration_account_id TEXT NOT NULL REFERENCES integration_accounts(id) ON DELETE CASCADE,

    -- Type of credential (e.g., "basic", "oauth2", "api_key")
    credential_type TEXT NOT NULL,

    -- Encrypted credential data (JSON encrypted with platform key)
    -- For basic auth: {"username": "...", "password": "..."}
    -- For OAuth2: {"access_token": "...", "refresh_token": "...", "expires_at": "..."}
    -- For API key: {"api_key": "..."}
    encrypted_data BYTEA NOT NULL,

    -- When the credential was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the credential was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for looking up credentials by integration
CREATE INDEX credentials_integration_id_idx ON credentials (integration_account_id);

-- Create integration_config table for non-secret configuration
-- This stores settings like server addresses, folder names, feed URLs
CREATE TABLE integration_config (
    -- Config ID (ULID)
    id TEXT PRIMARY KEY,

    -- Reference to the integration account
    integration_account_id TEXT NOT NULL REFERENCES integration_accounts(id) ON DELETE CASCADE,

    -- Configuration data (JSON)
    -- For IMAP: {"server": "imap.example.com", "port": 993, "use_tls": true}
    -- For calendar feed: {"url": "https://..."}
    config_data JSONB NOT NULL DEFAULT '{}',

    -- When the config was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the config was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure one config per integration
    CONSTRAINT integration_config_unique UNIQUE (integration_account_id)
);

-- Index for looking up config by integration
CREATE INDEX integration_config_integration_id_idx ON integration_config (integration_account_id);
