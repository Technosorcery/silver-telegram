# Platform Access

Access to the platform, authentication, and user/admin functionality.

## Background

### Context

The platform is multi-user from the start, supporting individuals and families. Access is controlled via an external OIDC provider - users must authenticate and have appropriate group grants to access the platform.

### Audience

- **Users**: Anyone with OIDC authentication and user-level group grant
- **Admins**: Users with additional admin-level group grant

### Problem Statements

- Users need a secure way to access their personal assistant
- Multiple users (e.g., family members) need isolated access to their own data
- Admins need additional capabilities for platform oversight

## Hypothesis

By delegating authentication and authorization to an external OIDC provider, the platform gets secure multi-user access without building auth infrastructure, and access control is managed where IT/operators already manage identity.

## Success Criteria

- User can log in via OIDC and access their data
- User without appropriate group grant cannot access the platform
- Multiple users have isolated experiences
- Admins can perform oversight functions when needed

## Requirements

- **Login flow**:
  - Unauthenticated users see a login screen with button to initiate OIDC flow
  - After successful OIDC authentication, user is redirected to main interface
  - First login creates user record; subsequent logins use existing record
  - Profile info (name, etc.) comes from OIDC claims

- **Authenticated experience**:
  - Header includes user menu on far right
  - User menu contains: user settings, logout
  - If user has admin group grant, menu also contains admin functionality link

- **User settings** (accessed via user menu):
  - Timezone configuration
  - User-specific integrations/connections (credentials for external services)

- **Admin functionality** (if authorized):
  - Trigger/cancel workflows belonging to other users

- **Multi-user**:
  - Access controlled via OIDC groups (user-level grant required)
  - Admin capabilities controlled via OIDC groups (admin-level grant required)
  - Users cannot see or access other users' data

## Non-requirements

- No invitation flow (handled at OIDC provider)
- No onboarding or tutorial on first login
- No in-platform user management (managed via OIDC provider)
- No in-platform password/credential management for login (OIDC handles this)
- No in-app health/status/metrics view (post-MVP)
- No assume role / impersonation (post-MVP)

## Tradeoffs and concerns

*Especially from engineering, what hard decisions will we have to make in order to implement this solution? What future problems might we have to solve because we chose to implement this?*
