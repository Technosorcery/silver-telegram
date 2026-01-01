-- Create users table for storing authenticated users
-- Users are created on first OIDC login and identified by their OIDC subject claim

CREATE TABLE users (
    -- Internal platform user ID (ULID stored as text with prefix)
    id TEXT PRIMARY KEY,

    -- OIDC subject claim - unique identifier from the identity provider
    subject TEXT NOT NULL,

    -- OIDC issuer URL - identifies which identity provider authenticated the user
    issuer TEXT NOT NULL,

    -- User's email address (from OIDC email claim)
    email TEXT,

    -- User's display name (from OIDC name or preferred_username claim)
    display_name TEXT,

    -- User's configured timezone (IANA timezone name, e.g., "America/New_York")
    timezone TEXT,

    -- When the user record was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the user record was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Unique constraint on subject + issuer (user is unique per OIDC provider)
    CONSTRAINT users_subject_issuer_unique UNIQUE (subject, issuer)
);

-- Index for looking up users by subject and issuer
CREATE INDEX users_subject_issuer_idx ON users (subject, issuer);
