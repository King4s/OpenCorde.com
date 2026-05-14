#!/usr/bin/env python3
"""Generate a route inventory JSON from Axum route declarations + permission annotations.

Walks the API module graph from src/routes/mod.rs and src/ws/mod.rs, parses
.route(...) calls, resolves handler functions, and extracts permission gates
declared inside their bodies.

Output: reports/raw/route-inventory.json (or $OC_ROUTE_INVENTORY_OUT).
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
API_SRC = ROOT / "crates" / "opencorde-api" / "src"
OUT = Path(os.environ.get("OC_ROUTE_INVENTORY_OUT", "reports/raw/route-inventory.json"))

ENTRY_POINTS = [
    API_SRC / "routes" / "mod.rs",
    API_SRC / "ws" / "handler" / "mod.rs",
]

METHOD_FNS = {"get", "post", "put", "delete", "patch", "head", "options"}

# Permission helpers we care about. Each pattern matches a `require_*_perm(`
# opener; the call's full argument list is then scanned for every `Permissions::FOO`.
PERM_CALL_RE = re.compile(r"require_(server|channel)_perm\s*\(")
PERM_FLAG_RE = re.compile(r"Permissions::([A-Z][A-Z0-9_]*)")
EFFECTIVE_PATTERNS = [
    ("server", re.compile(r"effective_server_perms\s*\(")),
    ("channel", re.compile(r"effective_channel_perms\s*\(")),
]
ADMIN_PATTERN = re.compile(r"\bis_admin\s*\(")
VERIFICATION_PATTERN = re.compile(r"check_verification_level\s*\(")
HIERARCHY_PATTERN = re.compile(r"check_role_hierarchy|require_higher_role|require_member_below_actor")
RATE_LIMIT_PATTERN = re.compile(r"rate_limit::check|RateLimitGuard")
# Inline ownership/authorship checks against the calling user.
OWNER_PATTERN = re.compile(r"owner_id\s*(?:!=|==)\s*auth\.user_id")
AUTHOR_PATTERN = re.compile(r"author_id\s*(?:!=|==)\s*auth\.user_id")
# Membership lookups gated on the caller (not on a target user).
MEMBER_PATTERN = re.compile(r"member_repo::get_member\s*\(\s*[^,]+,\s*auth\.user_id\b")


def resolve_module(parent: Path, mod_name: str) -> Path | None:
    """Resolve a `pub mod NAME;` declaration to its source file.

    For parent file `foo.rs` (or `foo/mod.rs`), child is either
    `foo/NAME.rs` or `foo/NAME/mod.rs`.
    """
    base = parent.with_suffix("") if parent.name != "mod.rs" else parent.parent
    sibling = base / f"{mod_name}.rs"
    if sibling.is_file():
        return sibling
    nested = base / mod_name / "mod.rs"
    if nested.is_file():
        return nested
    return None


MOD_RE = re.compile(r"^\s*(?:pub\s+(?:\([^)]*\)\s+)?)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;", re.MULTILINE)


def walk_modules(entry: Path, visited: set[Path]) -> None:
    if entry in visited or not entry.is_file():
        return
    visited.add(entry)
    text = entry.read_text(encoding="utf-8", errors="replace")
    # Strip line comments to avoid following commented-out modules.
    stripped = re.sub(r"//.*", "", text)
    for m in MOD_RE.finditer(stripped):
        child = resolve_module(entry, m.group(1))
        if child is not None:
            walk_modules(child, visited)


def find_balanced_block(text: str, open_idx: int) -> tuple[int, int]:
    """Given index pointing at '{', return (open_idx, close_idx_exclusive)."""
    depth = 0
    i = open_idx
    in_str = False
    in_char = False
    in_line_comment = False
    in_block_comment = False
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""
        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
        elif in_block_comment:
            if ch == "*" and nxt == "/":
                in_block_comment = False
                i += 1
        elif in_str:
            if ch == "\\":
                i += 1
            elif ch == '"':
                in_str = False
        elif in_char:
            if ch == "\\":
                i += 1
            elif ch == "'":
                in_char = False
        else:
            if ch == "/" and nxt == "/":
                in_line_comment = True
                i += 1
            elif ch == "/" and nxt == "*":
                in_block_comment = True
                i += 1
            elif ch == '"':
                in_str = True
            elif ch == "'":
                # Could be a lifetime; only treat as char literal if followed by char + '
                if i + 2 < len(text) and text[i + 2] == "'":
                    in_char = True
            elif ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    return open_idx, i + 1
        i += 1
    return open_idx, len(text)


FN_SIG_RE = re.compile(
    r"\b(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:<[^>]*>)?\s*\(",
)


def extract_fns(text: str) -> dict[str, str]:
    """Return {fn_name: signature_plus_body} for every fn definition in the file.

    Includes the parameter list so we can detect extractors like `AuthUser`.
    Trait fns without a body are skipped.
    """
    fns: dict[str, str] = {}
    for m in FN_SIG_RE.finditer(text):
        name = m.group(1)
        sig_start = m.start()
        # Skip past matching ')' for the signature's parameter list.
        i = m.end()
        depth = 1
        while i < len(text) and depth > 0:
            if text[i] == "(":
                depth += 1
            elif text[i] == ")":
                depth -= 1
            i += 1
        # Skip return type / where clause until '{' or ';'.
        while i < len(text) and text[i] not in "{;":
            i += 1
        if i >= len(text) or text[i] != "{":
            continue
        _, end = find_balanced_block(text, i)
        # Last definition wins on collisions (private helpers usually shadow).
        fns[name] = text[sig_start:end]
    return fns


ROUTE_RE = re.compile(r"\.route\s*\(\s*", re.MULTILINE)


def parse_route_call(text: str, start: int) -> tuple[str, str, int] | None:
    """Parse `.route("PATH", EXPR)` starting at `start` (pointing past `.route(`).

    Returns (path, expr, end_index) or None.
    """
    # Expect a string literal next.
    i = start
    while i < len(text) and text[i].isspace():
        i += 1
    if i >= len(text) or text[i] != '"':
        return None
    j = i + 1
    while j < len(text):
        if text[j] == "\\":
            j += 2
            continue
        if text[j] == '"':
            break
        j += 1
    if j >= len(text):
        return None
    path = text[i + 1 : j]
    # Skip past comma.
    k = j + 1
    while k < len(text) and text[k] in " \t\r\n":
        k += 1
    if k >= len(text) or text[k] != ",":
        return None
    k += 1
    # Now read the expression up to the matching ')' for the .route(.
    depth = 1
    expr_start = k
    in_str = False
    while k < len(text) and depth > 0:
        ch = text[k]
        if in_str:
            if ch == "\\":
                k += 2
                continue
            if ch == '"':
                in_str = False
        else:
            if ch == '"':
                in_str = True
            elif ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
                if depth == 0:
                    break
        k += 1
    if depth != 0:
        return None
    expr = text[expr_start:k].strip().rstrip(",").strip()
    return path, expr, k + 1


METHOD_CALL_RE = re.compile(
    r"\b(get|post|put|delete|patch|head|options)\s*\(\s*([A-Za-z_][A-Za-z0-9_:]*)\s*\)",
)


def extract_methods(expr: str) -> list[tuple[str, str]]:
    """From an expression like `get(foo).post(bar)`, return [(METHOD, handler_path)]."""
    out: list[tuple[str, str]] = []
    for m in METHOD_CALL_RE.finditer(expr):
        out.append((m.group(1).upper(), m.group(2)))
    return out


def collect_routes(files: list[Path]) -> list[dict]:
    """Walk each file's `pub fn router()` body and extract route declarations."""
    routes: list[dict] = []
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        for m in re.finditer(r"\bfn\s+router\s*\(", text):
            # Find body of this fn
            i = m.end()
            depth = 1
            while i < len(text) and depth > 0:
                if text[i] == "(":
                    depth += 1
                elif text[i] == ")":
                    depth -= 1
                i += 1
            while i < len(text) and text[i] not in "{;":
                i += 1
            if i >= len(text) or text[i] != "{":
                continue
            _, end = find_balanced_block(text, i)
            body = text[i:end]
            for r in ROUTE_RE.finditer(body):
                parsed = parse_route_call(body, r.end())
                if parsed is None:
                    continue
                route_path, expr, _ = parsed
                methods = extract_methods(expr)
                if not methods:
                    continue
                routes.append(
                    {
                        "path": route_path,
                        "methods_handlers": methods,
                        "declared_in": str(path.relative_to(ROOT)),
                    }
                )
    return routes


def index_handlers(files: list[Path]) -> dict[str, list[tuple[Path, str]]]:
    """Build {fn_name: [(file, body), ...]} across all files."""
    idx: dict[str, list[tuple[Path, str]]] = defaultdict(list)
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        for name, body in extract_fns(text).items():
            idx[name].append((path, body))
    return idx


def resolve_handler(
    handler_path: str,
    declaring_file: Path,
    handlers: dict[str, list[tuple[Path, str]]],
) -> tuple[Path, str] | None:
    """Pick the best match for a handler reference, preferring same module subtree."""
    name = handler_path.split("::")[-1]
    candidates = handlers.get(name, [])
    if not candidates:
        return None
    if len(candidates) == 1:
        return candidates[0]
    # Prefer candidates inside the same module subtree as the declaring file.
    base = declaring_file.with_suffix("") if declaring_file.name != "mod.rs" else declaring_file.parent
    in_subtree = [c for c in candidates if str(c[0]).startswith(str(base) + os.sep) or c[0] == declaring_file]
    if in_subtree:
        return in_subtree[0]
    # Fall back to the first one.
    return candidates[0]


def extract_perms(body: str) -> dict:
    """Pull permission/admin/verification annotations from a fn body."""
    perms: list[str] = []
    for m in PERM_CALL_RE.finditer(body):
        scope = m.group(1)
        # Read the call's full argument list so OR'd flags are all captured.
        depth = 1
        i = m.end()
        while i < len(body) and depth > 0:
            if body[i] == "(":
                depth += 1
            elif body[i] == ")":
                depth -= 1
            i += 1
        args = body[m.end() : max(i - 1, m.end())]
        for f in PERM_FLAG_RE.finditer(args):
            perms.append(f"{scope}:{f.group(1)}")
    inspections: list[str] = []
    for scope, pat in EFFECTIVE_PATTERNS:
        if pat.search(body):
            inspections.append(f"{scope}:effective")
    flags: list[str] = []
    if ADMIN_PATTERN.search(body):
        flags.append("admin")
    if VERIFICATION_PATTERN.search(body):
        flags.append("verification_level")
    if HIERARCHY_PATTERN.search(body):
        flags.append("role_hierarchy")
    if RATE_LIMIT_PATTERN.search(body):
        flags.append("rate_limit")
    if OWNER_PATTERN.search(body):
        flags.append("owner_check")
    if AUTHOR_PATTERN.search(body):
        flags.append("author_check")
    if MEMBER_PATTERN.search(body):
        flags.append("member_check")
    auth_required = bool(re.search(r"\bAuthUser\b", body))
    return {
        "permissions": sorted(set(perms)),
        "inspects_effective": sorted(set(inspections)),
        "flags": sorted(set(flags)),
        "auth_required": auth_required,
    }


def auth_class(annotations: dict) -> str:
    if "admin" in annotations["flags"]:
        return "admin"
    if annotations["auth_required"]:
        return "user"
    return "public"


def classify_path(path: str) -> str:
    """URL-shape hint for triage."""
    # Own-resource: anything scoped to the calling user's identity / state.
    if "/@me" in path or "/me/" in path or path.endswith("/me"):
        return "own_resource"
    # Caller's friend graph + push tokens are all caller-implicit, like Discord.
    if path.startswith(("/api/v1/friends", "/api/v1/push/")):
        return "own_resource"
    if path.startswith("/api/v1/admin/"):
        return "admin"
    if path.startswith("/api/v1/auth/"):
        return "auth"
    if path.startswith("/api/v1/federation/"):
        return "federation"
    if path.startswith("/api/v1/mesh/"):
        return "mesh"
    if path in {"/api/v1/health", "/api/v1/gateway", "/api/v1/unfurl"}:
        return "infrastructure"
    return "generic"


def needs_review(route: dict) -> bool:
    """An auth-required, generic-path route with no detected gate of any kind."""
    if route["auth"] != "user":
        return False
    if route["path_kind"] != "generic":
        return False
    if route["permissions"]:
        return False
    if route["flags"]:
        return False
    return True


def build_inventory() -> dict:
    visited: set[Path] = set()
    for entry in ENTRY_POINTS:
        walk_modules(entry, visited)
    files = sorted(visited)
    handlers = index_handlers(files)
    raw_routes = collect_routes(files)

    inventory: list[dict] = []
    seen: set[tuple[str, str]] = set()
    for r in raw_routes:
        for method, handler_ref in r["methods_handlers"]:
            key = (method, r["path"])
            if key in seen:
                continue
            seen.add(key)
            resolved = resolve_handler(handler_ref, ROOT / r["declared_in"], handlers)
            if resolved is None:
                annotations = {
                    "permissions": [],
                    "inspects_effective": [],
                    "flags": [],
                    "auth_required": False,
                }
                handler_file = None
            else:
                handler_file, body = resolved
                annotations = extract_perms(body)
            entry = {
                "method": method,
                "path": r["path"],
                "handler": handler_ref,
                "handler_file": str(handler_file.relative_to(ROOT)) if handler_file else None,
                "declared_in": r["declared_in"],
                "auth": auth_class(annotations),
                "path_kind": classify_path(r["path"]),
                **annotations,
            }
            entry["needs_review"] = needs_review(entry)
            inventory.append(entry)
    inventory.sort(key=lambda x: (x["path"], x["method"]))

    method_counts = Counter(r["method"] for r in inventory)
    auth_counts = Counter(r["auth"] for r in inventory)
    path_kind_counts = Counter(r["path_kind"] for r in inventory)
    perm_counts: Counter[str] = Counter()
    for r in inventory:
        for p in r["permissions"]:
            perm_counts[p] += 1
    unresolved = [r for r in inventory if r["handler_file"] is None]
    review_list = [r for r in inventory if r["needs_review"]]

    return {
        "schema_version": 2,
        "generated_at": _dt.date.today().isoformat(),
        "source": {
            "entry_points": [str(p.relative_to(ROOT)) for p in ENTRY_POINTS],
            "modules_walked": len(files),
        },
        "summary": {
            "total_routes": len(inventory),
            "by_method": dict(sorted(method_counts.items())),
            "by_auth_class": dict(sorted(auth_counts.items())),
            "by_path_kind": dict(sorted(path_kind_counts.items())),
            "by_permission": dict(sorted(perm_counts.items())),
            "unresolved_handlers": len(unresolved),
            "needs_review": len(review_list),
        },
        "needs_review_routes": [
            {"method": r["method"], "path": r["path"], "handler_file": r["handler_file"]}
            for r in review_list
        ],
        "routes": inventory,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--fail-on-unresolved",
        action="store_true",
        help="Exit non-zero when any handler reference cannot be resolved.",
    )
    args = parser.parse_args()

    inv = build_inventory()
    out_path = OUT if OUT.is_absolute() else ROOT / OUT
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(inv, indent=2) + "\n", encoding="utf-8")
    summary = inv["summary"]
    print(
        f"Wrote {out_path.relative_to(ROOT)} "
        f"({summary['total_routes']} routes, "
        f"{summary['unresolved_handlers']} unresolved)",
        file=sys.stderr,
    )
    if args.fail_on_unresolved and summary["unresolved_handlers"] > 0:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
