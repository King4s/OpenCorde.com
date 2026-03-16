# /crates/opencorde-api/

Purpose: REST API + WebSocket gateway for real-time events. Binary crate.

Entry: src/main.rs
Depends on: opencorde-core, opencorde-db

Pattern: Organized by concern — routes for HTTP handlers, middleware for layer composition, config for environment variables.

## Core Files

| File | Purpose | Status |
|------|---------|--------|
| src/main.rs | Server startup, initialization, tracing setup | ✓ Implemented |
| src/lib.rs | Library exports (AppState, error, config, routes, middleware) | ✓ Implemented |
| src/config.rs | Environment variable loading with secrets masking | ✓ Implemented |
| src/error.rs | Unified ApiError type + IntoResponse for HTTP error handling | ✓ Implemented |

## Modules

| Module | Purpose | Files | Status |
|--------|---------|-------|--------|
| src/routes/ | HTTP route handlers (REST endpoints) | mod.rs, health.rs | ✓ Implemented (health only) |
| src/middleware/ | Axum middleware layers (CORS, request ID) | mod.rs, cors.rs, request_id.rs | ✓ Implemented |
| src/ws/ | WebSocket gateway logic (Phase 1: Weeks 7-8) | INDEX.md only | — Future |

## Phase 1 (Weeks 1-4) — Core API Infrastructure
- [x] Config module with environment variable loading
- [x] Error type with IntoResponse implementation
- [x] Middleware (CORS, request ID)
- [x] Health check endpoint
- [x] Main.rs with server startup and tracing

## Phase 2 (Weeks 5-6) — Authentication & Authorization
- [ ] Auth routes (register, login, logout, refresh)
- [ ] JWT middleware
- [ ] Rate limiting middleware

## Phase 3 (Weeks 7-8) — WebSocket & Real-time Events
- [ ] WebSocket gateway
- [ ] Event dispatch system
- [ ] Voice session management
