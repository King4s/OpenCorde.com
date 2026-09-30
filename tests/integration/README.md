# OpenCorde API Integration Tests

## Running

From the repo root:

```bash
# using the venv
/home/mb/.hermes/kanban/boards/opencorde/workspaces/t_1b8248ab/.venv/bin/pytest tests/integration/ -v \
  --json-report --json-report-file=reports/raw/integration-report.json
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OC_BASE` | `https://opencorde.com` | API base URL |
| `OC_ADMIN_EMAIL` | `permission-admin@opencorde.local` | Admin user email |
| `OC_ADMIN_PASSWORD` | `admin123456` | Admin user password |
| `OC_MEMBER_EMAIL` | `browsertest@opencorde.local` | Regular member email |
| `OC_MEMBER_PASSWORD` | `BrowserTest@99` | Regular member password |
| `OC_JWT_SECRET` | — | JWT secret for token generation (optional) |
| `OC_ADMIN_USER_IDS` | — | Comma-separated admin IDs for token generation |

When `OC_JWT_SECRET` and `OC_ADMIN_USER_IDS` are set, the admin tests can generate a short-lived token instead of logging in.
