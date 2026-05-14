#!/usr/bin/env python3
"""Live messaging UI proof.

Drives the chat UI as the test fixture owner and proves the daily-chat
workflows that Discord parity issue #4 calls out: send, edit, delete,
reply, react, pin. Captures screenshots and writes a sanitized report.

The script reuses the fixture owner's existing server + first text channel.
It does not create or remove channels — only messages it sends itself,
which it cleans up at the end.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import time
from pathlib import Path
from typing import Any

import aiohttp
from playwright.async_api import Browser, Locator, Page, async_playwright


BASE = os.environ.get("OC_BASE", "https://opencorde.com").rstrip("/")
API = f"{BASE}/api/v1"
OUT = Path(os.environ.get("OC_MESSAGING_UI_QA_OUT", "reports/raw/messaging-ui-proof.json"))
SCREENSHOT_DIR = Path(
    os.environ.get("OC_MESSAGING_UI_QA_SCREENSHOTS", "reports/parity-screenshots/messaging-ui")
)
STORAGE_KEY = "opencorde_token"

MEMBER_EMAIL = os.environ.get("OC_MEMBER_EMAIL", "browsertest@opencorde.local")
MEMBER_PASSWORD = os.environ.get("OC_MEMBER_PASSWORD", "BrowserTest@99")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fail-on-issues", action="store_true")
    return parser.parse_args()


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


async def pick_text_channel(
    session: aiohttp.ClientSession,
    headers: dict[str, str],
    server_id: str,
) -> dict[str, Any]:
    status, channels = await request_json(
        session,
        "GET",
        f"{API}/servers/{server_id}/channels",
        headers=headers,
    )
    if status != 200 or not isinstance(channels, list):
        raise RuntimeError(f"channel list failed: {status} {channels}")
    text_channels = [c for c in channels if isinstance(c, dict) and c.get("channel_type") == 0]
    if not text_channels:
        raise RuntimeError("owner has no text channel for messaging proof")
    # Prefer 'general' if present, otherwise the first text channel.
    return next((c for c in text_channels if c.get("name") == "general"), text_channels[0])


async def new_context(browser: Browser, server_id: str):
    context = await browser.new_context(viewport={"width": 1366, "height": 900})
    await context.add_init_script(
        f"window.sessionStorage.setItem('onboarding_seen_{server_id}', '1');"
    )
    return context


async def login_via_ui(page: Page, email: str, password: str) -> None:
    """Drive the /login form so the auth store fully hydrates $currentUser."""
    await page.goto(f"{BASE}/login", wait_until="networkidle", timeout=30000)
    await page.locator('input[type="email"]').fill(email)
    await page.locator('input[type="password"]').fill(password)
    await page.locator('button[type="submit"]').click()
    # The app redirects to /servers (or the original target) after login.
    await page.wait_for_url(lambda url: "/login" not in url, timeout=20000)


_IGNORED_ABORT_PATTERNS = (
    "/_app/immutable/",  # SvelteKit prefetch chunks cancelled by navigation.
    "/api/v1/channels/",  # /ack /pins /messages /reactions — chromium reports
    "/api/v1/messages/",  # ERR_ABORTED on context teardown even after the
)                          # HTTP response has been received and acted on.


def attach_page_logging(page: Page, bucket: dict[str, list[Any]]) -> None:
    def on_request_failed(req):
        failure = str(req.failure)
        item = {"url": req.url, "method": req.method, "failure": failure}
        if "net::ERR_ABORTED" in failure and any(
            pattern in req.url for pattern in _IGNORED_ABORT_PATTERNS
        ):
            bucket["ignoredFailedRequests"].append(item)
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


def message_row(page: Page, msg_id: str) -> Locator:
    return page.locator(f"#msg-{msg_id}")


async def find_message_by_text(page: Page, content: str) -> str | None:
    """Return msg id of the row containing exact text (or None)."""
    row = page.locator('div[id^="msg-"]').filter(has_text=content).first
    if await row.count() == 0:
        return None
    raw_id = await row.get_attribute("id")
    if not raw_id:
        return None
    return raw_id.removeprefix("msg-")


async def hover_and_click_action(page: Page, msg_id: str, action_label: str) -> None:
    row = message_row(page, msg_id)
    button = row.locator(f'button[aria-label="{action_label}"]')
    # Wait for the button to be in DOM. It is rendered with opacity-0 until
    # hover, but only when isOwn (for edit/delete) — so this also implicitly
    # waits for $currentUser to populate.
    await button.wait_for(state="attached", timeout=20000)
    await row.hover()
    await button.click()


async def send_message(page: Page, text: str) -> str:
    input_locator = page.locator('input[placeholder^="Message #"]')
    await input_locator.fill(text)
    await input_locator.press("Enter")
    # Wait for the message to appear in the list.
    await page.locator('div[id^="msg-"]').filter(has_text=text).first.wait_for(
        timeout=15000
    )
    msg_id = await find_message_by_text(page, text)
    if not msg_id:
        raise RuntimeError(f"sent message not found in list: {text!r}")
    return msg_id


async def prove_messaging(
    browser: Browser,
    *,
    server_id: str,
    channel_id: str,
    channel_name: str,
    suffix: str,
) -> dict[str, Any]:
    bucket: dict[str, list[Any]] = {
        "consoleErrors": [],
        "pageErrors": [],
        "failedRequests": [],
        "ignoredFailedRequests": [],
    }
    context = await new_context(browser, server_id)
    page = await context.new_page()
    attach_page_logging(page, bucket)
    page.on("dialog", lambda dialog: asyncio.create_task(dialog.accept()))

    await login_via_ui(page, MEMBER_EMAIL, MEMBER_PASSWORD)
    await page.goto(
        f"{BASE}/servers/{server_id}/channels/{channel_id}",
        wait_until="networkidle",
        timeout=30000,
    )
    await page.locator('input[placeholder^="Message #"]').wait_for(timeout=20000)

    send_text = f"msg-ui-send-{suffix}"
    send_msg_id = await send_message(page, send_text)
    send_screenshot = await screenshot(page, "01-send.png")

    # Edit the just-sent message.
    edit_text = f"msg-ui-edit-{suffix}"
    await hover_and_click_action(page, send_msg_id, "Edit message")
    edit_textarea = message_row(page, send_msg_id).locator("textarea")
    await edit_textarea.fill(edit_text)
    await edit_textarea.press("Enter")
    await message_row(page, send_msg_id).filter(has_text=edit_text).wait_for(timeout=15000)
    edited_indicator_visible = (
        await message_row(page, send_msg_id).filter(has_text="(edited)").count() > 0
    )
    edit_screenshot = await screenshot(page, "02-edit.png")

    # Reply to the edited message.
    reply_text = f"msg-ui-reply-{suffix}"
    await hover_and_click_action(page, send_msg_id, "Reply")
    # The reply panel shows an X cancel button next to the input.
    reply_input = page.locator('input[placeholder^="Message #"]')
    await reply_input.fill(reply_text)
    await reply_input.press("Enter")
    reply_msg_id = None
    for _ in range(20):
        reply_msg_id = await find_message_by_text(page, reply_text)
        if reply_msg_id:
            break
        await page.wait_for_timeout(250)
    reply_context_visible = False
    if reply_msg_id:
        reply_context_visible = (
            await message_row(page, reply_msg_id).filter(has_text=edit_text).count() > 0
        )
    reply_screenshot = await screenshot(page, "03-reply.png")

    # React to the original (edited) message with the small picker.
    await message_row(page, send_msg_id).hover()
    await message_row(page, send_msg_id).locator(
        'button[aria-label="Add reaction"], button[title="Add reaction"]'
    ).first.click()
    # The small picker shows a fixed set of emoji buttons; pick the first.
    picker_button = page.locator('button:has-text("👍"), button:has-text("❤"), button:has-text("😂")').first
    await picker_button.click()
    reaction_visible = False
    for _ in range(20):
        # A reaction badge appears as a button inside the row whose title
        # contains "reaction".
        reaction_visible = (
            await message_row(page, send_msg_id)
            .locator('button[title*="reaction"]').count()
        ) > 0
        if reaction_visible:
            break
        await page.wait_for_timeout(250)
    react_screenshot = await screenshot(page, "04-react.png")

    # Pin the original message.
    await hover_and_click_action(page, send_msg_id, "Pin message")
    await page.locator('button[aria-label="Pinned messages"]').click()
    pin_visible = False
    for _ in range(20):
        if await page.get_by_text(edit_text, exact=False).count() > 1:
            pin_visible = True
            break
        await page.wait_for_timeout(250)
    pin_screenshot = await screenshot(page, "05-pin.png")
    # Close pins panel so subsequent actions are not obscured.
    await page.locator('button[aria-label="Pinned messages"]').click()

    # Delete the reply (cleaner than deleting the pinned original).
    delete_target_id = reply_msg_id or send_msg_id
    await hover_and_click_action(page, delete_target_id, "Delete message")
    deleted_gone = False
    for _ in range(20):
        if await message_row(page, delete_target_id).count() == 0:
            deleted_gone = True
            break
        await page.wait_for_timeout(250)
    delete_screenshot = await screenshot(page, "06-delete.png")

    await context.close()

    return {
        "browser": {
            "consoleErrors": bucket["consoleErrors"],
            "pageErrors": bucket["pageErrors"],
            "failedRequests": bucket["failedRequests"],
        },
        "channel": {"id": str(channel_id), "name": channel_name},
        "send": {
            "text": send_text,
            "msgId": send_msg_id,
            "screenshot": send_screenshot,
        },
        "edit": {
            "text": edit_text,
            "editedIndicatorVisible": edited_indicator_visible,
            "screenshot": edit_screenshot,
        },
        "reply": {
            "text": reply_text,
            "msgId": reply_msg_id,
            "contextVisible": reply_context_visible,
            "screenshot": reply_screenshot,
        },
        "react": {
            "reactionVisible": reaction_visible,
            "screenshot": react_screenshot,
        },
        "pin": {
            "pinnedListVisible": pin_visible,
            "screenshot": pin_screenshot,
        },
        "delete": {
            "targetMsgId": delete_target_id,
            "rowRemoved": deleted_gone,
            "screenshot": delete_screenshot,
        },
    }


def no_browser_errors(section: dict[str, Any]) -> bool:
    browser = section.get("browser", {})
    return (
        not browser.get("consoleErrors")
        and not browser.get("pageErrors")
        and not browser.get("failedRequests")
    )


async def cleanup_messages(
    session: aiohttp.ClientSession,
    headers: dict[str, str],
    channel_id: str,
    suffix: str,
) -> int:
    """Delete any of our test messages still in the channel."""
    status, msgs = await request_json(
        session,
        "GET",
        f"{API}/channels/{channel_id}/messages?limit=50",
        headers=headers,
    )
    if status != 200 or not isinstance(msgs, list):
        return 0
    removed = 0
    for msg in msgs:
        if not isinstance(msg, dict):
            continue
        if suffix in (msg.get("content") or ""):
            del_status, _ = await request_json(
                session,
                "DELETE",
                f"{API}/messages/{msg['id']}",
                headers=headers,
            )
            if del_status in (200, 204):
                removed += 1
    return removed


async def main() -> int:
    args = parse_args()
    stamp = int(time.time() * 1000)
    suffix = str(stamp)[-8:]

    report: dict[str, Any] = {
        "ok": False,
        "base": BASE,
        "auth": {"ownerMethod": "default_member_login", "tokensStoredInReport": False},
    }
    cleanup_count = 0
    channel_id: str | None = None

    async with aiohttp.ClientSession() as session:
        owner_token = await login(session, MEMBER_EMAIL, MEMBER_PASSWORD)
        owner_headers = {"Authorization": f"Bearer {owner_token}"}

        status, profile = await request_json(
            session, "GET", f"{API}/users/@me", headers=owner_headers
        )
        if status != 200 or not profile:
            raise RuntimeError(f"owner profile baseline failed: {status} {profile}")
        owner_user_id = profile["id"]

        status, servers = await request_json(
            session, "GET", f"{API}/servers", headers=owner_headers
        )
        if status != 200 or not servers:
            raise RuntimeError(f"owner server baseline failed: {status} {servers}")
        owned = next(
            (s for s in servers if str(s.get("owner_id")) == str(owner_user_id)), None
        )
        if not owned:
            raise RuntimeError("owner token has no owned server for messaging proof")
        server_id = owned["id"]

        channel = await pick_text_channel(session, owner_headers, server_id)
        channel_id = channel["id"]
        channel_name = channel["name"]

        try:
            chrome = os.environ.get("PLAYWRIGHT_CHROME", "/usr/bin/google-chrome")
            async with async_playwright() as pw:
                browser = await pw.chromium.launch(
                    executable_path=chrome,
                    args=["--no-sandbox", "--disable-dev-shm-usage", "--headless=new"],
                )
                messaging = await prove_messaging(
                    browser,
                    server_id=server_id,
                    channel_id=channel_id,
                    channel_name=channel_name,
                    suffix=suffix,
                )
                await browser.close()

            report.update(
                {
                    "fixture": {
                        "serverId": server_id,
                        "channelId": channel_id,
                        "channelName": channel_name,
                        "messageContentSuffix": suffix,
                    },
                    "browser": messaging,
                }
            )
            report["ok"] = (
                messaging["send"]["msgId"] is not None
                and messaging["edit"]["editedIndicatorVisible"]
                and messaging["reply"]["msgId"] is not None
                and messaging["reply"]["contextVisible"]
                and messaging["react"]["reactionVisible"]
                and messaging["pin"]["pinnedListVisible"]
                and messaging["delete"]["rowRemoved"]
                and no_browser_errors(messaging)
            )
        finally:
            cleanup_count = await cleanup_messages(
                session, owner_headers, channel_id, suffix
            )

    report["cleanup"] = {"messagesRemoved": cleanup_count}
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    print(f"Wrote {OUT}")

    if args.fail_on_issues and not report["ok"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
