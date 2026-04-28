#!/usr/bin/env python3
"""Permission smoke tests against a running OpenCorde API."""

from __future__ import annotations

import asyncio
import json
import os
import re
import time
from pathlib import Path
from urllib.parse import quote

import aiohttp


BASE = os.environ.get("OC_BASE", "https://opencorde.com").rstrip("/")
API = f"{BASE}/api/v1"
OUT = Path(os.environ.get("OC_PERMISSION_QA_OUT", "reports/raw/permission-smoke.json"))

MEMBER_EMAIL = os.environ.get("OC_MEMBER_EMAIL", "browsertest@opencorde.local")
MEMBER_PASSWORD = os.environ.get("OC_MEMBER_PASSWORD", "BrowserTest@99")
NONMEMBER_USERNAME = os.environ.get("OC_NONMEMBER_USERNAME", "permission_nonmember")
NONMEMBER_EMAIL = os.environ.get("OC_NONMEMBER_EMAIL", "permission-nonmember@opencorde.local")
NONMEMBER_PASSWORD = os.environ.get("OC_NONMEMBER_PASSWORD", "PermissionSmoke@99")


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


async def ensure_nonmember(session: aiohttp.ClientSession) -> str:
    status, _data = await request_json(
        session,
        "POST",
        f"{API}/auth/register",
        json={
            "username": NONMEMBER_USERNAME,
            "email": NONMEMBER_EMAIL,
            "password": NONMEMBER_PASSWORD,
        },
    )
    if status not in (200, 201, 409):
        raise RuntimeError(f"nonmember register failed: {status}")
    return await login(session, NONMEMBER_EMAIL, NONMEMBER_PASSWORD)


async def main() -> int:
    results: list[dict] = []

    async with aiohttp.ClientSession() as session:
        member_token = await login(session, MEMBER_EMAIL, MEMBER_PASSWORD)
        nonmember_token = await ensure_nonmember(session)

        member_headers = {"Authorization": f"Bearer {member_token}"}
        nonmember_headers = {"Authorization": f"Bearer {nonmember_token}"}

        status, servers = await request_json(session, "GET", f"{API}/servers", headers=member_headers)
        if status != 200 or not servers:
            raise RuntimeError(f"member has no server baseline: {status} {servers}")
        server_id = servers[0]["id"]

        status, channels = await request_json(
            session,
            "GET",
            f"{API}/servers/{server_id}/channels",
            headers=member_headers,
        )
        channel_id = None
        message_id = None
        search_term = None
        if status == 200 and channels:
            channel_id = channels[0]["id"]
            status, messages = await request_json(
                session,
                "GET",
                f"{API}/channels/{channel_id}/messages",
                headers=member_headers,
            )
            if status == 200 and messages:
                message_id = messages[0]["id"]
                words = re.findall(r"[A-Za-z0-9]{2,}", messages[0].get("content") or "")
                search_term = words[0] if words else None

        if channel_id:
            search_term = f"permsearch{int(time.time())}"
            status, message = await request_json(
                session,
                "POST",
                f"{API}/channels/{channel_id}/messages",
                headers=member_headers,
                json={"content": f"{search_term} permission smoke baseline"},
            )
            if status in (200, 201) and isinstance(message, dict):
                message_id = message.get("id", message_id)
            else:
                search_term = None

        checks = [
            {
                "name": "nonmember cannot create server invite",
                "method": "POST",
                "url": f"{API}/servers/{server_id}/invites",
                "json": {},
                "expect": 403,
            },
            {
                "name": "nonmember cannot list server invites",
                "method": "GET",
                "url": f"{API}/servers/{server_id}/invites",
                "expect": 403,
            },
            {
                "name": "nonmember cannot read onboarding",
                "method": "GET",
                "url": f"{API}/servers/{server_id}/onboarding",
                "expect": 403,
            },
        ]

        if channel_id:
            checks.extend(
                [
                    {
                        "name": "nonmember cannot list channel threads",
                        "method": "GET",
                        "url": f"{API}/channels/{channel_id}/threads",
                        "expect": 403,
                    },
                    {
                        "name": "nonmember cannot list channel webhooks",
                        "method": "GET",
                        "url": f"{API}/channels/{channel_id}/webhooks",
                        "expect": 403,
                    },
                    {
                        "name": "nonmember cannot list channel pins",
                        "method": "GET",
                        "url": f"{API}/channels/{channel_id}/pins",
                        "expect": 403,
                    },
                ]
            )

        if channel_id and message_id:
            checks.extend(
                [
                    {
                        "name": "nonmember cannot pin channel message",
                        "method": "PUT",
                        "url": f"{API}/channels/{channel_id}/pins/{message_id}",
                        "expect": 403,
                    },
                    {
                        "name": "nonmember cannot list message reactions",
                        "method": "GET",
                        "url": f"{API}/messages/{message_id}/reactions",
                        "expect": 403,
                    },
                ]
            )

        if search_term:
            query = quote(search_term)
            member_search = None
            for _ in range(5):
                status, member_search = await request_json(
                    session,
                    "GET",
                    f"{API}/search?q={query}&server_id={server_id}&limit=10",
                    headers=member_headers,
                )
                if status == 200 and member_search and member_search.get("count", 0) > 0:
                    break
                await asyncio.sleep(0.5)
            if status == 200 and member_search and member_search.get("count", 0) > 0:
                checks.append(
                    {
                        "name": "nonmember search cannot see private server messages",
                        "method": "GET",
                        "url": f"{API}/search?q={query}&server_id={server_id}&limit=10",
                        "expect": 200,
                        "expect_empty_results": True,
                    }
                )

        for check in checks:
            kwargs = {"headers": nonmember_headers}
            if "json" in check:
                kwargs["json"] = check["json"]
            status, data = await request_json(session, check["method"], check["url"], **kwargs)
            ok = status == check["expect"]
            if ok and check.get("expect_empty_results"):
                ok = isinstance(data, dict) and data.get("count") == 0 and data.get("results") == []
            result = {
                "name": check["name"],
                "method": check["method"],
                "url": check["url"].replace(API, "/api/v1"),
                "expectedStatus": check["expect"],
                "actualStatus": status,
                "ok": ok,
                "response": data,
            }
            results.append(result)
            print(f"[{'PASS' if ok else 'FAIL'}] {check['name']} expected={check['expect']} actual={status}")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {OUT}")
    return 0 if all(item["ok"] for item in results) else 1


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
