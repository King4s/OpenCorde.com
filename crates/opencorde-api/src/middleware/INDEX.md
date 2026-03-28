# /crates/opencorde-api/src/middleware/

Purpose: Axum middleware layers for request processing.

Pattern: One file per middleware concern. Each middleware is wrapped via Axum's layer composition in main.rs.

## Current (Phase 1)

| File | Purpose | Status |
|------|---------|--------|
| request_id.rs | UUID generation and tracing context propagation | ✓ Implemented |
| cors.rs | CORS configuration (environment-aware) | ✓ Implemented |
| auth.rs | JWT validation and AuthUser extractor | ✓ Implemented |
| rate_limit.rs | Per-IP token-bucket rate limiting (governor, admin-configurable) | ✓ Implemented |
| mod.rs | Middleware module exports | ✓ Implemented |

## Implementation Notes
- Middleware is applied in main.rs in order: CORS → request_id → trace
- Request ID is added to tracing span for distributed tracing
- CORS is environment-aware (strict in production, permissive in development)
- All middleware uses tower and tower-http for consistency with Axum
