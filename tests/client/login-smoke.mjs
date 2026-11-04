import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { randomUUID } from 'node:crypto';
import { readFile, stat } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

// Exercise the actual compiled client, including Svelte's resolved runtime.
// Only API responses are stubbed; never replace components or lifecycle hooks.
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const build = resolve(root, process.env.LOGIN_SMOKE_BUILD_DIR || 'client/build');
const timeout = 8_000;
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
  '.json': 'application/json', '.webmanifest': 'application/manifest+json',
  '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const credentials = { email: 'login-smoke@example.invalid', password: randomUUID() };
let browser;
let origin;
let failed = 0;
let activeScenario;
const server = createServer(async (req, res) => {
  try {
    // Chromium can proxy local URLs in absolute form even with a loopback bypass.
    // Serve only this test origin; nothing is ever forwarded to another origin.
    const url = new URL(req.url, origin);
    if (url.origin !== origin) {
      activeScenario?.errors.push(`Blocked proxy request: ${req.method} ${req.url}`);
      res.writeHead(403).end();
      return;
    }
    const pathname = decodeURIComponent(url.pathname);
    if (pathname.startsWith('/api/')) {
      const chunks = [];
      for await (const chunk of req) chunks.push(chunk);
      const body = Buffer.concat(chunks).toString();
      const request = { path: pathname, method: req.method,
        body: body ? JSON.parse(body) : null, authorization: req.headers.authorization };
      const state = activeScenario;
      state?.requests.push(request);
      const attempt = state?.requests.filter((item) => item.path === pathname).length;
      const fixture = state?.reply?.(attempt, request);
      if (!fixture) state?.errors.push(`Unexpected API request: ${req.method} ${pathname}`);
      const status = fixture?.status ?? 503;
      if (fixture) state.expectedStatuses.add(`${origin}${pathname}|${status}`);
      res.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' });
      res.end(JSON.stringify(fixture?.body ?? { code: 'UNEXPECTED_TEST_REQUEST' }));
      return;
    }
    let file = resolve(build, '.' + pathname);
    if (file !== build && !file.startsWith(build + sep)) {
      res.writeHead(403).end();
      return;
    }
    try {
      if ((await stat(file)).isDirectory()) file = resolve(file, 'index.html');
      await stat(file);
    } catch {
      // Static adapter fallback applies to document routes, never missing assets.
      if (!req.headers.accept?.includes('text/html')) {
        res.writeHead(404).end();
        return;
      }
      file = resolve(build, '200.html');
    }
    res.writeHead(200, { 'Content-Type': mime[extname(file)] || 'application/octet-stream',
      'Cache-Control': 'no-store' });
    res.end(await readFile(file));
  } catch {
    res.writeHead(500).end('Static test server failed');
  }
});
server.on('connect', (req, socket) => {
  activeScenario?.errors.push(`Blocked proxy tunnel: ${req.url}`);
  socket.destroy();
});
server.on('upgrade', (req, socket) => {
  activeScenario?.errors.push(`Unexpected websocket: ${req.url}`);
  socket.destroy();
});

async function eventually(check, description) {
  const deadline = Date.now() + timeout;
  let last;
  do {
    try { await check(); return; } catch (error) { last = error; }
    await new Promise((done) => setTimeout(done, 50));
  } while (Date.now() < deadline);
  throw new Error(`${description}: ${last?.message}`);
}

async function loginForm(page) {
  await page.getByRole('heading', { name: 'Welcome back!', exact: true }).waitFor();
  for (const [label, type] of [['Email', 'email'], ['Password', 'password']]) {
    const field = page.getByLabel(new RegExp(`^${label}`));
    await field.waitFor({ state: 'visible' });
    assert.equal(await field.getAttribute('type'), type);
    assert.notEqual(await field.getAttribute('required'), null, `${label} must be required`);
  }
  assert.equal(await page.getByRole('button', { name: /QR code/i }).count(), 0,
    'QR login must be hidden in default builds until the deployed API supports it');
  await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
  const worker = await page.evaluate(async () => {
    const registration = await navigator.serviceWorker.ready;
    return { scope: registration.scope, script: registration.active?.scriptURL };
  });
  assert.deepEqual(worker, { scope: `${origin}/`, script: `${origin}/sw.js` },
    'The real production worker must activate and control the login page');
}

async function enterCredentials(page) {
  await page.getByLabel(/^Email/).fill(credentials.email);
  await page.getByLabel(/^Password/).fill(credentials.password);
}

async function scenario(name, run, reply) {
  const context = await browser.newContext({ serviceWorkers: 'allow' });
  const page = await context.newPage();
  page.setDefaultTimeout(timeout);
  page.setDefaultNavigationTimeout(timeout);
  const errors = [];
  const requests = [];
  const diagnostics = [];
  const expectedStatuses = new Set();
  activeScenario = { requests, errors, expectedStatuses, reply };
  page.on('framenavigated', (frame) => {
    if (frame === page.mainFrame()) diagnostics.push(`navigation: ${frame.url()}`);
  });
  context.on('serviceworker', (worker) => diagnostics.push(`worker created: ${worker.url()}`));
  context.on('requestfailed', (request) => diagnostics.push(
    `request failed: ${request.method()} ${new URL(request.url()).pathname}: ${request.failure()?.errorText}`));
  page.on('pageerror', (error) => errors.push(`pageerror: ${error.message}`));
  page.on('console', (message) => {
    if (message.text().startsWith('[sw]')) diagnostics.push(`worker lifecycle: ${message.text()}`);
    if (message.type() !== 'error') return;
    // Chromium logs failed resources for intentional API error fixtures too.
    const intentional = [...expectedStatuses].some((entry) => {
      const [url, status] = entry.split('|');
      return message.location().url === url && message.text().startsWith('Failed to load resource:')
        && message.text().includes(`status of ${status}`);
    });
    if (!intentional) errors.push(`console: ${message.text()}`);
  });
  await context.route('**/*', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.origin !== origin) {
      errors.push(`Unexpected external request: ${url.origin}${url.pathname}`);
      await route.abort();
      return;
    }
    // Server-side fixtures also cover requests issued by the real service worker.
    await route.continue();
  });
  try {
    await run(page, requests);
    assert.deepEqual(errors, [], 'Browser must have no unexpected errors');
    assert.equal(requests.filter((request) => request.path.includes('/auth/qr')).length, 0,
      'Normal login must not request or poll a QR session');
    console.log(`PASS ${name}`);
  } catch (error) {
    failed++;
    console.error(`FAIL ${name}: ${error.message}\n${errors.join('\n')}`);
    console.error(`Startup diagnostics:\n${diagnostics.join('\n')}\nAPI paths: ${requests.map((request) => `${request.method} ${request.path}`).join(', ')}`);
  } finally {
    await context.close();
    activeScenario = undefined;
  }
}

try {
  await stat(resolve(build, '200.html'));
  await new Promise((done, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', done);
  });
  origin = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({ headless: true, timeout,
    proxy: { server: origin, bypass: '127.0.0.1' } });

  await scenario('direct /login renders required fields without phantom QR polling', async (page) => {
    const response = await page.goto(`${origin}/login`);
    assert.equal(response.status(), 200, 'SPA document fallback must return HTTP 200');
    await loginForm(page);
    // Observe longer than the application's 2-second QR polling interval.
    await page.waitForTimeout(2_200);
  });

  await scenario('landing Sign in navigation renders login', async (page) => {
    await page.goto(origin);
    await page.getByRole('link', { name: 'Sign in', exact: true }).first().click();
    await page.waitForURL(`${origin}/login`);
    await loginForm(page);
  });

  await scenario('rejected credentials show a stable friendly error', async (page, requests) => {
    await page.goto(`${origin}/login`);
    await loginForm(page);
    await enterCredentials(page);
    await page.getByRole('button', { name: 'Log In', exact: true }).click();
    await eventually(() => assert.equal(requests.length, 1), 'Login request must reach API');
    assert.deepEqual(requests[0].body, credentials);
    const alert = page.getByRole('alert');
    await alert.waitFor({ state: 'visible' });
    assert.match(await alert.innerText(), /invalid|incorrect|unable|failed|try again/i,
      'Rejected credentials need a useful error, not an expired-session message');
    await page.waitForTimeout(300);
    assert.equal(await alert.isVisible(), true, 'Error must survive instead of vanishing on reload');
    assert.equal(await page.getByLabel(/^Email/).inputValue(), credentials.email);
    assert.equal(new URL(page.url()).pathname, '/login');
    assert.equal(await page.getByRole('button', { name: 'Log In', exact: true }).isEnabled(), true);
  }, (_attempt, request) => request.path === '/api/v1/auth/login' && request.method === 'POST'
    ? { status: 401, body: { code: 'UNAUTHORIZED', message: 'unauthorized' } } : undefined);

  await scenario('TWO_FACTOR_REQUIRED reveals TOTP and submits six-digit code', async (page, requests) => {
    await page.goto(`${origin}/login`);
    await loginForm(page);
    await enterCredentials(page);
    await page.getByRole('button', { name: 'Log In', exact: true }).click();
    const code = page.getByLabel('Authenticator Code', { exact: true });
    await code.waitFor({ state: 'visible' });
    assert.notEqual(await code.getAttribute('required'), null);
    const verify = page.getByRole('button', { name: 'Verify Code', exact: true });
    assert.equal(await verify.isDisabled(), true);
    await code.fill('12345');
    assert.equal(await verify.isDisabled(), true);
    await code.fill('123456');
    await verify.click();
    await eventually(() => assert.equal(requests.length, 2), 'TOTP request must reach API');
    assert.deepEqual(requests[0].body, credentials);
    assert.deepEqual(requests[1].body, { ...credentials, totp_code: '123456' });
    await page.getByRole('alert').waitFor({ state: 'visible' });
    assert.match(await page.getByRole('alert').innerText(), /invalid|incorrect|unable|failed|try again/i);
    await page.waitForTimeout(300);
    assert.equal(await code.isVisible(), true, 'Rejected TOTP must preserve the challenge for retry');
    await code.fill('654321');
    await verify.click();
    await eventually(() => assert.equal(requests.length, 3), 'TOTP retry must reach API');
    assert.deepEqual(requests[2].body, { ...credentials, totp_code: '654321' });
    await page.getByRole('alert').waitFor({ state: 'visible' });
    assert.equal(await code.isVisible(), true, 'Challenge must also survive a second rejection');
  }, (attempt, request) => request.path !== '/api/v1/auth/login' || request.method !== 'POST'
    ? undefined : attempt === 1
    ? { status: 403, body: { code: 'TWO_FACTOR_REQUIRED', message: 'two-factor authentication code is required' } }
    : { status: 401, body: { code: 'UNAUTHORIZED', message: 'unauthorized' } });

  await scenario('unauthenticated protected /servers redirects to login', async (page) => {
    await page.goto(`${origin}/servers`);
    await page.waitForURL(`${origin}/login`);
    await loginForm(page);
  });
  await scenario('expired token on protected requests is cleared and redirects to login', async (page, requests) => {
    await page.goto(`${origin}/login`);
    await loginForm(page);
    // Synthetic token only; seed once so a reload cannot silently re-create it.
    await page.evaluate(() => localStorage.setItem('opencorde_token', 'expired-smoke-token'));
    await page.goto(`${origin}/servers`);
    await page.waitForURL(`${origin}/login`);
    await loginForm(page);
    assert.ok(requests.some((request) => request.path === '/api/v1/users/@me'
      && request.authorization === 'Bearer expired-smoke-token'), 'Must exercise authenticated profile restoration');
    assert.equal(await page.evaluate(() => localStorage.getItem('opencorde_token')), null,
      'Protected API 401 must clear the expired token');
  }, (_attempt, request) => request.method === 'GET'
    && ['/api/v1/users/@me', '/api/v1/servers'].includes(request.path)
    ? { status: 401, body: { code: 'UNAUTHORIZED', message: 'unauthorized' } } : undefined);
  console.log(`Login browser smoke: ${6 - failed}/6 passed (${build})`);
  process.exitCode = failed ? 1 : 0;
} catch (error) {
  console.error(`Login browser smoke setup failed: ${error.message}`);
  process.exitCode = 1;
} finally {
  try { await browser?.close(); }
  finally {
    server.closeAllConnections();
    await new Promise((done) => server.close(done));
  }
}
