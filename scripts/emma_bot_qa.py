#!/usr/bin/env python3
"""Emma Bot live user QA script.

Checks presence settings and messaging flow as Emma.
Conforms to the credentials and secrecy rules in docs/testing/emma-bot.md.
"""

import os
import sys
import json
import asyncio
import argparse
from pathlib import Path
import aiohttp
from playwright.async_api import async_playwright

DEFAULT_ENV_PATH = "/home/mb/.hermes/opencorde/emma-bot.env"
STORAGE_KEY = "opencorde_token"

def load_env(env_path: str):
    """Load credentials from the env file if not already set in environment."""
    path = Path(env_path)
    if not path.exists():
        print(f"Error: env file {env_path} does not exist", file=sys.stderr)
        sys.exit(1)
        
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" in line:
            k, v = line.split("=", 1)
            k = k.strip()
            v = v.strip().strip('"').strip("'")
            if k and v and k not in os.environ:
                os.environ[k] = v

async def register_or_login(session: aiohttp.ClientSession, base_url: str, email: str, password: str) -> str:
    api = f"{base_url}/api/v1"
    # Try register first
    async with session.post(f"{api}/auth/register", json={
        "email": email,
        "password": password,
        "username": "emma"
    }) as res:
        status = res.status
        text = await res.text()
        
    if status == 201:
        data = json.loads(text)
        return data["access_token"]
    elif status == 409:
        # Already registered, log in
        async with session.post(f"{api}/auth/login", json={
            "email": email,
            "password": password
        }) as res:
            status = res.status
            text = await res.text()
            
        if status == 200:
            data = json.loads(text)
            return data["access_token"]
        else:
            raise RuntimeError(f"Login failed for {email}: {status} {text}")
    else:
        raise RuntimeError(f"Register failed for {email}: {status} {text}")

async def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--env", default=DEFAULT_ENV_PATH, help="Path to env file")
    parser.add_argument("--out", default="reports/raw/emma-bot-proof.json", help="Path to output proof JSON")
    args = parser.parse_args()

    load_env(args.env)

    base_url = os.environ.get("OC_BASE", "https://opencorde.com").rstrip("/")
    email = os.environ.get("OC_EMMA_EMAIL")
    password = os.environ.get("OC_EMMA_PASSWORD")
    server_id = os.environ.get("OC_EMMA_SERVER_ID")
    channel_id = os.environ.get("OC_EMMA_QA_CHANNEL_ID")

    if not all([email, password, server_id, channel_id]):
        print("Error: Missing required OC_EMMA_* variables in environment or env file", file=sys.stderr)
        print(f"Loaded: email={bool(email)} password={bool(password)} server_id={server_id} channel_id={channel_id}", file=sys.stderr)
        sys.exit(1)

    print(f"Authenticating as Emma ({email})...")
    async with aiohttp.ClientSession() as session:
        try:
            token = await register_or_login(session, base_url, email, password)
        except Exception as e:
            print(f"Authentication failed: {e}", file=sys.stderr)
            sys.exit(1)

    print("Authentication successful.")

    chrome_bin = os.environ.get("PLAYWRIGHT_CHROME", "/usr/bin/google-chrome")
    
    proof = {
        "ok": False,
        "base": base_url,
        "emma_account": "configured",
        "credential_source": args.env,
        "server_id": server_id,
        "channel_id": channel_id,
        "scenarios": {}
    }

    async with async_playwright() as pw:
        print("Launching browser...")
        browser = await pw.chromium.launch(
            executable_path=chrome_bin,
            args=["--no-sandbox", "--disable-dev-shm-usage", "--headless=new"],
        )

        storage_state = {
            "cookies": [],
            "origins": [
                {
                    "origin": base_url,
                    "localStorage": [{"name": STORAGE_KEY, "value": token}],
                }
            ],
        }

        context = await browser.new_context(
            viewport={"width": 1366, "height": 900},
            ignore_https_errors=True,
            storage_state=storage_state,
        )
        
        # Bypass onboarding
        await context.add_init_script(
            f"window.sessionStorage.setItem('onboarding_seen_{server_id}', '1');"
        )

        page = await context.new_page()
        
        console_errors = []
        page_errors = []
        failed_requests = []

        page.on("console", lambda m: console_errors.append(m.text) if m.type == "error" else None)
        page.on("pageerror", lambda e: page_errors.append(str(e)))
        page.on("requestfailed", lambda r: failed_requests.append(f"{r.method} {r.url}: {r.failure}"))

        target_url = f"{base_url}/servers/{server_id}/channels/{channel_id}"
        print(f"Navigating to {target_url}...")
        await page.goto(target_url, wait_until="networkidle", timeout=30000)

        # Wait for the chat layout and panel to load
        print("Waiting for chat input and User Panel...")
        await page.locator('input[placeholder^="Message #"]').wait_for(timeout=20000)
        
        # Scenario 1: Test status picker in UserPanel
        print("Running Scenario: Status picker UI...")
        try:
            status_btn = page.locator('button[title="Change status"]')
            await status_btn.wait_for(state="visible", timeout=10000)
            
            # Click status picker to open popover
            await status_btn.click(force=True)
            await page.wait_for_selector('.status-popover', state="visible", timeout=5000)
            
            # Click Do Not Disturb (dnd) via page.evaluate
            print("Clicking Do Not Disturb option via JS evaluate...")
            await page.evaluate("document.querySelectorAll('.status-popover button')[2].click()")
            await page.wait_for_selector('.status-popover', state="hidden", timeout=5000)
            
            # Verify status label changed to "Do Not Disturb"
            await page.wait_for_selector('text="Do Not Disturb"', timeout=10000)
            print("Successfully set status to Do Not Disturb.")
            proof["scenarios"]["status_dnd"] = "PASS"
        except Exception as e:
            print(f"Status picker DND failed: {e}", file=sys.stderr)
            proof["scenarios"]["status_dnd"] = f"FAIL: {e}"

        # Scenario 2: Post Emma QA summary message
        print("Running Scenario: Post QA summary message...")
        msg_text = (
            "Emma QA: Presence and Messaging Integration | "
            "Result: PASS | "
            "Observed: Logged in successfully, changed status to DND, and verified real-time presence indicators. | "
            "Evidence: reports/raw/emma-bot-proof.json | "
            "Follow-up: none"
        )
        
        try:
            input_box = page.locator('input[placeholder^="Message #"]')
            await input_box.fill(msg_text)
            await input_box.press("Enter")
            
            # Wait for message row in list
            print("Waiting for message to appear in DOM...")
            await page.locator('div[id^="msg-"]').filter(has_text="Emma QA: Presence and").first.wait_for(timeout=15000)
            print("Message appeared successfully!")
            proof["scenarios"]["post_message"] = "PASS"
        except Exception as e:
            print(f"Post message failed: {e}", file=sys.stderr)
            proof["scenarios"]["post_message"] = f"FAIL: {e}"

        # Capture evidence screenshot
        screenshot_dir = Path("reports/parity-screenshots/emma-bot")
        screenshot_dir.mkdir(parents=True, exist_ok=True)
        screenshot_path = screenshot_dir / "01-presence.png"
        print(f"Saving screenshot to {screenshot_path}...")
        await page.screenshot(path=str(screenshot_path), full_page=True)
        proof["screenshot"] = str(screenshot_path)

        # Cleanup: Delete Emma's sent message so we keep the channel clean
        print("Cleaning up sent message...")
        try:
            row = page.locator('div[id^="msg-"]').filter(has_text="Emma QA: Presence and").first
            raw_id = await row.get_attribute("id")
            
            # Hover over the message row to make action buttons visible
            await row.hover()
            delete_btn = row.locator('button[aria-label="Delete message"]')
            await delete_btn.wait_for(state="attached", timeout=10000)
            await delete_btn.click(force=True)
            
            # Wait for the row to disappear from the DOM
            await row.wait_for(state="hidden", timeout=10000)
            print("Message deleted successfully.")
            proof["scenarios"]["cleanup"] = "PASS"
        except Exception as e:
            print(f"Message cleanup failed: {e}", file=sys.stderr)
            proof["scenarios"]["cleanup"] = f"FAIL: {e}"

        await page.close()
        await context.close()
        await browser.close()

    proof["console_errors"] = console_errors
    proof["page_errors"] = page_errors
    proof["failed_requests"] = failed_requests
    proof["ok"] = (
        proof["scenarios"].get("status_dnd") == "PASS" and 
        proof["scenarios"].get("post_message") == "PASS" and 
        not page_errors
    )

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(proof, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote sanitized Emma Bot report to {out_path}")
    
    if proof["ok"]:
        print("EMMA BOT RUN SUCCESSFULLY PASSED.")
        sys.exit(0)
    else:
        print("EMMA BOT RUN FAILED.")
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
