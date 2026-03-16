# /tests/

Purpose: Integration and end-to-end tests.

Pattern: One subdirectory per test category. Separate tools and concerns.

| Directory | Purpose | Tool |
|-----------|---------|------|
| integration/ | API and database layer tests | testcontainers-rs, Tokio |
| e2e/ | Full app flows (auth, messaging, voice) | Playwright (browser automation) |
| fixtures/ | Test data, mock servers, database seeds | — |
