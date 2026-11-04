# Login browser regression

`node tests/client/dm-store.mjs` runs the real DM TypeScript store and Svelte
stores through the installed client Vite runtime, without loading the app's Vite
configuration, building the app, opening a port, or making external requests.
Only the API and gateway boundaries are mocked; localStorage uses synthetic
session values. Coverage includes API/gateway delivery in both orders, inactive
DM unread deduplication, shared listener ownership and teardown, leaving a DM,
late history/send responses after navigation, live/history pagination overlap,
failed-fetch loading cleanup, and logout/session-change isolation. It needs the
existing client dependencies and no additional test packages. When Jev owns the
check loop, let Jev execute this command.

`node tests/client/login-smoke.mjs` exercises an already-built `client/build`
with the root package's Playwright Chromium. Install the root dependencies and
Playwright Chromium first. Build with QR login disabled (the default).
When Jev owns the check loop, let Jev run this command after its build check.

The test starts a loopback-only static server with the adapter's `200.html`
SPA fallback, then launches a disposable headless browser. Each scenario has a
fresh context and empty caches. The real `/sw.js` must activate with `/` scope and
control the login page. API fixtures live in the loopback server, covering both
page requests and worker-owned GET requests; unrecognized API paths/methods fail
closed with 503. A deny-only local proxy blocks external traffic, including worker
traffic that page routing cannot intercept; page routing also rejects external
requests. No existing browser profile, credentials, or live server is used. External
requests and unexpected API requests fail the test. All browser/server resources
are closed at completion. Locator/navigation waits are bounded to eight seconds.

Coverage: direct login rendering, landing-page Sign in navigation, required
email/password inputs, persistent friendly errors after rejected credentials,
TOTP challenge and six-digit request/retry, unauthenticated `/servers` redirect,
expired synthetic bearer-token clearing and redirect after protected API 401,
default-hidden QR login and absence of unsolicited QR requests. Browser exceptions
and console errors fail; only Chromium's resource-error messages for the precise
mocked API URL/status are exempted. The normal page is observed for longer than
the two-second QR poll interval.

Fixtures match `crates/opencorde-api/src/error.rs`: invalid credentials and invalid
TOTP return 401 `UNAUTHORIZED`, while a missing TOTP returns 403
`TWO_FACTOR_REQUIRED`. Only HTTP API responses are mocked; compiled app code and
Svelte lifecycle/runtime execute unchanged.

Set `LOGIN_SMOKE_BUILD_DIR` to an archived build directory to reproduce regressions
against old assets (relative paths resolve from the repository root). A correct
regression demonstration runs the same harness against broken and fixed builds.

This focused suite does not prove successful session establishment, Steam OAuth,
QR handoff when enabled, service-worker upgrades, live-backend compatibility,
desktop/Tauri behavior, or all protected routes. It does not create accounts or
make live mutations.
