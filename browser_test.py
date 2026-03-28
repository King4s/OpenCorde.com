"""
OpenCorde Browser Inspection - Playwright
Tests all major UI routes via headless Chrome
Storage key: opencorde_token (from auth store)
API proxy: /api → localhost:8080 (vite proxy)
"""
import asyncio
import os
import json
import aiohttp
from playwright.async_api import async_playwright

BASE = "https://opencorde.com"
API = "https://opencorde.com/api/v1"
SHOTS = "/tmp/opencorde_shots"
TEST_USER = "browsertest_user"
TEST_PASS = "BrowserTest@99"
TEST_EMAIL = "browsertest@opencorde.local"
STORAGE_KEY = "opencorde_token"  # auth.ts uses this key

os.makedirs(SHOTS, exist_ok=True)
results = []

def log(check, status, detail=""):
    if status is None:
        icon = "SKIP"
    else:
        icon = "PASS" if status else "FAIL"
    results.append({"check": check, "status": icon, "detail": detail})
    print(f"[{icon}] {check}" + (f" — {detail}" if detail else ""))

async def nav(page, url, wait_ms=1500):
    try:
        await page.goto(url, wait_until="domcontentloaded", timeout=15000)
    except Exception as e:
        if "ERR_ABORTED" in str(e) or "net::ERR" in str(e):
            pass  # redirect aborted original nav — page still loaded somewhere
        else:
            print(f"  [nav warn] {url}: {e}")
    await page.wait_for_timeout(wait_ms)
    # Dismiss vite-error-overlay if present (from HMR errors)
    overlay = page.locator("vite-error-overlay")
    if await overlay.count() > 0:
        await page.keyboard.press("Escape")
        await page.wait_for_timeout(300)

async def safe_content(page):
    for _ in range(3):
        try:
            return await page.content()
        except Exception:
            await page.wait_for_timeout(400)
    return ""

async def register_test_user():
    async with aiohttp.ClientSession() as session:
        payload = {"username": TEST_USER, "email": TEST_EMAIL, "password": TEST_PASS}
        async with session.post(f"{API}/auth/register", json=payload) as r:
            body = await r.json()
            if r.status in (200, 201):
                print(f"  [API] Registered OK, token={body.get('access_token','')[:20]}...")
                return body.get("access_token")
            print(f"  [API] Register {r.status}: {body}")

        payload = {"email": TEST_EMAIL, "password": TEST_PASS}
        async with session.post(f"{API}/auth/login", json=payload) as r:
            body = await r.json()
            if r.status == 200:
                print(f"  [API] Login OK, token={body.get('access_token','')[:20]}...")
                return body.get("access_token")
            print(f"  [API] Login {r.status}: {body}")
    return None

async def get_server_id(token):
    async with aiohttp.ClientSession() as session:
        headers = {"Authorization": f"Bearer {token}"}
        async with session.get(f"{API}/servers", headers=headers) as r:
            if r.status == 200:
                data = await r.json()
                return data[0]["id"] if data else None
    return None

async def run_tests():
    print("=== API Pre-check ===")
    token = await register_test_user()
    server_id = await get_server_id(token) if token else None
    print(f"  Token: {'OK' if token else 'NONE'}, Server: {server_id or 'NONE'}")
    print()

    async with async_playwright() as pw:
        browser = await pw.chromium.launch(
            executable_path="/usr/bin/google-chrome",
            args=["--no-sandbox", "--disable-dev-shm-usage", "--headless=new"]
        )
        console_errors = []

        # =============================================
        # PHASE 1: UNAUTHENTICATED PAGES
        # =============================================
        ctx1 = await browser.new_context(viewport={"width": 1280, "height": 900})
        page = await ctx1.new_page()
        def _on_console(msg):
            if msg.type == "error" and "Failed to load resource" not in msg.text and "net::ERR_" not in msg.text:
                console_errors.append(msg.text)
        page.on("console", _on_console)
        page.on("pageerror", lambda err: console_errors.append(f"PAGE_ERR: {err}"))

        # 1. Login page
        await nav(page, BASE + "/login")
        await page.screenshot(path=f"{SHOTS}/01_login.png")
        has_email = await page.locator("#email").count() > 0
        has_pass = await page.locator("#password").count() > 0
        has_submit = await page.locator("button[type=submit]").count() > 0
        pg_html = await safe_content(page)
        has_reg_link = 'href="/register"' in pg_html
        log("Login: email input (#email)", has_email)
        log("Login: password input (#password)", has_pass)
        log("Login: submit button", has_submit)
        log("Login: register link", has_reg_link)

        # 2. Forgot-password toggle
        await page.wait_for_timeout(200)
        forgot_btn = page.locator("button:has-text('Forgot')")
        n_forgot = await forgot_btn.count()
        if n_forgot > 0:
            try:
                await forgot_btn.first.click(timeout=5000)
            except Exception:
                # Try force-click to bypass any overlay
                await forgot_btn.first.click(force=True, timeout=5000)
            await page.wait_for_timeout(800)
            await page.screenshot(path=f"{SHOTS}/02_forgot_pw.png")
            n_email_after = await page.locator("input[type=email]").count()
            n_send_btn = await page.locator("button:has-text('Send')").count()
            log("Login: forgot-password form visible after toggle",
                n_email_after > 0 or n_send_btn > 0,
                f"email_inputs={n_email_after}, send_btns={n_send_btn}")
        else:
            log("Login: forgot-password form visible after toggle", False, "forgot button not found")

        # 3. Register page
        await nav(page, BASE + "/register")
        await page.screenshot(path=f"{SHOTS}/03_register.png")
        reg_html = await safe_content(page)
        has_reg_email = await page.locator("#email, input[type=email]").count() > 0
        has_username = await page.locator("#username").count() > 0
        has_reg_pw = await page.locator("input[type=password]").count() > 0
        log("Register: email field", has_reg_email)
        log("Register: username field (#username)", has_username)
        log("Register: password field", has_reg_pw)

        # 4. Reset-password page
        await nav(page, BASE + "/reset-password?token=testtoken123")
        await page.screenshot(path=f"{SHOTS}/04_reset_pw.png")
        has_pw_input = await page.locator("input[type=password]").count() >= 1
        log("Reset-password: password input present", has_pw_input)

        # 5. Login flow (browser-level submit)
        await nav(page, BASE + "/login")
        if token:
            try:
                await page.locator("#email").fill(TEST_EMAIL)
                await page.locator("#password").fill(TEST_PASS)
                async with page.expect_navigation(timeout=8000):
                    await page.locator("button[type=submit]").first.click()
            except Exception as e:
                print(f"  [nav] login submit nav: {e}")
                await page.wait_for_timeout(2000)
            await page.screenshot(path=f"{SHOTS}/05_after_login.png")
            after_url = page.url
            log("Login flow: redirects away from /login", "/login" not in after_url, f"url={after_url}")
        else:
            log("Login flow: redirects away from /login", False, "no test token")

        await page.close()
        await ctx1.close()

        # =============================================
        # PHASE 2: AUTHENTICATED PAGES (pre-seeded token)
        # =============================================
        if not token:
            for lbl in ["Main app nav", "@me loads", "@me UI", "Friends", "Settings bio",
                        "Settings status", "Settings admin link", "Settings save btn",
                        "Admin heading", "Admin stats", "Discover loads", "Discover UI",
                        "Server loads", "Server channels", "No render broken markers"]:
                log(lbl, None, "SKIP — no token")
        else:
            auth_ctx = await browser.new_context(
                viewport={"width": 1280, "height": 900},
                storage_state={
                    "cookies": [],
                    "origins": [
                        {
                            "origin": BASE,
                            "localStorage": [
                                {"name": STORAGE_KEY, "value": token}
                            ]
                        }
                    ]
                }
            )
            ap = await auth_ctx.new_page()
            ap.on("console", _on_console)
            ap.on("pageerror", lambda err: console_errors.append(f"PAGE_ERR: {err}"))

            # 6. Main app — navigate to @me which has the sidebar nav
            await nav(ap, BASE + "/@me")
            await ap.screenshot(path=f"{SHOTS}/06_main_app.png")
            main_url = ap.url
            has_nav = await ap.locator("nav").count() > 0
            not_stuck_login = "/login" not in main_url
            log("Main app: @me loads with nav (not stuck on login)", has_nav and not_stuck_login,
                f"url={main_url}, nav={has_nav}")

            # 7. @me DMs
            await nav(ap, BASE + "/@me")
            await ap.screenshot(path=f"{SHOTS}/07_dms.png")
            dm_html = await safe_content(ap)
            no_crash = "Cannot GET" not in dm_html and "<title>404" not in dm_html
            has_dm_ui = await ap.locator("h1, h2, h3, button, [class*=dm]").count() > 0
            log("@me DMs: loads without crash", no_crash)
            log("@me DMs: UI elements present", has_dm_ui)

            # 8. Friends (ssr=false, needs longer wait for JS render)
            await nav(ap, BASE + "/@me/friends", wait_ms=2500)
            await ap.screenshot(path=f"{SHOTS}/08_friends.png")
            friends_html = await safe_content(ap)
            has_friends = "friend" in friends_html.lower() or "Friends" in friends_html
            log("Friends: page renders with friend content", has_friends)

            # 9. Settings
            await nav(ap, BASE + "/settings")
            await ap.screenshot(path=f"{SHOTS}/09_settings.png")
            has_bio = await ap.locator("#settings-bio").count() > 0
            has_status_field = await ap.locator("#settings-status").count() > 0
            has_admin_link = await ap.locator("a[href='/admin']").count() > 0
            has_save = await ap.locator("button:has-text('Save')").count() > 0
            log("Settings: bio textarea (#settings-bio)", has_bio)
            log("Settings: status message input (#settings-status)", has_status_field)
            log("Settings: admin dashboard link", has_admin_link)
            log("Settings: save changes button", has_save)

            # 10. Admin dashboard
            await nav(ap, BASE + "/admin")
            await ap.screenshot(path=f"{SHOTS}/10_admin.png")
            admin_html = await safe_content(ap)
            has_heading = await ap.locator("h1, h2").count() > 0
            admin_lower = admin_html.lower()
            has_stats = "users" in admin_lower and "servers" in admin_lower
            log("Admin: heading present", has_heading)
            log("Admin: stats data (users + servers mentioned)", has_stats)

            # 11. Discover
            await nav(ap, BASE + "/discover")
            await ap.screenshot(path=f"{SHOTS}/11_discover.png")
            disc_html = await safe_content(ap)
            no_crash = "Cannot GET" not in disc_html
            has_ui = await ap.locator("h1, h2, input, [class*=server]").count() > 0
            log("Discover: loads without crash", no_crash)
            log("Discover: UI elements present", has_ui)

            # 12. Server (if available)
            if server_id:
                await nav(ap, BASE + f"/servers/{server_id}")
                await ap.screenshot(path=f"{SHOTS}/12_server.png")
                s_html = await safe_content(ap)
                no_crash = "Cannot GET" not in s_html
                has_ch = await ap.locator("[class*=channel], [href*=channel]").count() > 0
                log("Server: loads", no_crash)
                log("Server: channel list visible", has_ch)
            else:
                log("Server: loads", None, "SKIP — test user has no servers")
                log("Server: channel list visible", None, "SKIP")

            # 13. Broken render markers
            broken = []
            for route in ["/", "/@me", "/settings"]:
                await nav(ap, BASE + route, wait_ms=800)
                content = await safe_content(ap)
                if "[object Object]" in content:
                    broken.append(f"{route}: [object Object]")
                if ">undefined<" in content:
                    broken.append(f"{route}: >undefined<")
                if '"undefined"' in content and route != "/login":
                    broken.append(f"{route}: prop=undefined")
            log("UI: no broken render markers", len(broken) == 0,
                "; ".join(broken) if broken else "clean")

            await ap.close()
            await auth_ctx.close()

        # 14. Console errors
        log("Console: no JS errors across all pages", len(console_errors) == 0,
            f"{len(console_errors)} errors" if console_errors else "clean")
        if console_errors:
            seen = set()
            for e in console_errors:
                if e not in seen:
                    print(f"    ERR: {e}")
                    seen.add(e)
                if len(seen) >= 8:
                    break

        await browser.close()

    # Summary
    print("\n" + "=" * 60)
    print("BROWSER INSPECTION SUMMARY")
    print("=" * 60)
    passed = sum(1 for r in results if r["status"] == "PASS")
    failed = sum(1 for r in results if r["status"] == "FAIL")
    skipped = sum(1 for r in results if r["status"] == "SKIP")
    print(f"PASSED: {passed}   FAILED: {failed}   SKIPPED: {skipped}   TOTAL: {len(results)}")
    if failed > 0:
        print("\nFailed checks:")
        for r in results:
            if r["status"] == "FAIL":
                print(f"  ✗  {r['check']}" + (f":  {r['detail']}" if r["detail"] else ""))

    with open("/tmp/opencorde_browser_results.json", "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nScreenshots → {SHOTS}/")
    print("Results JSON → /tmp/opencorde_browser_results.json")

asyncio.run(run_tests())
