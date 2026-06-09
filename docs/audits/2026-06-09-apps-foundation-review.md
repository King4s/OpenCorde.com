# OpenCorde M7.1 Foundation Audit - Applications/Bots/OAuth
**Date**: 2026-06-09  
**Reviewer**: Hermes Agent  
**Scope**: Read-only audit of models/application.rs, bot_user.rs, oauth_scope.rs, app_install.rs, repos/app_repo.rs, bot_user_repo.rs, and migration 064_apps_and_bots.sql.

---

## 1. Application Name Uniqueness
**Finding**: The `applications.name` column has a **global unique index** (`idx_applications_name` on `name` only).  
**Implication**: Application names must be unique across all owners (global scope). There is no owner-scoped uniqueness (i.e., two different users cannot register applications with the same name).  
**Assessment**: This design choice may be intentional to avoid name collisions in a global namespace, but it limits users from using desirable names already taken by others. Consider whether owner-scoped uniqueness (`UNIQUE (owner_user_id, name)`) would be more appropriate, aligning with typical SaaS patterns where names are unique per user/account.

---

## 2. Bot User Username Uniqueness per Application
**Finding**: The `bot_users` table **does not have a unique constraint** on `username` or `(application_id, username)`.  
**Note**: The `users` table (not shown in migration 064) likely has a global unique constraint on `username` (since `create_bot_user` inserts into `users` and would fail on duplicate username). This indirectly enforces global username uniqueness across all bot (and regular) users, but not per-application uniqueness.  
**Assessment**: If the requirement is that bot usernames must be unique **within an application** (i.e., same application cannot have two bots with same username, but different applications can), then the current schema does not enforce this. A unique index on `bot_users (application_id, username)` would be needed.

---

## 3. redirect_uris JSONB Sufficiency
**Finding**: The `applications.redirect_uris` column is defined as `JSONB NOT NULL DEFAULT '[]'` and is used to store an array of strings.  
**Implementation**: The repository layer serializes/deserializes between `Vec<String>` and `JSONB` via `serde_json::to_value`/`sqlx::FromRow`.  
**Assessment**: JSONB is sufficient for storing and querying arrays of strings. It supports containment (`@>`), existence (`?`), and indexing. No issues observed. Alternative: PostgreSQL `text[]` array type could offer more native array functions, but JSONB is a valid choice.

---

## 4. OAuth Scopes Seeding
**Finding**: Migration 064 seeds the `oauth_scopes` table with six standard Discord-compatible scopes (ids 1–6) using an `INSERT ... ON CONFLICT (id) DO UPDATE` pattern.  
**Scope names**: `identify`, `guilds`, `bot`, `messages.read`, `applications.commands`, `webhook.incoming`.  
**Assessment**: Seeding appears correct. The `oauth_scopes` table has a `UNIQUE` constraint on `name`, and the seeded values are unique. The `ON CONFLICT` clause ensures idempotent migrations (updates if row exists). The Rust model `OAuthScope` and constants in `scope_names` match the seeded values.

---

## Summary of Observations
- **Application name uniqueness** is global; consider if owner-scoped is desired.
- **Bot user username uniqueness** is not enforced per application; relies on global `users.username` uniqueness.
- **redirect_uris as JSONB** is appropriate.
- **OAuth scope seeding** is correct and idempotent.

No modifications were made; this is a read-only audit.