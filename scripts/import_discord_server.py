#!/usr/bin/env python3
"""
import_discord_server.py
Imports the Danish-Truckers.com Discord server structure into OpenCorde.

Usage:
    OC_BASE=https://opencorde.com OC_EMAIL=you@example.com OC_PASSWORD=secret python3 scripts/import_discord_server.py

Creates:
  - Server "Danish-Truckers.com"
  - 24 roles (non-bot, non-managed)
  - 8 categories
  - ~57 channels (text/voice/announcement) with topics, nsfw flags, slowmode

Discord→OpenCorde channel type mapping:
  0 (text)         → 0
  2 (voice)        → 1
  5 (announcement) → 4
  13 (stage)       → 3
"""

import os
import sys
import json
import time
import urllib.request
import urllib.error

# ---------------------------------------------------------------------------
# Config from environment
# ---------------------------------------------------------------------------
BASE = os.environ.get("OC_BASE", "https://opencorde.com").rstrip("/")
EMAIL = os.environ.get("OC_EMAIL", "")
PASSWORD = os.environ.get("OC_PASSWORD", "")

if not EMAIL or not PASSWORD:
    print("ERROR: OC_EMAIL and OC_PASSWORD must be set", file=sys.stderr)
    sys.exit(1)

# ---------------------------------------------------------------------------
# Discovered Discord structure (hardcoded from Emma's API call 2026-03-28)
# Guild: Danish-Truckers.com (995752009351319564)
# ---------------------------------------------------------------------------

# Non-bot, non-managed, non-@everyone roles, ordered position DESC (highest first)
ROLES = [
    {"name": "Channel Bot",     "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "VIP",             "color": 16742003,  "permissions": 0,                    "mentionable": False},
    {"name": "Hardcore Trucker","color": 10070709,  "permissions": 0,                    "mentionable": False},
    {"name": "Crew",            "color": 11342935,  "permissions": 0,                    "mentionable": False},
    {"name": "Admin",           "color": 15844367,  "permissions": 6766367834893905,     "mentionable": False},
    {"name": "PL-Admin",        "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "PL",              "color": None,     "permissions": 1024,                 "mentionable": False},
    {"name": "Members",         "color": None,     "permissions": 274881056256,          "mentionable": False},
    {"name": "Bots",            "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Adult",           "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Minecraft",       "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Minecraft VIP",   "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "VTC",             "color": 5533306,   "permissions": 0,                    "mentionable": False},
    {"name": "Bart Simpson",    "color": 10376700,  "permissions": 2222085186636353,     "mentionable": True},
    {"name": "verified",        "color": 7201172,   "permissions": 1024,                 "mentionable": False},
    {"name": "Muted",           "color": 8487814,   "permissions": 0,                    "mentionable": False},
    {"name": "MC Console Admin","color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Owner",           "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Developer",       "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Admin (ops)",     "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Moderator",       "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Don't show me!",  "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Misc role",       "color": None,     "permissions": 0,                    "mentionable": False},
    {"name": "Linked",          "color": None,     "permissions": 0,                    "mentionable": False},
]

# type: 0=text, 1=voice, 4=announcement
# d_type is the original Discord type (for reference)
CATEGORIES = [
    {
        "name": "General",
        "channels": [
            {"name": "generel-chat",    "type": 0, "topic": "Chat about anything—trucking or otherwise!", "nsfw": False, "slowmode": 0},
            {"name": "welcome",         "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "introductions",   "type": 0, "topic": "Say hello and tell us a bit about yourself", "nsfw": False, "slowmode": 0},
            {"name": "announcements",   "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "guest-chat",      "type": 0, "topic": None, "nsfw": False, "slowmode": 15},
            {"name": "verification",    "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
        ]
    },
    {
        "name": "Notifications",
        "channels": [
            {"name": "cs-go",                "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "ets2",                 "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "live-streams",         "type": 4, "topic": "This channel shows live streams from YT and Twitch", "nsfw": False, "slowmode": 0},
            {"name": "ats",                  "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "truckersmp",           "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "promods",              "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "minecraft",            "type": 0, "topic": "0/20 players online | 3 unique players ever joined", "nsfw": False, "slowmode": 0},
            {"name": "vtc-hub-notifications","type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "gta",                  "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "farming-sim",          "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "mc-console",           "type": 0, "topic": "TPS: 20.0 | Mem: 0.7GB used/0.2GB free/2.9GB max", "nsfw": False, "slowmode": 0},
        ]
    },
    {
        "name": "Sim Games",
        "channels": [
            {"name": "ets2-discussion",       "type": 0, "topic": "For Euro Truck Simulator 2 players.", "nsfw": False, "slowmode": 0},
            {"name": "ats-discussion",        "type": 0, "topic": "For American Truck Simulator players.", "nsfw": False, "slowmode": 0},
            {"name": "convoys-and-events",    "type": 0, "topic": "Share convoy plans, screenshots, or join scheduled events.", "nsfw": False, "slowmode": 0},
            {"name": "modding-talk",          "type": 0, "topic": "Discuss and share mods for ETS2/ATS.", "nsfw": False, "slowmode": 0},
            {"name": "trucking-showcase",     "type": 0, "topic": "Post screenshots of your trucks, customizations, or routes.", "nsfw": False, "slowmode": 0},
            {"name": "fs25",                  "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
        ]
    },
    {
        "name": "Other Games",
        "channels": [
            {"name": "game-room",         "type": 0, "topic": "General chat about non-trucking games", "nsfw": False, "slowmode": 0},
            {"name": "simulation-games",  "type": 0, "topic": "Discuss other sim games like Farming Simulator or Flight Simulator.", "nsfw": False, "slowmode": 0},
            {"name": "fps-and-rpgs",      "type": 0, "topic": "For fans of first-person shooters and role-playing games.", "nsfw": False, "slowmode": 0},
            {"name": "looking-for-group", "type": 0, "topic": "Find players for co-op or multiplayer games.", "nsfw": False, "slowmode": 0},
            {"name": "strategy-and-indie","type": 0, "topic": "Share recommendations or tips for strategy and indie games.", "nsfw": False, "slowmode": 0},
        ]
    },
    {
        "name": "Community",
        "channels": [
            {"name": "pets",              "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "youtube",           "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "memes-and-fun",     "type": 0, "topic": "Share memes, jokes, or funny gaming moments.", "nsfw": False, "slowmode": 0},
            {"name": "nsfw",              "type": 0, "topic": None, "nsfw": True,  "slowmode": 0},
            {"name": "off-topic",         "type": 0, "topic": "Non-gaming topics like movies, tech, or hobbies.", "nsfw": False, "slowmode": 0},
            {"name": "feedback-and-ideas","type": 0, "topic": "Suggestions for improving the server or new features.", "nsfw": False, "slowmode": 0},
            {"name": "ai",               "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
        ]
    },
    {
        "name": "Voice",
        "channels": [
            {"name": "Lobby",       "type": 1, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "Polska",      "type": 1, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "ETS / ATS",   "type": 1, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "Private 1:1", "type": 1, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "Farming Sim", "type": 1, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "+18",         "type": 1, "topic": None, "nsfw": True,  "slowmode": 0},
            {"name": "Music",       "type": 1, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "Inactive",    "type": 1, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "Guest-Voice", "type": 1, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "Crew",        "type": 1, "topic": None, "nsfw": False, "slowmode": 0},
        ]
    },
    {
        "name": "Music",
        "channels": [
            {"name": "music",    "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
            {"name": "disco-bot","type": 0, "topic": None, "nsfw": False, "slowmode": 0},
        ]
    },
    {
        "name": "Support and Help",
        "channels": [
            {"name": "tech-support", "type": 0, "topic": "Assistance with game settings, mods, or hardware issues.", "nsfw": False, "slowmode": 0},
            {"name": "server-help",  "type": 0, "topic": "Questions about using the Discord server.", "nsfw": False, "slowmode": 0},
        ]
    },
]

# Uncategorized channels (no parent category)
UNCATEGORIZED = [
    {"name": "rules",          "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
    {"name": "admins",         "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
    {"name": "moderator-only", "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
    {"name": "emma-ai",        "type": 0, "topic": None, "nsfw": False, "slowmode": 0},
]

# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------
TOKEN = None


def api(method: str, path: str, body=None, expected=None):
    url = f"{BASE}/api/v1{path}"
    headers = {"Content-Type": "application/json"}
    if TOKEN:
        headers["Authorization"] = f"Bearer {TOKEN}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            raw = resp.read()
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        raw = e.read()
        print(f"  ERROR {e.code} {method} {path}: {raw[:300]}", file=sys.stderr)
        if expected and e.code == expected:
            return None
        raise


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main():
    global TOKEN

    # 1. Login
    print("→ Logging in...")
    resp = api("POST", "/auth/login", {"email": EMAIL, "password": PASSWORD})
    TOKEN = resp["access_token"]
    print(f"  Logged in as {resp.get('user', {}).get('username', '?')}")

    # 2. Create server
    print("\n→ Creating server 'Danish-Truckers.com'...")
    server = api("POST", "/servers", {"name": "Danish-Truckers.com"})
    server_id = server["id"]
    print(f"  Server ID: {server_id}")

    # 3. Create roles
    print(f"\n→ Creating {len(ROLES)} roles...")
    for role in ROLES:
        body = {
            "name": role["name"],
            "permissions": role["permissions"],
            "mentionable": role["mentionable"],
        }
        if role["color"] is not None:
            body["color"] = role["color"]
        try:
            r = api("POST", f"/servers/{server_id}/roles", body)
            print(f"  Role: {r['name']} (id={r['id']})")
        except Exception as e:
            print(f"  SKIP role {role['name']}: {e}", file=sys.stderr)
        time.sleep(0.05)  # small delay to avoid rate limits

    # 4. Create categories + their channels
    print(f"\n→ Creating {len(CATEGORIES)} categories and their channels...")
    for cat_def in CATEGORIES:
        cat = api("POST", f"/servers/{server_id}/channels", {
            "name": cat_def["name"],
            "channel_type": 2,
        })
        cat_id = cat["id"]
        print(f"  Category: {cat_def['name']} (id={cat_id})")

        for ch_def in cat_def["channels"]:
            body = {
                "name": ch_def["name"],
                "channel_type": ch_def["type"],
                "parent_id": cat_id,
                "nsfw": ch_def["nsfw"],
                "slowmode_delay": ch_def["slowmode"],
            }
            if ch_def["topic"]:
                body["topic"] = ch_def["topic"]
            try:
                ch = api("POST", f"/servers/{server_id}/channels", body)
                type_name = {0: "text", 1: "voice", 4: "announcement"}.get(ch_def["type"], str(ch_def["type"]))
                print(f"    [{type_name}] {ch['name']}")
            except Exception as e:
                print(f"    SKIP channel {ch_def['name']}: {e}", file=sys.stderr)
            time.sleep(0.05)

    # 5. Uncategorized channels
    print(f"\n→ Creating {len(UNCATEGORIZED)} uncategorized channels...")
    for ch_def in UNCATEGORIZED:
        body = {
            "name": ch_def["name"],
            "channel_type": ch_def["type"],
            "nsfw": ch_def["nsfw"],
            "slowmode_delay": ch_def["slowmode"],
        }
        if ch_def["topic"]:
            body["topic"] = ch_def["topic"]
        try:
            ch = api("POST", f"/servers/{server_id}/channels", body)
            print(f"  [text] {ch['name']}")
        except Exception as e:
            print(f"  SKIP {ch_def['name']}: {e}", file=sys.stderr)
        time.sleep(0.05)

    print(f"\n✓ Done! Server '{server['name']}' created at {BASE}")
    print(f"  Server ID: {server_id}")
    print(f"  Visit: {BASE}/servers/{server_id}")


if __name__ == "__main__":
    main()
