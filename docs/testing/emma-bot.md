# Emma Bot Credentials and Test Protocol

Emma Bot is the named live-user QA actor for Discord-parity dogfooding. Emma must exercise OpenCorde through the same public UI and API surfaces as a normal user or future bot/application identity. Do not shortcut by writing directly to the database.

## Credential storage

Never put Emma credentials, JWTs, refresh tokens, webhook tokens, or bot tokens in the repository, reports, screenshots, GitHub issue comments, or kanban handoffs.

Store operator-local credentials outside the repo in a chmod-600 env file:

- Preferred path: `~/.hermes/opencorde/emma-bot.env`
- File mode: `0600`
- Owner: the local operator account that runs QA
- Source it only for the shell/process that runs Emma QA

Expected variables:

```bash
OC_BASE=https://opencorde.com
OC_EMMA_EMAIL=***
OC_EMMA_PASSWORD=***
OC_EMMA_SERVER_ID=***
OC_EMMA_QA_CHANNEL_ID=***
# Future app/bot identity, once implemented:
# OC_EMMA_APP_ID=***
# OC_EMMA_BOT_TOKEN=***
```

Safe setup pattern for operators:

```bash
mkdir -p ~/.hermes/opencorde
umask 077
$EDITOR ~/.hermes/opencorde/emma-bot.env
chmod 600 ~/.hermes/opencorde/emma-bot.env
```

Scripts should load this path only when explicitly requested, for example with `OC_EMMA_ENV=~/.hermes/opencorde/emma-bot.env`. If a script writes a report, the report may include `emma_account: "configured"` and `credential_source: "~/.hermes/opencorde/emma-bot.env"`, but it must not include raw values from any `OC_EMMA_*` secret variables.

## Secret redaction rules

Reports and logs may include:

- `OC_BASE`
- Account role label such as `emma-bot`
- Feature name and scenario name
- HTTP status codes
- OpenCorde message IDs, channel IDs, and server IDs only when they are already test-fixture identifiers
- Screenshot/report file paths

Reports and logs must not include:

- Passwords
- JWT access or refresh tokens
- Webhook tokens
- Future bot/application tokens
- Full `Authorization` headers
- Cookies or browser localStorage dumps

If a failed request object contains headers or body fields, redact before persisting it.

## Run protocol

Each Emma run should be small, reproducible, and tied to one parity feature or GitHub issue.

1. Load credentials from `~/.hermes/opencorde/emma-bot.env` into the current process only.
2. Authenticate as Emma through the public login/API path.
3. Navigate to the configured test server and QA channel.
4. Execute the feature scenario as a live user or future bot identity.
5. Assert the expected UI/API/realtime result.
6. Capture evidence under `reports/raw/` and screenshots under `reports/parity-screenshots/` when applicable.
7. Post one in-app QA summary message as Emma, if the scenario itself allows it.
8. Write a sanitized report containing pass/fail, scenario name, evidence paths, and redacted failure details.

Emma's in-app QA summary format:

```text
Emma QA: <feature/scenario>
Result: PASS|FAIL
Observed: <short UI/API/realtime observation>
Evidence: <report path or screenshot path>
Follow-up: <issue/card id or "none">
```

## Issue #8 protocol: apps, commands, bots, and webhooks

Until first-class OpenCorde app/bot identity exists, Emma uses a normal test user account to prove current user-facing behavior. Once bot/application identity exists, repeat the same scenarios through the real bot token path.

Minimum Issue #8 scenarios:

- Slash command execution: invoke a test command from the UI, assert the command request succeeds, and verify public response rendering.
- Ephemeral response behavior: invoke a command that should be visible only to Emma, then verify another logged-in test user cannot see it.
- Deferred response behavior: invoke a long-running test command, assert an interim pending/deferred state and final response.
- Webhook lifecycle: create a webhook, execute it into the QA channel, verify realtime message delivery, then delete or rotate the webhook token.
- Permission denial: attempt a command or webhook action without the required permission and assert denial without side effects.

For every scenario, the report should record the exact scenario name, the expected result, the observed result, and the evidence path. Do not mark Issue #8 acceptance complete until an Emma run succeeds against the live site and a sanitized report is committed.

## Failure handling

- If credentials are missing, fail closed with a message naming only the missing variable names.
- If login fails, do not print the supplied password or token response.
- If a report accidentally captures a secret, delete the report from the working tree, rotate the exposed credential, and re-run with redaction fixed before committing anything.
- If a live-site run fails because implementation is missing, file or update the relevant kanban/GitHub task with sanitized symptoms only.
