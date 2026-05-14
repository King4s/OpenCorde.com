#!/usr/bin/env python3
"""Live UI proof for private channels and role management.

The script creates temporary permission fixtures in the live database, proves the
same behavior through the browser, writes sanitized reports/screenshots, then
removes the fixtures. It does not store access tokens in report output.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import subprocess
import time
from pathlib import Path
from typing import Any

import aiohttp
import jwt
from playwright.async_api import Browser, Page, async_playwright


BASE = os.environ.get("OC_BASE", "https://opencorde.com").rstrip("/")
API = f"{BASE}/api/v1"
OUT = Path(os.environ.get("OC_PERMISSIONS_UI_QA_OUT", "reports/raw/permissions-ui-proof.json"))
SCREENSHOT_DIR = Path(
    os.environ.get("OC_PERMISSIONS_UI_QA_SCREENSHOTS", "reports/parity-screenshots/permissions-ui")
)
STORAGE_KEY = "opencorde_token"

MEMBER_EMAIL = os.environ.get("OC_MEMBER_EMAIL", "browsertest@opencorde.local")
MEMBER_PASSWORD = os.environ.get("OC_MEMBER_PASSWORD", "BrowserTest@99")
LIMITED_USERNAME = os.environ.get("OC_LIMITED_USERNAME", "permission_limited")
LIMITED_EMAIL = os.environ.get("OC_LIMITED_EMAIL", "permission-limited@opencorde.com")
LIMITED_PASSWORD = os.environ.get("OC_LIMITED_PASSWORD", "PermissionSmoke@99")

VIEW_CHANNEL = 1 << 10


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


def create_short_lived_token(jwt_secret: str, user_id: str, username: str) -> str:
    now = int(time.time())
    claims = {
        "sub": user_id,
        "username": username,
        "token_type": "access",
        "iat": now,
        "exp": now + 300,
    }
    return jwt.encode(claims, jwt_secret, algorithm="HS256")


def sql_literal(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


async def request_json(session: aiohttp.ClientSession, method: str, url: str, **kwargs):
    async with session.request(method, url, **kwargs) as response:
        text = await response.text()
        try:
            data = json.loads(text) if text else None
        except json.JSONDecodeError:
            data = text
        return response.status, data


async def login(session: aiohttp.ClientSession, email: str, password: str) -> str:
    status, data = await request_json(
        session,
        "POST",
        f"{API}/auth/login",
        json={"email": email, "password": password},
    )
    if status != 200:
        raise RuntimeError(f"login failed for {email}: {status} {data}")
    return data["access_token"]


async def get_owner_token(session: aiohttp.ClientSession, env_path: Path) -> tuple[str, str]:
    explicit_member_login = "OC_MEMBER_EMAIL" in os.environ or "OC_MEMBER_PASSWORD" in os.environ
    if explicit_member_login:
        return await login(session, MEMBER_EMAIL, MEMBER_PASSWORD), "member_login_from_env"

    try:
        return await login(session, MEMBER_EMAIL, MEMBER_PASSWORD), "default_member_login"
    except RuntimeError:
        file_env = load_env_file(env_path)
        jwt_secret = os.environ.get("JWT_SECRET") or file_env.get("JWT_SECRET")
        admin_ids = os.environ.get("ADMIN_USER_IDS") or file_env.get("ADMIN_USER_IDS")
        if not jwt_secret or not admin_ids:
            raise
        admin_id = next((item.strip() for item in admin_ids.split(",") if item.strip()), "")
        if not admin_id:
            raise RuntimeError("ADMIN_USER_IDS does not contain a usable user ID")
        return (
            create_short_lived_token(jwt_secret, admin_id, "permissions-ui-proof"),
            "short_lived_jwt_from_local_env",
        )


async def ensure_limited_user(
    session: aiohttp.ClientSession, env_path: Path
) -> tuple[str, str]:
    status, _data = await request_json(
        session,
        "POST",
        f"{API}/auth/register",
        json={
            "username": LIMITED_USERNAME,
            "email": LIMITED_EMAIL,
            "password": LIMITED_PASSWORD,
        },
    )
    if status not in (200, 201, 409):
        raise RuntimeError(f"limited register failed: {status}")
    try:
        return (
            await login(session, LIMITED_EMAIL, LIMITED_PASSWORD),
            "register_or_login_default_test_account",
        )
    except RuntimeError:
        file_env = load_env_file(env_path)
        jwt_secret = os.environ.get("JWT_SECRET") or file_env.get("JWT_SECRET")
        if not jwt_secret:
            raise
        limited_user_id = psql_value(
            "SELECT id FROM users "
            f"WHERE email = {sql_literal(LIMITED_EMAIL)} "
            f"   OR username = {sql_literal(LIMITED_USERNAME)} "
            "ORDER BY created_at DESC LIMIT 1;"
        )
        if not limited_user_id:
            raise RuntimeError(f"limited user {LIMITED_EMAIL} exists with unknown id")
        return (
            create_short_lived_token(jwt_secret, limited_user_id, LIMITED_USERNAME),
            "short_lived_jwt_from_local_env_for_existing_limited_user",
        )


def psql(sql: str) -> None:
    subprocess.run(
        [
            "docker",
            "exec",
            "opencorde-postgres",
            "psql",
            "-U",
            "opencorde",
            "-d",
            "opencorde",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            sql,
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )


def psql_value(sql: str) -> str:
    result = subprocess.run(
        [
            "docker",
            "exec",
            "opencorde-postgres",
            "psql",
            "-U",
            "opencorde",
            "-d",
            "opencorde",
            "-t",
            "-A",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            sql,
        ],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return result.stdout.strip()


def cleanup_fixture(
    *,
    server_id: str,
    limited_user_id: str,
    channel_id: int,
    allowed_role_id: int,
    ui_role_prefix: str,
) -> None:
    psql(
        f"""
        DELETE FROM channel_permission_overrides
        WHERE channel_id = {channel_id}
           OR target_id IN (
             SELECT id FROM roles
             WHERE server_id = {server_id}
               AND name LIKE '{ui_role_prefix}%'
           );

        DELETE FROM member_roles
        WHERE role_id = {allowed_role_id}
           OR role_id IN (
             SELECT id FROM roles
             WHERE server_id = {server_id}
               AND name LIKE '{ui_role_prefix}%'
           )
           OR (user_id = {limited_user_id} AND server_id = {server_id});

        DELETE FROM channels WHERE id = {channel_id};

        DELETE FROM roles
        WHERE id = {allowed_role_id}
           OR (server_id = {server_id} AND name LIKE '{ui_role_prefix}%');

        DELETE FROM server_members
        WHERE user_id = {limited_user_id} AND server_id = {server_id};
        """
    )


def create_fixture(
    *,
    server_id: str,
    limited_user_id: str,
    channel_id: int,
    channel_name: str,
    allowed_role_id: int,
    allowed_role_name: str,
) -> None:
    psql(
        f"""
        INSERT INTO server_members (user_id, server_id)
        VALUES ({limited_user_id}, {server_id})
        ON CONFLICT DO NOTHING;

        INSERT INTO channels (id, server_id, name, channel_type, position)
        VALUES ({channel_id}, {server_id}, '{channel_name}', 0, 9999);

        INSERT INTO roles (id, server_id, name, permissions, position)
        VALUES ({allowed_role_id}, {server_id}, '{allowed_role_name}', {VIEW_CHANNEL}, 10);

        INSERT INTO channel_permission_overrides (channel_id, target_type, target_id, allow_bits, deny_bits)
        VALUES
          ({channel_id}, 'role', {server_id}, 0, {VIEW_CHANNEL}),
          ({channel_id}, 'role', {allowed_role_id}, {VIEW_CHANNEL}, 0);
        """
    )


def grant_allowed_role(
    *, server_id: str, limited_user_id: str, allowed_role_id: int
) -> None:
    psql(
        f"""
        INSERT INTO member_roles (user_id, server_id, role_id)
        VALUES ({limited_user_id}, {server_id}, {allowed_role_id})
        ON CONFLICT DO NOTHING;
        """
    )


async def new_context(browser: Browser, token: str, server_id: str):
    context = await browser.new_context(
        viewport={"width": 1366, "height": 900},
        storage_state={
            "cookies": [],
            "origins": [
                {
                    "origin": BASE,
                    "localStorage": [{"name": STORAGE_KEY, "value": token}],
                }
            ],
        },
    )
    await context.add_init_script(
        f"window.sessionStorage.setItem('onboarding_seen_{server_id}', '1');"
    )
    return context


def attach_page_logging(page: Page, bucket: dict[str, list[Any]]) -> None:
    def on_request_failed(req):
        failure = str(req.failure)
        item = {"url": req.url, "method": req.method, "failure": failure}
        if (
            req.method == "DELETE"
            and "/api/v1/servers/" in req.url
            and "/roles/" in req.url
            and "net::ERR_ABORTED" in failure
        ):
            bucket.setdefault("ignoredFailedRequests", []).append(item)
            return
        bucket["failedRequests"].append(item)

    page.on(
        "console",
        lambda msg: bucket["consoleErrors"].append(msg.text)
        if msg.type == "error" and "Failed to load resource" not in msg.text
        else None,
    )
    page.on("pageerror", lambda err: bucket["pageErrors"].append(str(err)))
    page.on("requestfailed", on_request_failed)


async def screenshot(page: Page, name: str) -> str:
    SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)
    path = SCREENSHOT_DIR / name
    await page.screenshot(path=str(path), full_page=True)
    return str(path)


async def prove_private_channel(
    browser: Browser,
    *,
    token: str,
    server_id: str,
    channel_id: int,
    channel_name: str,
    limited_user_id: str,
    allowed_role_id: int,
) -> dict[str, Any]:
    bucket: dict[str, list[Any]] = {
        "consoleErrors": [],
        "pageErrors": [],
        "failedRequests": [],
        "ignoredFailedRequests": [],
    }
    context = await new_context(browser, token, server_id)
    page = await context.new_page()
    attach_page_logging(page, bucket)

    await page.goto(f"{BASE}/servers/{server_id}", wait_until="networkidle", timeout=30000)
    await page.wait_for_timeout(1000)
    hidden_count = await page.get_by_text(channel_name, exact=True).count()
    hidden_screenshot = await screenshot(page, "private-channel-hidden.png")

    grant_allowed_role(
        server_id=server_id,
        limited_user_id=limited_user_id,
        allowed_role_id=allowed_role_id,
    )
    await page.reload(wait_until="networkidle", timeout=30000)
    channel_text = page.get_by_text(channel_name, exact=True).first
    await channel_text.wait_for(timeout=15000)
    visible_count = await page.get_by_text(channel_name, exact=True).count()
    await channel_text.click()
    await page.wait_for_url(f"**/servers/{server_id}/channels/{channel_id}", timeout=15000)
    visible_screenshot = await screenshot(page, "private-channel-visible.png")

    await context.close()
    return {
        "hiddenBeforeRole": hidden_count == 0,
        "visibleAfterRole": visible_count > 0,
        "channelUrlOpened": f"/servers/{server_id}/channels/{channel_id}" in page.url,
        "screenshots": {
            "hidden": hidden_screenshot,
            "visible": visible_screenshot,
        },
        **bucket,
    }


async def prove_channel_permissions(
    browser: Browser,
    *,
    token: str,
    server_id: str,
    channel_id: int,
    channel_name: str,
    allowed_role_name: str,
) -> dict[str, Any]:
    bucket: dict[str, list[Any]] = {
        "consoleErrors": [],
        "pageErrors": [],
        "failedRequests": [],
        "ignoredFailedRequests": [],
    }
    context = await new_context(browser, token, server_id)
    page = await context.new_page()
    attach_page_logging(page, bucket)

    await page.goto(f"{BASE}/servers/{server_id}", wait_until="networkidle", timeout=30000)
    channel_text = page.get_by_text(channel_name, exact=True).first
    await channel_text.wait_for(timeout=15000)
    await channel_text.click()
    await page.wait_for_url(f"**/servers/{server_id}/channels/{channel_id}", timeout=15000)
    await page.get_by_label("Channel settings").click()
    await page.get_by_role("button", name="Permissions").click()
    await page.get_by_text("View Channel", exact=True).first.wait_for(timeout=15000)

    role_visible = await page.get_by_text(allowed_role_name, exact=True).count() > 0
    view_channel_visible = await page.get_by_text("View Channel", exact=True).count() > 0
    screenshot_path = await screenshot(page, "channel-permissions-overrides.png")

    await context.close()
    return {
        "channelName": channel_name,
        "allowedRoleVisible": role_visible,
        "viewChannelControlVisible": view_channel_visible,
        "screenshot": screenshot_path,
        **bucket,
    }


async def prove_role_management(
    browser: Browser,
    *,
    token: str,
    server_id: str,
    created_name: str,
    renamed_name: str,
) -> dict[str, Any]:
    bucket: dict[str, list[Any]] = {
        "consoleErrors": [],
        "pageErrors": [],
        "failedRequests": [],
        "ignoredFailedRequests": [],
    }
    context = await new_context(browser, token, server_id)
    page = await context.new_page()
    page.on("dialog", lambda dialog: asyncio.create_task(dialog.accept()))
    attach_page_logging(page, bucket)

    await page.goto(f"{BASE}/servers/{server_id}/settings", wait_until="networkidle", timeout=30000)
    await page.get_by_role("button", name="Roles & Permissions").click()
    await page.get_by_placeholder("Role name").fill(created_name)
    await page.get_by_role("button", name="Add", exact=True).click()
    await page.get_by_text(created_name, exact=True).wait_for(timeout=15000)
    created_screenshot = await screenshot(page, "role-management-created.png")

    role_row = page.locator(f'[data-role-name="{created_name}"]').first
    await role_row.hover()
    await role_row.get_by_text("Edit", exact=True).first.click(force=True)
    await page.locator('input:not([type]), input[type="text"]').last.fill(renamed_name)
    await page.get_by_role("button", name="Save").click()
    await page.get_by_text(renamed_name, exact=True).wait_for(timeout=15000)
    renamed_screenshot = await screenshot(page, "role-management-renamed.png")

    renamed_row = page.locator(f'[data-role-name="{renamed_name}"]').first
    await renamed_row.hover()
    await renamed_row.get_by_text("Delete", exact=True).first.click(force=True)
    await page.get_by_text(renamed_name, exact=True).wait_for(state="detached", timeout=15000)
    deleted_screenshot = await screenshot(page, "role-management-deleted.png")

    await context.close()
    return {
        "createdVisible": True,
        "renamedVisible": True,
        "deletedFromList": True,
        "screenshots": {
            "created": created_screenshot,
            "renamed": renamed_screenshot,
            "deleted": deleted_screenshot,
        },
        **bucket,
    }


def no_browser_errors(section: dict[str, Any]) -> bool:
    return (
        not section.get("consoleErrors")
        and not section.get("pageErrors")
        and not section.get("failedRequests")
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--env", default=os.environ.get("OC_ENV_FILE", ".env"))
    parser.add_argument("--fail-on-issues", action="store_true")
    return parser.parse_args()


async def main() -> int:
    args = parse_args()
    stamp = int(time.time() * 1000)
    suffix = str(stamp)[-8:]
    channel_id = stamp * 1000 + 601
    allowed_role_id = stamp * 1000 + 602
    channel_name = f"private-ui-{suffix}"
    allowed_role_name = f"allow-ui-{suffix}"
    ui_role_prefix = f"managed-ui-{suffix}"
    ui_role_name = ui_role_prefix
    ui_role_renamed = f"{ui_role_prefix}-renamed"

    cleanup_ok = False
    report: dict[str, Any] = {
        "ok": False,
        "base": BASE,
        "auth": {
            "ownerMethod": None,
            "limitedMethod": "register_or_login_default_test_account",
            "tokensStoredInReport": False,
        },
    }

    async with aiohttp.ClientSession() as session:
        owner_token, owner_method = await get_owner_token(session, Path(args.env))
        report["auth"]["ownerMethod"] = owner_method
        limited_token, limited_method = await ensure_limited_user(session, Path(args.env))
        report["auth"]["limitedMethod"] = limited_method

        owner_headers = {"Authorization": f"Bearer {owner_token}"}
        limited_headers = {"Authorization": f"Bearer {limited_token}"}

        status, owner_profile = await request_json(
            session, "GET", f"{API}/users/@me", headers=owner_headers
        )
        if status != 200 or not owner_profile:
            raise RuntimeError(f"owner profile baseline failed: {status} {owner_profile}")
        owner_user_id = owner_profile["id"]

        status, servers = await request_json(session, "GET", f"{API}/servers", headers=owner_headers)
        if status != 200 or not servers:
            raise RuntimeError(f"owner server baseline failed: {status} {servers}")
        owner_server = next(
            (server for server in servers if str(server.get("owner_id")) == str(owner_user_id)),
            None,
        )
        if not owner_server:
            raise RuntimeError("owner token has no owned server for role-management UI proof")
        server_id = owner_server["id"]

        status, limited_profile = await request_json(
            session, "GET", f"{API}/users/@me", headers=limited_headers
        )
        if status != 200 or not limited_profile:
            raise RuntimeError(f"limited profile baseline failed: {status} {limited_profile}")
        limited_user_id = limited_profile["id"]

        cleanup_fixture(
            server_id=server_id,
            limited_user_id=limited_user_id,
            channel_id=channel_id,
            allowed_role_id=allowed_role_id,
            ui_role_prefix=ui_role_prefix,
        )
        create_fixture(
            server_id=server_id,
            limited_user_id=limited_user_id,
            channel_id=channel_id,
            channel_name=channel_name,
            allowed_role_id=allowed_role_id,
            allowed_role_name=allowed_role_name,
        )
        grant_allowed_role(
            server_id=server_id,
            limited_user_id=owner_user_id,
            allowed_role_id=allowed_role_id,
        )

        try:
            chrome = os.environ.get("PLAYWRIGHT_CHROME", "/usr/bin/google-chrome")
            async with async_playwright() as pw:
                browser = await pw.chromium.launch(
                    executable_path=chrome,
                    args=["--no-sandbox", "--disable-dev-shm-usage", "--headless=new"],
                )
                private_channel = await prove_private_channel(
                    browser,
                    token=limited_token,
                    server_id=server_id,
                    channel_id=channel_id,
                    channel_name=channel_name,
                    limited_user_id=limited_user_id,
                    allowed_role_id=allowed_role_id,
                )
                channel_permissions = await prove_channel_permissions(
                    browser,
                    token=owner_token,
                    server_id=server_id,
                    channel_id=channel_id,
                    channel_name=channel_name,
                    allowed_role_name=allowed_role_name,
                )
                role_management = await prove_role_management(
                    browser,
                    token=owner_token,
                    server_id=server_id,
                    created_name=ui_role_name,
                    renamed_name=ui_role_renamed,
                )
                await browser.close()

            report.update(
                {
                    "fixture": {
                        "serverId": server_id,
                        "channelId": str(channel_id),
                        "channelName": channel_name,
                        "allowedRoleId": str(allowed_role_id),
                        "allowedRoleName": allowed_role_name,
                        "uiManagedRoleName": ui_role_name,
                    },
                    "browser": {
                        "privateChannel": private_channel,
                        "channelPermissions": channel_permissions,
                        "roleManagement": role_management,
                    },
                }
            )
            report["ok"] = (
                private_channel["hiddenBeforeRole"]
                and private_channel["visibleAfterRole"]
                and private_channel["channelUrlOpened"]
                and channel_permissions["allowedRoleVisible"]
                and channel_permissions["viewChannelControlVisible"]
                and role_management["createdVisible"]
                and role_management["renamedVisible"]
                and role_management["deletedFromList"]
                and no_browser_errors(private_channel)
                and no_browser_errors(channel_permissions)
                and no_browser_errors(role_management)
            )
        finally:
            cleanup_fixture(
                server_id=server_id,
                limited_user_id=limited_user_id,
                channel_id=channel_id,
                allowed_role_id=allowed_role_id,
                ui_role_prefix=ui_role_prefix,
            )
            cleanup_ok = True

    report["cleanup"] = {"temporaryFixturesRemoved": cleanup_ok}
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    print(f"Wrote {OUT}")

    if args.fail_on_issues and not report["ok"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
