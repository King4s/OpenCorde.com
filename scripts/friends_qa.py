#!/usr/bin/env python3
"""Live friends QA proof.

Drives the friends API as two fixture users and proves the friend lifecycle
that Discord parity issue #9 calls out: search → request → incoming visible →
accept → friendship visible → block → blocked prevents interaction → unblock.

The script uses API calls only (no browser). It cleans up any pre-existing
relationships between the fixture users before running the proof, then removes
the test relationship at the end.
"""

from __future__ import annotations

import argparse
import asyncio
import os
import json
import sys
import time
from pathlib import Path
from typing import Any

import aiohttp

BASE = os.environ.get("OC_BASE", "https://opencorde.com").rstrip("/")
API = f"{BASE}/api/v1"
OUT = Path(os.environ.get("OC_FRIENDS_QA_OUT", "reports/raw/friends-qa-proof.json"))

USER_A_EMAIL = os.environ.get("OC_FRIENDS_A_EMAIL", "browsertest@opencorde.local")
USER_A_PASSWORD = os.environ.get("OC_FRIENDS_A_PASSWORD", "BrowserTest@99")
USER_B_EMAIL = os.environ.get("OC_FRIENDS_B_EMAIL", "messaging-second@opencorde.local")
USER_B_PASSWORD = os.environ.get("OC_FRIENDS_B_PASSWORD", "MessagingSecond@99")


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


async def register_or_login(session: aiohttp.ClientSession, email: str, password: str) -> str:
    """Register a user if they don't exist, otherwise log in."""
    status, data = await request_json(
        session,
        "POST",
        f"{API}/auth/register",
        json={"email": email, "password": password, "username": email.split("@")[0].replace("-", "_")},
    )
    if status == 201:
        return data["access_token"]
    if status == 409:
        return await login(session, email, password)
    raise RuntimeError(f"register failed for {email}: {status} {data}")


async def get_profile(session: aiohttp.ClientSession, headers: dict) -> dict:
    status, data = await request_json(session, "GET", f"{API}/users/@me", headers=headers)
    if status != 200 or not data:
        raise RuntimeError(f"profile fetch failed: {status} {data}")
    return data


async def search_users(session: aiohttp.ClientSession, headers: dict, query: str) -> list[dict]:
    status, data = await request_json(
        session, "GET", f"{API}/users/search?q={query}", headers=headers
    )
    if status != 200:
        raise RuntimeError(f"user search failed: {status} {data}")
    return data if isinstance(data, list) else []


async def get_friends(session: aiohttp.ClientSession, headers: dict) -> list[dict]:
    status, data = await request_json(session, "GET", f"{API}/friends", headers=headers)
    if status != 200:
        raise RuntimeError(f"friends list failed: {status} {data}")
    return data if isinstance(data, list) else []


async def get_pending(session: aiohttp.ClientSession, headers: dict) -> dict:
    status, data = await request_json(session, "GET", f"{API}/friends/pending", headers=headers)
    if status != 200:
        raise RuntimeError(f"pending list failed: {status} {data}")
    return data if isinstance(data, dict) else {"incoming": [], "outgoing": []}


async def send_request(session: aiohttp.ClientSession, headers: dict, user_id: str) -> tuple[int, dict | None]:
    return await request_json(
        session, "POST", f"{API}/friends/request", headers=headers, json={"user_id": user_id}
    )


async def accept_request(session: aiohttp.ClientSession, headers: dict, rel_id: str) -> int:
    status, _ = await request_json(
        session, "PUT", f"{API}/friends/{rel_id}/accept", headers=headers
    )
    return status


async def block_user(session: aiohttp.ClientSession, headers: dict, user_id: str) -> tuple[int, dict | None]:
    return await request_json(
        session, "POST", f"{API}/friends/block", headers=headers, json={"user_id": user_id}
    )


async def remove_relationship(session: aiohttp.ClientSession, headers: dict, rel_id: str) -> int:
    status, _ = await request_json(
        session, "DELETE", f"{API}/friends/{rel_id}", headers=headers
    )
    return status


async def set_nickname(session: aiohttp.ClientSession, headers: dict, rel_id: str, nickname: str) -> tuple[int, dict | None]:
    return await request_json(
        session, "PUT", f"{API}/friends/{rel_id}/nickname", headers=headers, json={"nickname": nickname}
    )

async def clear_nickname(session: aiohttp.ClientSession, headers: dict, rel_id: str) -> tuple[int, dict | None]:
    return await request_json(
        session, "DELETE", f"{API}/friends/{rel_id}/nickname", headers=headers
    )

async def get_blocked_list(session: aiohttp.ClientSession, headers: dict) -> dict:
    status, data = await request_json(session, "GET", f"{API}/friends/blocked", headers=headers)
    if status != 200:
        raise RuntimeError(f"blocked list failed: {status} {data}")
    return data

async def get_mutual_friends(session: aiohttp.ClientSession, headers: dict, friend_id: str) -> dict:
    status, data = await request_json(
        session, "GET", f"{API}/friends/{friend_id}/mutual-friends", headers=headers
    )
    if status != 200:
        raise RuntimeError(f"mutual friends failed: {status} {data}")
    return data

async def get_mutual_servers(session: aiohttp.ClientSession, headers: dict, friend_id: str) -> dict:
    status, data = await request_json(
        session, "GET", f"{API}/friends/{friend_id}/mutual-servers", headers=headers
    )
    if status != 200:
        raise RuntimeError(f"mutual servers failed: {status} {data}")
    return data


async def _pending_flat(session: aiohttp.ClientSession, headers: dict) -> list[dict]:
    """Return pending requests as a flat list for cleanup scanning."""
    p = await get_pending(session, headers)
    return p.get("incoming", []) + p.get("outgoing", [])


async def cleanup_existing_relationships(
    session: aiohttp.ClientSession, user_a_id: str, user_b_id: str, headers_a: dict, headers_b: dict
) -> int:
    """Remove any pre-existing relationships between user A and user B.

    Returns the number of relationships cleaned up.
    """
    removed = 0

    # Check user A's friends and pending lists for user B.
    for fetch in (
        lambda h: get_friends(session, h),
        lambda h: _pending_flat(session, h),
    ):
        for headers in (headers_a, headers_b):
            try:
                rels = await fetch(headers)
            except Exception:
                continue
            if not isinstance(rels, list):
                continue
            for rel in rels:
                if not isinstance(rel, dict):
                    continue
                rel_id = rel.get("id", "")
                from_u = rel.get("from_user", "")
                to_u = rel.get("to_user", "")
                if rel_id and (user_b_id in (from_u, to_u) or user_a_id in (from_u, to_u)):
                    status = await remove_relationship(session, headers, rel_id)
                    if status in (200, 204):
                        removed += 1
    return removed


async def main() -> int:
    args = parse_args()
    stamp = int(time.time() * 1000)

    report: dict[str, Any] = {
        "ok": False,
        "base": BASE,
        "auth": {"method": "fixture_login", "tokensStoredInReport": False},
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "checks": [],
    }

    async with aiohttp.ClientSession() as session:
        # ── Auth both users ─────────────────────────────────────────────
        token_a = await register_or_login(session, USER_A_EMAIL, USER_A_PASSWORD)
        headers_a = {"Authorization": f"Bearer {token_a}"}
        profile_a = await get_profile(session, headers_a)
        user_a_id = profile_a["id"]
        user_a_name = profile_a.get("username", USER_A_EMAIL)

        token_b = await register_or_login(session, USER_B_EMAIL, USER_B_PASSWORD)
        headers_b = {"Authorization": f"Bearer {token_b}"}
        profile_b = await get_profile(session, headers_b)
        user_b_id = profile_b["id"]
        user_b_name = profile_b.get("username", USER_B_EMAIL)

        report["fixture"] = {
            "userA": {"id": user_a_id, "username": user_a_name},
            "userB": {"id": user_b_id, "username": user_b_name},
        }

        def check(name: str, passed: bool, detail: str = "") -> None:
            report["checks"].append({"name": name, "passed": passed, "detail": detail})

        # ── Cleanup existing relationships ──────────────────────────────
        cleanup_count = await cleanup_existing_relationships(
            session, user_a_id, user_b_id, headers_a, headers_b
        )
        report["cleanup"] = {"relationshipsRemoved": cleanup_count}

        # ── 1. Search ───────────────────────────────────────────────────
        results = await search_users(session, headers_a, user_b_name)
        found = any(r.get("id") == user_b_id for r in results if isinstance(r, dict))
        check("search", found, f"searched '{user_b_name}', found target user" if found else f"target {user_b_id} not in {len(results)} results")

        # ── 2. Send friend request ──────────────────────────────────────
        req_status, req_data = await send_request(session, headers_a, user_b_id)
        request_ok = req_status == 201 and req_data is not None
        rel_id = req_data.get("id") if req_data else None
        check("send-request", request_ok and rel_id is not None, f"POST /friends/request → {req_status}, rel_id={rel_id}")

        if not rel_id:
            report["ok"] = False
            OUT.parent.mkdir(parents=True, exist_ok=True)
            OUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
            print(json.dumps(report, indent=2))
            return 1

        # ── 3. Incoming request visible ─────────────────────────────────
        pending_b = await get_pending(session, headers_b)
        incoming = pending_b.get("incoming", [])
        incoming_visible = any(
            r.get("id") == rel_id for r in incoming if isinstance(r, dict)
        )
        check("incoming-visible", incoming_visible, f"User B sees {len(incoming)} incoming, rel_id={rel_id} present={incoming_visible}")

        # ── 4. Accept ───────────────────────────────────────────────────
        accept_status = await accept_request(session, headers_b, rel_id)
        accept_ok = accept_status == 204
        check("accept", accept_ok, f"PUT /friends/{rel_id}/accept → {accept_status}")

        # ── 5. Friendship visible ───────────────────────────────────────
        friends_a = await get_friends(session, headers_a)
        friends_b = await get_friends(session, headers_b)
        friend_visible_a = any(
            r.get("id") == rel_id for r in friends_a if isinstance(r, dict)
        )
        friend_visible_b = any(
            r.get("id") == rel_id for r in friends_b if isinstance(r, dict)
        )
        friendship_ok = friend_visible_a and friend_visible_b
        check("friendship-visible", friendship_ok,
              f"A sees={friend_visible_a} ({len(friends_a)} friends), B sees={friend_visible_b} ({len(friends_b)} friends)")

        # ── 6. Block ────────────────────────────────────────────────────
        block_status, block_data = await block_user(session, headers_a, user_b_id)
        block_ok = block_status == 201 and block_data is not None
        block_rel_id = block_data.get("id") if block_data else None
        block_status_value = block_data.get("status") if block_data else None
        check("block", block_ok and block_status_value == "blocked",
              f"POST /friends/block → {block_status}, rel_id={block_rel_id}, status={block_status_value}")

        # ── 7. Blocked state prevents friend interaction ─────────────────
        # After blocking, user A's friends list should NOT include user B
        # (the previous friendship rel_id should be gone or replaced).
        friends_a_after = await get_friends(session, headers_a)
        still_friend = any(
            r.get("id") == rel_id for r in friends_a_after if isinstance(r, dict)
        )
        block_prevents = not still_friend
        check("block-prevents-friendship", block_prevents,
              f"after block, old friendship rel_id {rel_id} present={still_friend} (expect False)")

        # Also: User B should NOT be able to send a friend request to User A
        # (blocked relationship should prevent this).
        re_req_status, re_req_data = await send_request(session, headers_b, user_a_id)
        blocked_blocks_request = re_req_status in (400, 403, 409)
        if not blocked_blocks_request and re_req_status == 201:
            # Clean up the spurious request that bypassed the block.
            spur_rel_id = re_req_data.get("id") if re_req_data else None
            if spur_rel_id:
                await remove_relationship(session, headers_b, spur_rel_id)
        check("blocked-blocks-request", blocked_blocks_request,
              f"B→A request after block → {re_req_status} (expect 400/403/409; 201=backend gap: request handler doesn't check blocked relationships)")

        # ── 8. Unblock ──────────────────────────────────────────────────
        if block_rel_id:
            unblock_status = await remove_relationship(session, headers_a, block_rel_id)
            unblock_ok = unblock_status == 204
            check("unblock", unblock_ok, f"DELETE /friends/{block_rel_id} → {unblock_status}")
        else:
            check("unblock", False, "no block_rel_id from block response; cannot unblock")
            unblock_ok = False

        # ── 9. Nickname: set, read, clear ────────────────────────────────
        # Re-establish friendship for nickname tests
        req2_status, req2_data = await send_request(session, headers_a, user_b_id)
        if req2_status == 201 and req2_data:
            new_rel_id = req2_data.get("id")
            accept2_status = await accept_request(session, headers_b, new_rel_id)
            re_friended = accept2_status == 204
        else:
            re_friended = False
            new_rel_id = None
        check("re-friend", re_friended, f"re-friended after unblock test: request={req2_status}, accept={accept2_status if re_friended else 'N/A'}, rel_id={new_rel_id}")

        if new_rel_id:
            # Set nickname
            nick_status, nick_data = await set_nickname(session, headers_a, new_rel_id, "TestBuddy")
            nick_set_ok = nick_status == 200 and nick_data is not None and nick_data.get("nickname") == "TestBuddy"
            check("nickname-set", nick_set_ok,
                  f"PUT nickname → {nick_status}, nickname={nick_data.get('nickname') if nick_data else None}")

            # Verify nickname appears in friends list
            friends_after_nick = await get_friends(session, headers_a)
            nick_friend = next(
                (r for r in friends_after_nick if r.get("id") == new_rel_id), None
            )
            nick_visible = nick_friend is not None and nick_friend.get("nickname") == "TestBuddy"
            check("nickname-visible", nick_visible,
                  f"friend list shows nickname: {nick_friend.get('nickname') if nick_friend else 'N/A'}")

            # Clear nickname
            clear_status, clear_data = await clear_nickname(session, headers_a, new_rel_id)
            nick_cleared = clear_status == 200 and clear_data is not None and clear_data.get("nickname") is None
            check("nickname-cleared", nick_cleared,
                  f"DELETE nickname → {clear_status}, nickname={clear_data.get('nickname') if clear_data else 'N/A'}")

            # Mutual friends (should be 0 for two isolates)
            mut_f = await get_mutual_friends(session, headers_a, user_b_id)
            mut_f_ok = isinstance(mut_f, dict) and mut_f.get("count", -1) >= 0
            check("mutual-friends", mut_f_ok,
                  f"mutual friends count: {mut_f.get('count', 'N/A') if isinstance(mut_f, dict) else 'N/A'}")

            # Mutual servers
            mut_s = await get_mutual_servers(session, headers_a, user_b_id)
            mut_s_ok = isinstance(mut_s, dict) and mut_s.get("count", -1) >= 0
            check("mutual-servers", mut_s_ok,
                  f"mutual servers count: {mut_s.get('count', 'N/A') if isinstance(mut_s, dict) else 'N/A'}")

        # Blocked list
        blocked_data = await get_blocked_list(session, headers_a)
        blocked_ok = isinstance(blocked_data, dict) and "blocked" in blocked_data and "count" in blocked_data
        check("blocked-list", blocked_ok,
              f"GET /friends/blocked → count={blocked_data.get('count', 'N/A') if isinstance(blocked_data, dict) else 'N/A'}")

        # ── Aggregate ───────────────────────────────────────────────────
        all_passed = all(c["passed"] for c in report["checks"])
        report["ok"] = all_passed
        report["summary"] = {
            "total": len(report["checks"]),
            "passed": sum(1 for c in report["checks"] if c["passed"]),
            "failed": sum(1 for c in report["checks"] if not c["passed"]),
        }

        # ── Final cleanup: remove any lingering test relationships ──────
        final_cleanup = await cleanup_existing_relationships(
            session, user_a_id, user_b_id, headers_a, headers_b
        )
        report["cleanup"]["finalRemoved"] = final_cleanup

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    print(f"Wrote {OUT}")

    if args.fail_on_issues and not report["ok"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
