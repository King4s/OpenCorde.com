#!/usr/bin/env python3
"""Check the adopted Phase 1 source snapshot against its frozen hashes."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "reports" / "phase1-source-manifest.json"
RETIRED = {"client/package-lock.json": "reports/legacy-client-package-lock.json"}


def canonical_lf_bytes(data: bytes) -> bytes:
    """Normalize CRLF only for UTF-8 text files without NUL bytes."""
    if b"\0" in data:
        return data
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        return data
    return data.replace(b"\r\n", b"\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--allow-modified", action="append", default=[], metavar="PATH")
    args = parser.parse_args()
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    expected = {item["path"]: item for item in manifest["files"]}
    allowed = set(args.allow_modified)
    unknown = allowed - expected.keys()
    if unknown:
        print(f"Unknown allowed path(s): {', '.join(sorted(unknown))}", file=sys.stderr)
        return 1
    errors = []
    modified = []
    raw_matches = 0
    canonical_matches = 0
    for relative, item in expected.items():
        file = ROOT / RETIRED.get(relative, relative)
        if relative in RETIRED and (ROOT / relative).exists():
            errors.append(f"retired competing lockfile still present: {relative}")
            continue
        if not file.is_file():
            errors.append(f"missing {relative}")
            continue
        data = file.read_bytes()
        actual_raw = hashlib.sha256(data).hexdigest()
        actual_canonical = hashlib.sha256(canonical_lf_bytes(data)).hexdigest()
        expected_raw = item["sha256"]
        expected_canonical = item.get("canonical_lf_sha256", expected_raw)
        if actual_raw == expected_raw:
            raw_matches += 1
        elif actual_canonical == expected_canonical:
            canonical_matches += 1
        else:
            if relative in allowed and not relative.startswith("crates/opencorde-db/migrations/"):
                modified.append(relative)
            else:
                errors.append(f"raw/canonical hash mismatch {relative}")
    migrations = ROOT / "crates" / "opencorde-db" / "migrations"
    numbers = []
    for file in migrations.glob("*.sql"):
        match = re.match(r"^(\d+)_", file.name)
        if match:
            numbers.append(int(match.group(1)))
    ceiling = manifest["migrationMaximum"]
    if not numbers or max(numbers) != ceiling or any(n > ceiling for n in numbers):
        errors.append(f"migration ceiling expected {ceiling}; found {max(numbers) if numbers else 'none'}")
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print(
        f"PASS: {len(expected)} adopted file hashes checked "
        f"(raw exact: {raw_matches}; canonical LF: {canonical_matches}); "
        f"migration ceiling {ceiling}"
    )
    for relative in modified:
        print(f"INTENTIONAL SOURCE EDIT: {relative}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
