import { chromium, firefox, webkit } from 'playwright';
import fs from 'node:fs/promises';

const base = 'https://opencorde.com';
const routes = ['/', '/login', '/register', '/servers', '/invite/invalid-code-123'];
const viewports = [
  { name: 'mobile', width: 390, height: 844 },
  { name: 'tablet', width: 768, height: 1024 },
  { name: 'desktop', width: 1440, height: 900 }
];
const browsers = [
  ['chromium', chromium],
  ['firefox', firefox],
  ['webkit', webkit]
];

const results = [];
for (const [browserName, browserType] of browsers) {
  let browser;
  try {
    browser = await browserType.launch({ headless: true });
  } catch (e) {
    results.push({ browser: browserName, launch_error: String(e) });
    continue;
  }
  for (const viewport of viewports) {
    const context = await browser.newContext({ viewport });
    const page = await context.newPage();
    const consoleMessages = [];
    const pageErrors = [];
    const failedRequests = [];
    page.on('console', msg => {
      if (['error', 'warning'].includes(msg.type())) {
        consoleMessages.push({ type: msg.type(), text: msg.text() });
      }
    });
    page.on('pageerror', err => pageErrors.push(String(err)));
    page.on('requestfailed', req => failedRequests.push({ url: req.url(), error: req.failure()?.errorText || 'unknown' }));

    for (const route of routes) {
      const entry = { browser: browserName, viewport: viewport.name, route };
      try {
        const resp = await page.goto(base + route, { waitUntil: 'networkidle', timeout: 30000 });
        entry.status = resp?.status() ?? null;
        entry.finalUrl = page.url();
        entry.title = await page.title();
        entry.h1 = await page.locator('h1').first().textContent().catch(() => null);
        entry.console = consoleMessages.splice(0);
        entry.pageErrors = pageErrors.splice(0);
        entry.failedRequests = failedRequests.splice(0);
        entry.anchorCount = await page.locator('a').count();
        entry.buttonCount = await page.locator('button').count();
        entry.horizontalOverflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
        entry.textSnippet = (await page.locator('body').innerText().catch(() => '')).slice(0, 500);
      } catch (e) {
        entry.error = String(e);
        entry.console = consoleMessages.splice(0);
        entry.pageErrors = pageErrors.splice(0);
        entry.failedRequests = failedRequests.splice(0);
      }
      results.push(entry);
    }
    await context.close();
  }
  await browser.close();
}

await fs.mkdir('/home/mb/opencorde/reports/raw', { recursive: true });
await fs.writeFile('/home/mb/opencorde/reports/raw/public-qa.json', JSON.stringify(results, null, 2));
console.log(JSON.stringify(results, null, 2));