#!/usr/bin/env python3
"""Admin dashboard LiveKit health proof for OpenCorde.

This script verifies the admin stats API and the rendered `/admin` dashboard
without storing credentials or JWTs in the report. It reads `JWT_SECRET` and
`ADMIN_USER_IDS` from `.env`, creates a short-lived access token for the first
configured admin ID, and injects that token into browser localStorage.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import time
from pathlib import Path
from typing import Any

import jwt
from playwright.async_api import async_playwright


DEFAULT_BASE = "https://opencorde.com"
DEFAULT_OUT = "reports/raw/admin-livekit-health-ui.json"
DEFAULT_SCREENSHOT = "reports/parity-screenshots/admin-livekit-health.png"
STORAGE_KEY = "opencorde_token"


def load_env_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.exists():
        return values

    for raw_line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
    return values


def get_config(env_path: Path) -> tuple[str, str]:
    file_env = load_env_file(env_path)
    jwt_secret = os.environ.get("JWT_SECRET") or file_env.get("JWT_SECRET")
    admin_ids = os.environ.get("ADMIN_USER_IDS") or file_env.get("ADMIN_USER_IDS")

    if not jwt_secret:
        raise RuntimeError("JWT_SECRET is missing from environment and .env")
    if not admin_ids:
        raise RuntimeError("ADMIN_USER_IDS is missing from environment and .env")

    admin_id = next((item.strip() for item in admin_ids.split(",") if item.strip()), "")
    if not admin_id:
        raise RuntimeError("ADMIN_USER_IDS does not contain a usable user ID")
    return jwt_secret, admin_id


def create_admin_token(jwt_secret: str, admin_id: str) -> str:
    now = int(time.time())
    claims = {
        "sub": admin_id,
        "username": "admin-health-proof",
        "token_type": "access",
        "iat": now,
        "exp": now + 300,
    }
    return jwt.encode(claims, jwt_secret, algorithm="HS256")


async def run_browser_check(base: str, token: str, screenshot_path: Path) -> dict[str, Any]:
    chrome = os.environ.get("PLAYWRIGHT_CHROME", "/usr/bin/google-chrome")
    base = base.rstrip("/")
    console_errors: list[str] = []
    page_errors: list[str] = []
    failed_requests: list[dict[str, str | None]] = []

    async with async_playwright() as pw:
        browser = await pw.chromium.launch(
            executable_path=chrome,
            args=["--no-sandbox", "--disable-dev-shm-usage", "--headless=new"],
        )
        context = await browser.new_context(
            viewport={"width": 1280, "height": 900},
            storage_state={
                "cookies": [],
                "origins": [
                    {
                        "origin": base,
                        "localStorage": [{"name": STORAGE_KEY, "value": token}],
                    }
                ],
            },
        )
        page = await context.new_page()
        page.on(
            "console",
            lambda msg: console_errors.append(msg.text)
            if msg.type == "error" and "Failed to load resource" not in msg.text
            else None,
        )
        page.on("pageerror", lambda err: page_errors.append(str(err)))
        page.on(
            "requestfailed",
            lambda req: failed_requests.append(
                {"url": req.url, "failure": str(req.failure)}
            ),
        )

        response = await page.goto(f"{base}/admin", wait_until="networkidle", timeout=30000)
        await page.wait_for_timeout(1000)

        screenshot_path.parent.mkdir(parents=True, exist_ok=True)
        await page.screenshot(path=str(screenshot_path), full_page=True)

        livekit_panel = page.locator("text=LiveKit Health").first
        local_row = page.locator("text=Local").first
        public_row = page.locator("text=Public Proxy").first
        ok_label_count = await page.locator("text=OK").count()
        content = await page.content()

        result = {
            "url": page.url,
            "status": response.status if response else None,
            "livekitPanelVisible": await livekit_panel.count() > 0,
            "localRowVisible": await local_row.count() > 0,
            "publicProxyRowVisible": await public_row.count() > 0,
            "okLabelCount": ok_label_count,
            "containsToken": token in content,
            "consoleErrors": console_errors,
            "pageErrors": page_errors,
            "failedRequests": failed_requests,
            "screenshot": str(screenshot_path),
        }
        result["ok"] = (
            result["status"] == 200
            and result["livekitPanelVisible"]
            and result["localRowVisible"]
            and result["publicProxyRowVisible"]
            and result["okLabelCount"] >= 1
            and not result["containsToken"]
            and not result["pageErrors"]
        )

        await context.close()
        await browser.close()
        return result


async def fetch_admin_stats(base: str, token: str) -> dict[str, Any]:
    import urllib.error
    import urllib.request

    req = urllib.request.Request(
        f"{base.rstrip('/')}/api/v1/admin/stats",
        headers={"Authorization": f"Bearer {token}"},
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            body = response.read().decode("utf-8", errors="replace")
            data = json.loads(body)
            health = data.get("livekit_health", {})
            return {
                "ok": response.status == 200
                and bool(health.get("local"))
                and bool(health.get("public")),
                "status": response.status,
                "livekitHealth": health,
            }
    except (urllib.error.URLError, TimeoutError, OSError, json.JSONDecodeError) as exc:
        return {"ok": False, "status": None, "error": str(exc)}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default=os.environ.get("OC_BASE", DEFAULT_BASE))
    parser.add_argument("--env", default=os.environ.get("OC_ENV_FILE", ".env"))
    parser.add_argument("--out", default=os.environ.get("OC_ADMIN_LIVEKIT_QA_OUT", DEFAULT_OUT))
    parser.add_argument(
        "--screenshot",
        default=os.environ.get("OC_ADMIN_LIVEKIT_QA_SCREENSHOT", DEFAULT_SCREENSHOT),
    )
    parser.add_argument("--fail-on-issues", action="store_true")
    return parser.parse_args()


async def main() -> int:
    args = parse_args()
    jwt_secret, admin_id = get_config(Path(args.env))
    token = create_admin_token(jwt_secret, admin_id)

    api_result = await fetch_admin_stats(args.base, token)
    browser_result = await run_browser_check(args.base, token, Path(args.screenshot))
    report = {
        "ok": bool(api_result.get("ok")) and bool(browser_result.get("ok")),
        "base": args.base.rstrip("/"),
        "auth": {
            "method": "short_lived_jwt_from_local_env",
            "adminIdConfigured": True,
            "tokenStoredInReport": False,
        },
        "api": api_result,
        "browser": browser_result,
    }

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    print(json.dumps(report, indent=2))
    print(f"Wrote {out_path}")
    if args.fail_on_issues and not report["ok"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
