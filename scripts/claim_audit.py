#!/usr/bin/env python3
"""Fail on public/docs copy that overclaims Discord parity."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

DEFAULT_PATTERNS = [
    r"\bfeature-complete\b",
    r"\bfull feature set\b",
    r"\bcomplete Discord\b",
    r"\bDiscord replacement\b",
    r"Everything your team needs",
]

DEFAULT_INCLUDE = [
    "ReadMeFirst.md",
    "client/src",
    "docs",
    "tasks.md",
    "tasks-future.md",
]

DEFAULT_EXCLUDE = [
    "docs/plans/2026-04-28-discord-parity-master-plan.md",
    "docs/audits/2026-04-28-discord-foundation-audit.md",
    "client/build",
    "reports",
    "target",
    "client/src-tauri/target",
]


def is_excluded(path: Path, excludes: list[str]) -> bool:
    rel = path.relative_to(ROOT).as_posix()
    return any(rel == item or rel.startswith(f"{item}/") for item in excludes)


def iter_files(includes: list[str], excludes: list[str]):
    for item in includes:
        path = ROOT / item
        if not path.exists():
            continue
        if path.is_file():
            if not is_excluded(path, excludes):
                yield path
            continue
        for child in path.rglob("*"):
            if child.is_file() and not is_excluded(child, excludes):
                yield child


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fail", action="store_true", help="Exit non-zero when claims are found")
    args = parser.parse_args()

    pattern = re.compile("|".join(f"(?:{p})" for p in DEFAULT_PATTERNS), re.IGNORECASE)
    findings: list[str] = []

    for path in iter_files(DEFAULT_INCLUDE, DEFAULT_EXCLUDE):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue

        for line_no, line in enumerate(text.splitlines(), start=1):
            if pattern.search(line):
                rel = path.relative_to(ROOT).as_posix()
                findings.append(f"{rel}:{line_no}: {line.strip()}")

    if findings:
        print("Overclaim findings:")
        for finding in findings:
            print(f"- {finding}")
        return 1 if args.fail else 0

    print("No public/docs overclaim findings.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
