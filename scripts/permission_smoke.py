#!/usr/bin/env python3
"""Permission smoke tests against a running OpenCorde API."""

from __future__ import annotations

import asyncio
import base64
import json
import os
import re
import subprocess
import time
from datetime import datetime, timedelta, timezone
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
LIMITED_USERNAME = os.environ.get("OC_LIMITED_USERNAME", "permission_limited")
LIMITED_EMAIL = os.environ.get("OC_LIMITED_EMAIL", "permission-limited@opencorde.local")
LIMITED_PASSWORD = os.environ.get("OC_LIMITED_PASSWORD", "PermissionSmoke@99")


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


async def ensure_limited_user(session: aiohttp.ClientSession) -> str:
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
    return await login(session, LIMITED_EMAIL, LIMITED_PASSWORD)


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


def cleanup_private_channel_fixture(
    *,
    channel_id: int,
    role_id: int,
    server_id: str,
    limited_user_id: str,
) -> None:
    psql(
        f"""
        DELETE FROM channel_permission_overrides WHERE channel_id = {channel_id};
        DELETE FROM channels WHERE id = {channel_id};
        DELETE FROM member_roles WHERE role_id = {role_id}
           OR (user_id = {limited_user_id} AND server_id = {server_id});
        DELETE FROM roles WHERE id = {role_id};
        DELETE FROM server_members WHERE user_id = {limited_user_id} AND server_id = {server_id};
        """
    )


def cleanup_voice_fixture(
    *,
    channel_id: int,
    server_id: str,
    limited_user_id: str,
) -> None:
    psql(
        f"""
        DELETE FROM voice_states WHERE user_id = {limited_user_id};
        DELETE FROM channel_permission_overrides WHERE channel_id = {channel_id};
        DELETE FROM channels WHERE id = {channel_id};
        DELETE FROM server_members WHERE user_id = {limited_user_id} AND server_id = {server_id};
        """
    )


def cleanup_stage_fixture(
    *,
    channel_ids: list[int],
    server_id: str,
    limited_user_id: str,
) -> None:
    channel_list = ",".join(str(channel_id) for channel_id in channel_ids)
    psql(
        f"""
        DELETE FROM stage_participants WHERE channel_id IN ({channel_list});
        DELETE FROM stage_sessions WHERE channel_id IN ({channel_list});
        DELETE FROM channel_permission_overrides WHERE channel_id IN ({channel_list});
        DELETE FROM channels WHERE id IN ({channel_list});
        DELETE FROM server_members WHERE user_id = {limited_user_id} AND server_id = {server_id};
        """
    )


def cleanup_role_reorder_fixture(
    *,
    role_ids: list[int],
    server_id: str,
    limited_user_id: str,
) -> None:
    role_list = ",".join(str(role_id) for role_id in role_ids)
    psql(
        f"""
        DELETE FROM member_roles WHERE role_id IN ({role_list})
           OR (user_id = {limited_user_id} AND server_id = {server_id});
        DELETE FROM roles WHERE id IN ({role_list});
        DELETE FROM server_members WHERE user_id = {limited_user_id} AND server_id = {server_id};
        """
    )


def decode_jwt_payload(token: str) -> dict:
    payload = token.split(".")[1]
    payload += "=" * (-len(payload) % 4)
    return json.loads(base64.urlsafe_b64decode(payload.encode("ascii")))


async def main() -> int:
    results: list[dict] = []

    async with aiohttp.ClientSession() as session:
        member_token = await login(session, MEMBER_EMAIL, MEMBER_PASSWORD)
        nonmember_token = await ensure_nonmember(session)
        limited_token = await ensure_limited_user(session)

        member_headers = {"Authorization": f"Bearer {member_token}"}
        nonmember_headers = {"Authorization": f"Bearer {nonmember_token}"}
        limited_headers = {"Authorization": f"Bearer {limited_token}"}

        status, member_profile = await request_json(
            session,
            "GET",
            f"{API}/users/@me",
            headers=member_headers,
        )
        if status != 200 or not member_profile:
            raise RuntimeError(f"member profile baseline failed: {status} {member_profile}")
        member_id = member_profile["id"]

        await request_json(
            session,
            "POST",
            f"{API}/users/me/key-packages",
            headers=member_headers,
            json={"key_package": "AQ"},
        )

        status, servers = await request_json(session, "GET", f"{API}/servers", headers=member_headers)
        if status != 200 or not servers:
            raise RuntimeError(f"member has no server baseline: {status} {servers}")
        server_id = servers[0]["id"]

        status, limited_profile = await request_json(
            session,
            "GET",
            f"{API}/users/@me",
            headers=limited_headers,
        )
        if status != 200 or not limited_profile:
            raise RuntimeError(f"limited profile baseline failed: {status} {limited_profile}")
        limited_user_id = limited_profile["id"]

        now_ms = int(time.time() * 1000)
        private_channel_id = now_ms * 1000 + 101
        allowed_role_id = now_ms * 1000 + 102
        cleanup_private_channel_fixture(
            channel_id=private_channel_id,
            role_id=allowed_role_id,
            server_id=server_id,
            limited_user_id=limited_user_id,
        )
        psql(
            f"""
            INSERT INTO server_members (user_id, server_id)
            VALUES ({limited_user_id}, {server_id})
            ON CONFLICT DO NOTHING;

            INSERT INTO channels (id, server_id, name, channel_type, position)
            VALUES ({private_channel_id}, {server_id}, 'permission-private-smoke', 0, 9999);

            INSERT INTO roles (id, server_id, name, permissions, position)
            VALUES ({allowed_role_id}, {server_id}, 'permission-private-allow', {1 << 10}, 10);

            INSERT INTO channel_permission_overrides (channel_id, target_type, target_id, allow_bits, deny_bits)
            VALUES
              ({private_channel_id}, 'role', {server_id}, 0, {1 << 10}),
              ({private_channel_id}, 'role', {allowed_role_id}, {1 << 10}, 0);
            """
        )

        try:
            status, limited_channels = await request_json(
                session,
                "GET",
                f"{API}/servers/{server_id}/channels",
                headers=limited_headers,
            )
            hidden_from_list = (
                status == 200
                and isinstance(limited_channels, list)
                and str(private_channel_id) not in {c.get("id") for c in limited_channels if isinstance(c, dict)}
            )
            results.append(
                {
                    "name": "private channel hidden from member without allowed role",
                    "method": "GET",
                    "url": f"/api/v1/servers/{server_id}/channels",
                    "expectedStatus": 200,
                    "actualStatus": status,
                    "ok": hidden_from_list,
                    "response": limited_channels,
                }
            )
            print(
                f"[{'PASS' if hidden_from_list else 'FAIL'}] private channel hidden from member without allowed role expected=200 actual={status}"
            )

            status, data = await request_json(
                session,
                "GET",
                f"{API}/channels/{private_channel_id}/messages",
                headers=limited_headers,
            )
            denied_messages = status == 403
            results.append(
                {
                    "name": "private channel messages denied without allowed role",
                    "method": "GET",
                    "url": f"/api/v1/channels/{private_channel_id}/messages",
                    "expectedStatus": 403,
                    "actualStatus": status,
                    "ok": denied_messages,
                    "response": data,
                }
            )
            print(
                f"[{'PASS' if denied_messages else 'FAIL'}] private channel messages denied without allowed role expected=403 actual={status}"
            )

            psql(
                f"""
                INSERT INTO member_roles (user_id, server_id, role_id)
                VALUES ({limited_user_id}, {server_id}, {allowed_role_id})
                ON CONFLICT DO NOTHING;
                """
            )

            status, limited_channels = await request_json(
                session,
                "GET",
                f"{API}/servers/{server_id}/channels",
                headers=limited_headers,
            )
            visible_in_list = (
                status == 200
                and isinstance(limited_channels, list)
                and str(private_channel_id) in {c.get("id") for c in limited_channels if isinstance(c, dict)}
            )
            results.append(
                {
                    "name": "private channel visible with allowed role",
                    "method": "GET",
                    "url": f"/api/v1/servers/{server_id}/channels",
                    "expectedStatus": 200,
                    "actualStatus": status,
                    "ok": visible_in_list,
                    "response": limited_channels,
                }
            )
            print(
                f"[{'PASS' if visible_in_list else 'FAIL'}] private channel visible with allowed role expected=200 actual={status}"
            )

            status, data = await request_json(
                session,
                "GET",
                f"{API}/channels/{private_channel_id}/messages",
                headers=limited_headers,
            )
            allowed_messages = status == 200
            results.append(
                {
                    "name": "private channel messages allowed with allowed role",
                    "method": "GET",
                    "url": f"/api/v1/channels/{private_channel_id}/messages",
                    "expectedStatus": 200,
                    "actualStatus": status,
                    "ok": allowed_messages,
                    "response": data,
                }
            )
            print(
                f"[{'PASS' if allowed_messages else 'FAIL'}] private channel messages allowed with allowed role expected=200 actual={status}"
            )
        finally:
            cleanup_private_channel_fixture(
                channel_id=private_channel_id,
                role_id=allowed_role_id,
                server_id=server_id,
                limited_user_id=limited_user_id,
            )

        voice_channel_id = now_ms * 1000 + 201
        cleanup_voice_fixture(
            channel_id=voice_channel_id,
            server_id=server_id,
            limited_user_id=limited_user_id,
        )
        psql(
            f"""
            INSERT INTO server_members (user_id, server_id)
            VALUES ({limited_user_id}, {server_id})
            ON CONFLICT DO NOTHING;

            INSERT INTO channels (id, server_id, name, channel_type, position)
            VALUES ({voice_channel_id}, {server_id}, 'permission-voice-smoke', 1, 9999);

            INSERT INTO channel_permission_overrides (channel_id, target_type, target_id, allow_bits, deny_bits)
            VALUES ({voice_channel_id}, 'role', {server_id}, 0, {1 << 21});
            """
        )

        try:
            status, data = await request_json(
                session,
                "POST",
                f"{API}/voice/join",
                headers=limited_headers,
                json={"channel_id": str(voice_channel_id)},
            )
            can_publish = None
            if status == 200 and isinstance(data, dict):
                token = data.get("livekit_token")
                if token:
                    can_publish = decode_jwt_payload(token).get("video", {}).get("canPublish")
            ok = status == 200 and can_publish is False
            results.append(
                {
                    "name": "voice CONNECT without SPEAK gets subscribe-only token",
                    "method": "POST",
                    "url": "/api/v1/voice/join",
                    "expectedStatus": 200,
                    "actualStatus": status,
                    "ok": ok,
                    "response": {"canPublish": can_publish},
                }
            )
            print(
                f"[{'PASS' if ok else 'FAIL'}] voice CONNECT without SPEAK gets subscribe-only token expected=200 actual={status}"
            )

            status, data = await request_json(
                session,
                "POST",
                f"{API}/livekit/token",
                headers=limited_headers,
                json={"channel_id": str(voice_channel_id)},
            )
            can_publish = None
            if status == 200 and isinstance(data, dict):
                token = data.get("token")
                if token:
                    can_publish = decode_jwt_payload(token).get("video", {}).get("canPublish")
            ok = status == 200 and can_publish is False
            results.append(
                {
                    "name": "fresh LiveKit token without SPEAK stays subscribe-only",
                    "method": "POST",
                    "url": "/api/v1/livekit/token",
                    "expectedStatus": 200,
                    "actualStatus": status,
                    "ok": ok,
                    "response": {"canPublish": can_publish},
                }
            )
            print(
                f"[{'PASS' if ok else 'FAIL'}] fresh LiveKit token without SPEAK stays subscribe-only expected=200 actual={status}"
            )
        finally:
            cleanup_voice_fixture(
                channel_id=voice_channel_id,
                server_id=server_id,
                limited_user_id=limited_user_id,
            )

        denied_stage_channel_id = now_ms * 1000 + 301
        limited_stage_channel_id = now_ms * 1000 + 302
        denied_stage_session_id = now_ms * 1000 + 303
        limited_stage_session_id = now_ms * 1000 + 304
        limited_stage_participant_id = now_ms * 1000 + 305
        stage_channel_ids = [denied_stage_channel_id, limited_stage_channel_id]
        cleanup_stage_fixture(
            channel_ids=stage_channel_ids,
            server_id=server_id,
            limited_user_id=limited_user_id,
        )
        psql(
            f"""
            INSERT INTO server_members (user_id, server_id)
            VALUES ({limited_user_id}, {server_id})
            ON CONFLICT DO NOTHING;

            INSERT INTO channels (id, server_id, name, channel_type, position)
            VALUES
              ({denied_stage_channel_id}, {server_id}, 'permission-stage-no-connect', 3, 9999),
              ({limited_stage_channel_id}, {server_id}, 'permission-stage-limited', 3, 9999);

            INSERT INTO stage_sessions (id, channel_id, topic, started_by)
            VALUES
              ({denied_stage_session_id}, {denied_stage_channel_id}, 'permission smoke', {member_id}),
              ({limited_stage_session_id}, {limited_stage_channel_id}, 'permission smoke', {member_id});

            INSERT INTO stage_participants (id, channel_id, user_id, role)
            VALUES ({limited_stage_participant_id}, {limited_stage_channel_id}, {limited_user_id}, 'audience');

            INSERT INTO channel_permission_overrides (channel_id, target_type, target_id, allow_bits, deny_bits)
            VALUES
              ({denied_stage_channel_id}, 'role', {server_id}, 0, {1 << 20}),
              ({limited_stage_channel_id}, 'role', {server_id}, 0, {(1 << 21) | (1 << 32)});
            """
        )

        try:
            stage_checks = [
                {
                    "name": "stage detail denied without CONNECT",
                    "method": "GET",
                    "url": f"{API}/channels/{denied_stage_channel_id}/stage",
                    "headers": limited_headers,
                    "expect": 403,
                },
                {
                    "name": "stage join denied without CONNECT",
                    "method": "POST",
                    "url": f"{API}/channels/{denied_stage_channel_id}/stage/join",
                    "headers": limited_headers,
                    "expect": 403,
                },
                {
                    "name": "stage hand raise denied without REQUEST_TO_SPEAK",
                    "method": "POST",
                    "url": f"{API}/channels/{limited_stage_channel_id}/stage/hand",
                    "headers": limited_headers,
                    "json": {"raised": True},
                    "expect": 403,
                },
                {
                    "name": "stage speaker promotion denied when target lacks SPEAK",
                    "method": "PATCH",
                    "url": f"{API}/channels/{limited_stage_channel_id}/stage/speakers/{limited_user_id}",
                    "headers": member_headers,
                    "json": {"speaker": True},
                    "expect": 403,
                },
            ]

            for check in stage_checks:
                kwargs = {"headers": check["headers"]}
                if "json" in check:
                    kwargs["json"] = check["json"]
                status, data = await request_json(session, check["method"], check["url"], **kwargs)
                ok = status == check["expect"]
                results.append(
                    {
                        "name": check["name"],
                        "method": check["method"],
                        "url": check["url"].replace(API, "/api/v1"),
                        "expectedStatus": check["expect"],
                        "actualStatus": status,
                        "ok": ok,
                        "response": data,
                    }
                )
                print(
                    f"[{'PASS' if ok else 'FAIL'}] {check['name']} expected={check['expect']} actual={status}"
                )
        finally:
            cleanup_stage_fixture(
                channel_ids=stage_channel_ids,
                server_id=server_id,
                limited_user_id=limited_user_id,
            )

        manager_role_id = now_ms * 1000 + 401
        target_role_id = now_ms * 1000 + 402
        role_reorder_ids = [manager_role_id, target_role_id]
        cleanup_role_reorder_fixture(
            role_ids=role_reorder_ids,
            server_id=server_id,
            limited_user_id=limited_user_id,
        )
        psql(
            f"""
            INSERT INTO server_members (user_id, server_id)
            VALUES ({limited_user_id}, {server_id})
            ON CONFLICT DO NOTHING;

            INSERT INTO roles (id, server_id, name, permissions, position)
            VALUES
              ({manager_role_id}, {server_id}, 'permission-reorder-manager', {1 << 28}, 10),
              ({target_role_id}, {server_id}, 'permission-reorder-target', 0, 1);

            INSERT INTO member_roles (user_id, server_id, role_id)
            VALUES ({limited_user_id}, {server_id}, {manager_role_id})
            ON CONFLICT DO NOTHING;
            """
        )

        try:
            status, data = await request_json(
                session,
                "PATCH",
                f"{API}/servers/{server_id}/roles",
                headers=limited_headers,
                json=[{"id": str(target_role_id), "position": 10}],
            )
            ok = status == 403
            results.append(
                {
                    "name": "role batch reorder cannot move role to actor position",
                    "method": "PATCH",
                    "url": f"/api/v1/servers/{server_id}/roles",
                    "expectedStatus": 403,
                    "actualStatus": status,
                    "ok": ok,
                    "response": data,
                }
            )
            print(
                f"[{'PASS' if ok else 'FAIL'}] role batch reorder cannot move role to actor position expected=403 actual={status}"
            )
        finally:
            cleanup_role_reorder_fixture(
                role_ids=role_reorder_ids,
                server_id=server_id,
                limited_user_id=limited_user_id,
            )

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
            {
                "name": "nonmember cannot list server roles",
                "method": "GET",
                "url": f"{API}/servers/{server_id}/roles",
                "expect": 403,
            },
            {
                "name": "nonmember cannot consume unrelated user key package",
                "method": "GET",
                "url": f"{API}/users/{member_id}/key-packages/one",
                "expect": 403,
            },
            {
                "name": "nonmember cannot list server events",
                "method": "GET",
                "url": f"{API}/servers/{server_id}/events",
                "expect": 403,
            },
            {
                "name": "nonmember cannot create server event",
                "method": "POST",
                "url": f"{API}/servers/{server_id}/events",
                "json": {
                    "title": "permission smoke event",
                    "starts_at": (datetime.now(timezone.utc) + timedelta(days=1)).isoformat(),
                    "location_type": "external",
                    "location_name": "online",
                },
                "expect": 403,
            },
            {
                "name": "nonmember cannot list soundboard",
                "method": "GET",
                "url": f"{API}/servers/{server_id}/soundboard",
                "expect": 403,
            },
            {
                "name": "nonmember cannot create soundboard sound",
                "method": "POST",
                "url": f"{API}/servers/{server_id}/soundboard",
                "json": {"name": "smoke", "file_key": "smoke.wav"},
                "expect": 403,
            },
            {
                "name": "nonmember cannot delete soundboard sound",
                "method": "DELETE",
                "url": f"{API}/servers/{server_id}/soundboard/1",
                "expect": 403,
            },
            {
                "name": "nonmember cannot play soundboard sound",
                "method": "POST",
                "url": f"{API}/servers/{server_id}/soundboard/1/play",
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
                    {
                        "name": "nonmember cannot list channel recordings",
                        "method": "GET",
                        "url": f"{API}/channels/{channel_id}/recordings",
                        "expect": 403,
                    },
                    {
                        "name": "nonmember cannot ack channel read state",
                        "method": "POST",
                        "url": f"{API}/channels/{channel_id}/ack",
                        "json": {"message_id": message_id or "1"},
                        "expect": 403,
                    },
                    {
                        "name": "nonmember cannot initialize e2ee group",
                        "method": "POST",
                        "url": f"{API}/channels/{channel_id}/e2ee/init",
                        "json": {"group_state": "AQ", "member_welcomes": []},
                        "expect": 403,
                    },
                    {
                        "name": "nonmember cannot fetch e2ee welcome",
                        "method": "GET",
                        "url": f"{API}/channels/{channel_id}/e2ee/welcome",
                        "expect": 403,
                    },
                    {
                        "name": "nonmember cannot update e2ee state",
                        "method": "PUT",
                        "url": f"{API}/channels/{channel_id}/e2ee/state",
                        "json": {"group_state": "AQ"},
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
