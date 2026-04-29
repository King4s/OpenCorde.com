#!/usr/bin/env python3
"""LiveKit operational health smoke for OpenCorde."""

from __future__ import annotations

import json
import os
import time
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import urlparse, urlunparse


OUT = Path(os.environ.get("OC_LIVEKIT_HEALTH_OUT", "reports/raw/livekit-health.json"))


def http_url(url: str) -> str:
    parsed = urlparse(url)
    scheme = {"ws": "http", "wss": "https"}.get(parsed.scheme, parsed.scheme)
    return urlunparse((scheme, parsed.netloc, parsed.path or "/", "", "", ""))


def check(name: str, url: str) -> dict:
    started = time.monotonic()
    result = {"name": name, "url": url, "ok": False, "status": None, "body": "", "latencyMs": None, "error": None}
    try:
        with urllib.request.urlopen(url, timeout=5) as response:
            body = response.read(128).decode("utf-8", errors="replace")
            result.update(
                {
                    "ok": response.status == 200 and body.strip() == "OK",
                    "status": response.status,
                    "body": body.strip(),
                    "latencyMs": round((time.monotonic() - started) * 1000),
                }
            )
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        result.update({"latencyMs": round((time.monotonic() - started) * 1000), "error": str(exc)})
    return result


def main() -> int:
    local_url = http_url(os.environ.get("LIVEKIT_URL", "ws://localhost:7880"))
    public_url = http_url(os.environ.get("LIVEKIT_PUBLIC_URL", "wss://opencorde.com/livekit"))
    checks = [check("local", local_url), check("public", public_url)]
    report = {"ok": all(item["ok"] for item in checks), "checks": checks}
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
