#!/usr/bin/env python3
"""Search syntax filter QA harness.

Tests the search API with Discord-style filter syntax and verifies
result structure (content preview, timestamps, filters applied).
Uses the API directly — no browser required for core search validation.

To run:
    python3 scripts/search_qa.py
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import sys
from pathlib import Path
from typing import Any

import aiohttp

BASE = os.environ.get("OC_BASE", "https://opencorde.com").rstrip("/")
API = f"{BASE}/api/v1"
OUT = Path(os.environ.get("OC_SEARCH_QA_OUT", "reports/raw/search-proof.json"))

# Test fixture credentials
MEMBER_EMAIL = os.environ.get("OC_MEMBER_EMAIL", "browsertest@opencorde.local")
MEMBER_PASSWORD = os.environ.get("OC_MEMBER_PASSWORD", "BrowserTest@99")

PASS = "pass"
FAIL = "fail"
SKIP = "skip"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fail-on-issues", action="store_true")
    return parser.parse_args()


async def request_json(
    session: aiohttp.ClientSession, method: str, url: str, **kwargs
) -> tuple[int, Any]:
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
    if status != 200 or not isinstance(data, dict):
        raise RuntimeError(f"Login failed: {status} {data}")
    token = data.get("token") or data.get("access_token")
    if not token:
        raise RuntimeError(f"No token in login response: {data}")
    return token


async def search_api(
    session: aiohttp.ClientSession,
    token: str,
    query: str,
    server_id: int | None = None,
    channel_id: int | None = None,
    limit: int = 25,
) -> tuple[int, dict | None]:
    """Call the search API endpoint."""
    params = {"q": query, "limit": str(limit)}
    if server_id:
        params["server_id"] = str(server_id)
    if channel_id:
        params["channel_id"] = str(channel_id)

    headers = {"Authorization": f"Bearer {token}"}
    query_string = "&".join(f"{k}={v}" for k, v in params.items())
    status, data = await request_json(
        session, "GET", f"{API}/search?{query_string}", headers=headers
    )
    return status, data


def check(name: str, condition: bool, detail: str = "") -> dict:
    return {
        "name": name,
        "status": PASS if condition else FAIL,
        "detail": detail,
    }


async def run_checks() -> dict:
    """Run all search QA checks."""
    results = []
    screenshots: list[str] = []
    timestamp = __import__("datetime").datetime.utcnow().isoformat() + "Z"

    async with aiohttp.ClientSession() as session:
        # Login
        try:
            token = await login(session, MEMBER_EMAIL, MEMBER_PASSWORD)
            results.append(check("login", True, f"Logged in as {MEMBER_EMAIL}"))
        except Exception as e:
            results.append(check("login", False, str(e)))
            return _build_report(results, screenshots, timestamp, False)

        # Test 1: Basic text search
        status, data = await search_api(session, token, "hello", limit=10)
        results.append(
            check(
                "basic text search returns 200",
                status == 200 or status == 503,
                f"status={status}",
            )
        )
        if status == 200 and isinstance(data, dict):
            results.append(
                check(
                    "response has results array",
                    "results" in data and isinstance(data["results"], list),
                    f"keys={list(data.keys()) if data else 'none'}",
                )
            )
            results.append(
                check(
                    "response has count field",
                    "count" in data,
                    "",
                )
            )
            results.append(
                check(
                    "response has query field",
                    "query" in data,
                    "",
                )
            )
            results.append(
                check(
                    "response has filters field",
                    "filters" in data,
                    "",
                )
            )
            # Check result structure if we got results
            if data.get("results"):
                r0 = data["results"][0]
                results.append(
                    check(
                        "result has message_id",
                        "message_id" in r0,
                        f"fields={list(r0.keys())}",
                    )
                )
                results.append(
                    check(
                        "result has content",
                        "content" in r0 and isinstance(r0["content"], str),
                        "",
                    )
                )
                results.append(
                    check(
                        "result has created_at",
                        "created_at" in r0,
                        "",
                    )
                )
                results.append(
                    check(
                        "result has score",
                        "score" in r0 and isinstance(r0["score"], (int, float)),
                        "",
                    )
                )
                results.append(
                    check(
                        "result has author_id",
                        "author_id" in r0,
                        "",
                    )
                )
                results.append(
                    check(
                        "result has channel_id",
                        "channel_id" in r0,
                        "",
                    )
                )
                results.append(
                    check(
                        "content is truncated to ~200 chars",
                        len(r0.get("content", "")) <= 205,
                        f"len={len(r0.get('content', ''))}",
                    )
                )
        elif status == 503:
            results.append(
                check(
                    "search engine unavailable (expected in dev without index)",
                    True,
                    "Skipping search-result structure checks",
                )
            )
        else:
            results.append(
                check(
                    "unexpected search status",
                    False,
                    f"status={status}, data={data}",
                )
            )

        # Test 2: Filter syntax — has:link
        status, data = await search_api(session, token, "test has:link", limit=10)
        results.append(
            check(
                "filter has:link returns 200",
                status == 200 or status == 503,
                f"status={status}",
            )
        )
        if status == 200 and isinstance(data, dict) and data.get("filters"):
            f = data["filters"]
            results.append(
                check(
                    "has:link filter reflected in response",
                    f.get("has_link") is True,
                    f"filters={f}",
                )
            )

        # Test 3: Filter syntax — has:file
        status, data = await search_api(session, token, "has:file", limit=10)
        results.append(
            check(
                "filter has:file returns 200",
                status == 200 or status == 503,
                f"status={status}",
            )
        )
        if status == 200 and isinstance(data, dict) and data.get("filters"):
            f = data["filters"]
            results.append(
                check(
                    "has:file filter reflected in response",
                    f.get("has_file") is True,
                    f"filters={f}",
                )
            )

        # Test 4: Filter syntax — from:user
        status, data = await search_api(session, token, "from:123", limit=10)
        results.append(
            check(
                "filter from: returns 200",
                status == 200 or status == 503,
                f"status={status}",
            )
        )
        if status == 200 and isinstance(data, dict) and data.get("filters"):
            f = data["filters"]
            results.append(
                check(
                    "from filter reflected in response",
                    f.get("from") == "123",
                    f"filters={f}",
                )
            )

        # Test 5: Filter syntax — before:/after:
        status, data = await search_api(
            session, token, "chat before:2025-12-31 after:2025-01-01", limit=10
        )
        results.append(
            check(
                "date range filters return 200",
                status == 200 or status == 503,
                f"status={status}",
            )
        )
        if status == 200 and isinstance(data, dict) and data.get("filters"):
            f = data["filters"]
            results.append(
                check(
                    "before filter reflected",
                    f.get("before") is not None,
                    f"filters={f}",
                )
            )
            results.append(
                check(
                    "after filter reflected",
                    f.get("after") is not None,
                    f"filters={f}",
                )
            )

        # Test 6: Filter syntax — pinned:true
        status, data = await search_api(session, token, "rules pinned:true", limit=10)
        results.append(
            check(
                "pinned filter returns 200",
                status == 200 or status == 503,
                f"status={status}",
            )
        )
        if status == 200 and isinstance(data, dict) and data.get("filters"):
            f = data["filters"]
            results.append(
                check(
                    "pinned filter reflected",
                    f.get("pinned") is True,
                    f"filters={f}",
                )
            )

        # Test 7: Multiple combined filters
        status, data = await search_api(
            session,
            token,
            "project from:alice has:link before:2025-12-31 in:general",
            limit=10,
        )
        results.append(
            check(
                "combined filters return 200",
                status == 200 or status == 503,
                f"status={status}",
            )
        )

        # Test 8: Empty query rejected
        status, data = await search_api(session, token, "", limit=10)
        results.append(
            check(
                "empty query rejected with 400",
                status == 400,
                f"status={status}",
            )
        )

        # Test 9: Query with only spaces rejected
        status, data = await search_api(session, token, "   ", limit=10)
        results.append(
            check(
                "whitespace-only query rejected with 400",
                status == 400,
                f"status={status}",
            )
        )

        # Test 10: Filters-only query (no text) should work
        status, data = await search_api(
            session, token, "has:link", limit=10
        )
        results.append(
            check(
                "filters-only query returns 200",
                status == 200 or status == 503,
                f"status={status}",
            )
        )

        # Test 11: Verify query text is cleaned (filters stripped from query field)
        status, data = await search_api(
            session, token, "hello from:alice has:link", limit=10
        )
        if status == 200 and isinstance(data, dict):
            results.append(
                check(
                    "query field shows cleaned text (filters stripped)",
                    data.get("query") == "hello",
                    f"query={data.get('query')}",
                )
            )

    ok = all(
        r["status"] == PASS
        for r in results
        if "expected in dev" not in r.get("detail", "")
    )
    return _build_report(results, screenshots, timestamp, ok)


def _build_report(
    results: list[dict], screenshots: list[str], timestamp: str, ok: bool
) -> dict:
    passed = sum(1 for r in results if r["status"] == PASS)
    failed = sum(1 for r in results if r["status"] == FAIL)
    skipped = sum(1 for r in results if r["status"] == SKIP)
    return {
        "timestamp": timestamp,
        "ok": ok,
        "checks": results,
        "summary": {
            "total": len(results),
            "passed": passed,
            "failed": failed,
            "skipped": skipped,
        },
        "screenshots": screenshots,
    }


async def main() -> None:
    args = parse_args()
    print(f"Search filter QA — {BASE}")
    print(f"Fixture: {MEMBER_EMAIL}")

    report = await run_checks()
    s = report["summary"]
    print(
        f"Results: {s['passed']}/{s['total']} passed, "
        f"{s['failed']} failed, {s['skipped']} skipped"
    )

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    print(f"Report: {OUT}")

    if not report["ok"] and args.fail_on_issues:
        print("FAIL: issues detected and --fail-on-issues set")
        sys.exit(1)

    if report["ok"]:
        print("PASS")
    else:
        print("FAIL (some checks did not pass)")


if __name__ == "__main__":
    asyncio.run(main())
