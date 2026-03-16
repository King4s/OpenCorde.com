# /crates/opencorde-api/src/routes/auth/

Purpose: Authentication endpoints and related logic (registration, login, token refresh).

Pattern: Modular design with types, handlers, and validation separated by concern.

## Files

| File | Purpose | Lines | Status |
|------|---------|-------|--------|
| mod.rs | Module composition and router export | 56 | ✓ Implemented |
| types.rs | Request/response types | 98 | ✓ Implemented |
| handlers.rs | Login and refresh HTTP handlers | 214 | ✓ Implemented |
| register.rs | Registration HTTP handler | 131 | ✓ Implemented |
| validation.rs | Input validation and cookie helpers | 176 | ✓ Implemented |

## Endpoints

| Route | Method | Handler | Status |
|-------|--------|---------|--------|
| /api/v1/auth/register | POST | register::register | ✓ Implemented |
| /api/v1/auth/login | POST | handlers::login | ✓ Implemented |
| /api/v1/auth/refresh | POST | handlers::refresh | ✓ Implemented |

## Features

- Input validation (username, email, password format)
- Conflict detection (duplicate email/username)
- Argon2id password hashing
- JWT access + refresh token generation
- HttpOnly refresh token cookies
- Comprehensive structured logging
- Extensive unit test coverage

## Dependencies

- axum (web framework)
- opencorde_db::repos::user_repo (CRUD operations)
- opencorde_core::password (password hashing)
- opencorde_core::Snowflake (ID generation)
- crate::jwt (token creation/validation)
- crate::AppState (database + config)
- crate::error::ApiError (unified error handling)

## Design Notes

- Each handler is individually instrumented with tracing::instrument
- Validation logic is centralized in validation.rs
- Register and login/refresh handlers are separated to keep files under 300 lines
- Types are shared and re-exported from mod.rs
- All handlers return Result<Response, ApiError> for unified error handling
