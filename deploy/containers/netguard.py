#!/usr/bin/env python3
"""Own only this container namespace's firewall; never mount the Docker socket."""
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile

TABLE = "opencorde"
# This file is in the guard's private mount namespace, never an application mount.
SNAPSHOT = Path("/tmp/opencorde-policy.json")


def enabled(name):
    value = os.environ.get(name, "false")
    if value not in ("true", "false"):
        raise ValueError(f"{name} must be true or false")
    return value == "true"


def rules():
    role = os.environ["GUARD_ROLE"]
    if role not in ("backend", "web", "media"):
        raise ValueError("invalid GUARD_ROLE")
    backend = str(ipaddress.IPv4Address(os.environ["BACKEND_IP"]))
    media = str(ipaddress.IPv4Address(os.environ["MEDIA_IP"]))
    # Priority -150 is BEFORE Docker's output DNAT (-100). This prevents
    # Docker embedded DNS from escaping through the host's resolver.
    lines = [f"table inet {TABLE} {{",
             "chain dns { type filter hook output priority -150; policy accept;",
             "ip daddr 127.0.0.11 drop", "udp dport 53 drop", "tcp dport 53 drop", "}",
             "chain output { type filter hook output priority 0; policy drop;",
             'oifname "lo" accept', "ct state established,related accept"]
    if role == "web":
        lines += [f"ip daddr {backend} tcp dport {{ 3000, 9000 }} accept",
                  f"ip daddr {media} tcp dport {{ 7880, 5349 }} accept"]
    elif role == "backend":
        lines += [f"ip daddr {media} tcp dport 7880 accept"]
    elif role == "media":
        lines += [f"ip daddr {backend} tcp dport 6379 accept"]
        if enabled("MEDIA_EGRESS"):
            # Client ICE/TURN traffic is an explicit opt-in, confined to this
            # namespace. Never grant media access to metadata/loopback ranges.
            lines += ["ip daddr { 0.0.0.0/8, 127.0.0.0/8, 169.254.0.0/16, 224.0.0.0/4 } drop"]
            for cidr in os.environ.get("MEDIA_CLIENT_CIDRS", "0.0.0.0/0").split(","):
                subnet = str(ipaddress.IPv4Network(cidr.strip(), strict=True))
                lines.append(f"ip daddr {subnet} meta l4proto udp accept")
    lines += ["}", "chain forward { type filter hook forward priority 0; policy drop; }", "}"]
    return "\n".join(lines) + "\n"


def normalize_policy(value):
    """Ignore nft object identifiers/version metadata, preserve all policy/order."""
    if isinstance(value, dict):
        return {key: normalize_policy(item) for key, item in value.items()
                if key not in ("handle", "metainfo")}
    if isinstance(value, list):
        return [normalize_policy(item) for item in value
                if not (isinstance(item, dict) and "metainfo" in item)]
    return value


def current_policy():
    # Compare every object in the owned table. Docker's unrelated DNAT tables
    # contain changing counters; other ACCEPT chains cannot override our DROP.
    # Rule order and all expressions in this table remain significant.
    result = subprocess.run(["nft", "-j", "list", "table", "inet", TABLE], capture_output=True)
    if result.returncode:
        raise RuntimeError("firewall policy inspection failed")
    policy = json.loads(result.stdout)
    if not isinstance(policy, dict) or not isinstance(policy.get("nftables"), list):
        raise ValueError("invalid firewall policy response")
    return normalize_policy(policy)


def expected_snapshot():
    text = rules()
    config = {name: os.environ.get(name, default) for name, default in (
        ("GUARD_ROLE", ""), ("BACKEND_IP", ""), ("MEDIA_IP", ""),
        ("MEDIA_EGRESS", "false"), ("MEDIA_CLIENT_CIDRS", "0.0.0.0/0"))}
    return {"version": 1, "expected_rules": text,
            "rules_sha256": hashlib.sha256(text.encode()).hexdigest(),
            "config_sha256": hashlib.sha256(json.dumps(config, sort_keys=True).encode()).hexdigest()}


def record_installed_policy():
    # Called only AFTER nft's atomic transaction succeeds. Replace atomically so
    # a concurrent health probe cannot accept a partially written checkpoint.
    snapshot = {**expected_snapshot(), "policy": current_policy()}
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", dir=SNAPSHOT.parent,
                                         prefix=".opencorde-policy-", delete=False) as file:
            temporary = Path(file.name)
            os.chmod(temporary, 0o600)
            json.dump(snapshot, file, sort_keys=True)
        os.replace(temporary, SNAPSHOT)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def healthy():
    try:
        saved = json.loads(SNAPSHOT.read_text())
        expected = expected_snapshot()
        if any(saved.get(key) != value for key, value in expected.items()):
            return False
        return saved.get("policy") == current_policy()
    except (OSError, ValueError, KeyError, TypeError, RuntimeError):
        return False


def main():
    if sys.argv[1:] == ["health"]:
        return 0 if healthy() else 1
    text = rules()  # Validate before touching the existing policy.
    SNAPSHOT.unlink(missing_ok=True)  # Never reuse health evidence from an earlier install.
    exists = subprocess.run(["nft", "list", "table", "inet", TABLE], capture_output=True).returncode == 0
    if exists:
        text = f"delete table inet {TABLE}\n" + text
    # One nft transaction replaces the owned table without an ACCEPT interval.
    subprocess.run(["nft", "-f", "-"], input=text, text=True, check=True)
    record_installed_policy()
    if not healthy():
        raise RuntimeError("firewall policy verification failed")
    print("Namespace privacy policy installed", flush=True)
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    while True:
        signal.pause()


if __name__ == "__main__":
    sys.exit(main())
