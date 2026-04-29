#!/usr/bin/env python3
"""Schema smoke test for the live OpenCorde database."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MIGRATIONS = ROOT / "crates/opencorde-db/migrations"
OUT = ROOT / "reports/raw/schema-smoke.json"


CREATE_TABLE_RE = re.compile(
    r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([a-zA-Z_][a-zA-Z0-9_]*)\s*\(",
    re.IGNORECASE,
)
ALTER_ADD_RE = re.compile(
    r"ALTER\s+TABLE\s+([a-zA-Z_][a-zA-Z0-9_]*)\s+(.*?);",
    re.IGNORECASE | re.DOTALL,
)
ADD_COLUMN_RE = re.compile(
    r"ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?([a-zA-Z_][a-zA-Z0-9_]*)",
    re.IGNORECASE,
)


def psql(sql: str) -> list[str]:
    result = subprocess.run(
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
            "-At",
            "-F",
            "\t",
            "-c",
            sql,
        ],
        check=True,
        text=True,
        capture_output=True,
    )
    return [line for line in result.stdout.splitlines() if line]


def strip_sql_comments(sql: str) -> str:
    return "\n".join(line.split("--", 1)[0] for line in sql.splitlines())


def split_columns(body: str) -> list[str]:
    columns: list[str] = []
    depth = 0
    current: list[str] = []
    for char in body:
        if char == "(":
            depth += 1
        elif char == ")":
            depth = max(0, depth - 1)
        if char == "," and depth == 0:
            item = "".join(current).strip()
            if item:
                columns.append(item)
            current = []
        else:
            current.append(char)
    item = "".join(current).strip()
    if item:
        columns.append(item)
    return columns


def created_table_body(sql: str, open_paren: int) -> str:
    if open_paren < 0 or open_paren >= len(sql) or sql[open_paren] != "(":
        return ""
    depth = 0
    for index in range(open_paren, len(sql)):
        char = sql[index]
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return sql[open_paren + 1 : index]
    return ""


def expected_schema() -> tuple[set[str], set[tuple[str, str]]]:
    tables: set[str] = set()
    columns: set[tuple[str, str]] = set()

    for path in sorted(MIGRATIONS.glob("*.sql")):
        sql = strip_sql_comments(path.read_text(encoding="utf-8"))
        for match in CREATE_TABLE_RE.finditer(sql):
            table = match.group(1)
            tables.add(table)
            for definition in split_columns(created_table_body(sql, match.end() - 1)):
                first = definition.split(None, 1)[0].strip('"').lower()
                if first in {"primary", "foreign", "unique", "check", "constraint"} or first.startswith(
                    ("unique(", "check(")
                ):
                    continue
                columns.add((table, first))

        for alter in ALTER_ADD_RE.finditer(sql):
            table = alter.group(1)
            for column in ADD_COLUMN_RE.findall(alter.group(2)):
                columns.add((table, column))

    return tables, columns


def live_tables() -> set[str]:
    return set(
        psql(
            "SELECT table_name FROM information_schema.tables "
            "WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
        )
    )


def live_columns() -> set[tuple[str, str]]:
    return {
        tuple(line.split("\t", 1))  # type: ignore[misc]
        for line in psql(
            "SELECT table_name, column_name FROM information_schema.columns "
            "WHERE table_schema = 'public'"
        )
    }


def applied_migrations() -> dict[str, bool]:
    rows = psql("SELECT version::text, success FROM _sqlx_migrations ORDER BY version")
    return {version: success == "t" for version, success in (row.split("\t", 1) for row in rows)}


def main() -> int:
    expected_tables, expected_columns = expected_schema()
    actual_tables = live_tables()
    actual_columns = live_columns()
    applied = applied_migrations()
    migration_versions = {path.name.split("_", 1)[0] for path in MIGRATIONS.glob("*.sql")}

    missing_tables = sorted(expected_tables - actual_tables)
    missing_columns = sorted(
        f"{table}.{column}"
        for table, column in expected_columns
        if table in actual_tables and (table, column) not in actual_columns
    )
    applied_versions = {version.zfill(3) for version in applied}
    missing_migrations = sorted(version for version in migration_versions if version not in applied_versions)
    failed_migrations = sorted(version for version, success in applied.items() if not success)

    report = {
        "ok": not (missing_tables or missing_columns or missing_migrations or failed_migrations),
        "expectedTableCount": len(expected_tables),
        "expectedColumnCount": len(expected_columns),
        "appliedMigrationCount": len(applied),
        "missingTables": missing_tables,
        "missingColumns": missing_columns,
        "missingMigrations": missing_migrations,
        "failedMigrations": failed_migrations,
    }

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
