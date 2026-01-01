-- Create sessions table for storing authenticated user sessions
-- Sessions are created after successful OIDC authentication

CREATE TABLE sessions (
    -- Session ID (random string used in cookie)
    id TEXT PRIMARY KEY,

    -- Reference to the authenticated user
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Roles derived from OIDC groups at authentication time (JSON array)
    roles JSONB NOT NULL DEFAULT '[]',

    -- When the session was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When the session expires
    expires_at TIMESTAMPTZ NOT NULL,

    -- OIDC access token (for API calls that need it)
    access_token TEXT,

    -- OIDC refresh token (for token refresh)
    refresh_token TEXT
);

-- Index for looking up sessions by user
CREATE INDEX sessions_user_id_idx ON sessions (user_id);

-- Index for finding expired sessions (for cleanup)
CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);

-- Clean up expired sessions (can be called periodically)
-- Note: This is a function, actual cleanup should be scheduled separately
