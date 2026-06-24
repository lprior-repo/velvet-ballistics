#!/usr/bin/env bash
# **PO-vb-hbav-033**: CI error exhaustiveness check script.
#
# Compares fuzz oracle function bodies against production error enum definitions.
# Exits 0 when every checked oracle body mentions every current production
# variant. Exits non-zero when variants are missing or inputs cannot be parsed.
#
# Usage: bash scripts/check-error-exhaustiveness.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/.."

if ! command -v python3 >/dev/null 2>&1; then
    printf 'Missing required parser: python3\n' >&2
    exit 127
fi

printf '=== Checking fuzz harness error exhaustiveness ===\n'

python3 - "$ROOT_DIR" <<'PY'
from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path


def strip_comments(text: str) -> str:
    without_blocks = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"//.*", "", without_blocks)


def enum_body(text: str, enum_name: str) -> str:
    marker = re.search(rf"\benum\s+{re.escape(enum_name)}\s*{{", text)
    if marker is None:
        raise ValueError(f"enum {enum_name} not found")
    start = marker.end() - 1
    depth = 0
    for index in range(start, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[start + 1:index]
    raise ValueError(f"enum {enum_name} body is not closed")


@dataclass(frozen=True)
class Oracle:
    path: Path
    function: str


def braced_body(text: str, open_brace: int, label: str) -> str:
    depth = 0
    for index in range(open_brace, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[open_brace + 1:index]
    raise ValueError(f"{label} body is not closed")


def function_body(text: str, function_name: str) -> str:
    marker = re.search(rf"\bfn\s+{re.escape(function_name)}\s*(?:<[^>]*>)?\(", text)
    if marker is None:
        raise ValueError(f"function {function_name} not found")
    open_brace = text.find("{", marker.end())
    if open_brace < 0:
        raise ValueError(f"function {function_name} body not found")
    return braced_body(text, open_brace, f"function {function_name}")


def enum_variants(path: Path, enum_name: str) -> set[str]:
    text = strip_comments(path.read_text(encoding="utf-8"))
    body = enum_body(text, enum_name)
    variants: set[str] = set()
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        match = re.match(r"([A-Z][A-Za-z0-9_]*)\b(?:\s*[({,]|$)", stripped)
        if match is not None:
            variants.add(match.group(1))
    if not variants:
        raise ValueError(f"no variants parsed for {enum_name} in {path}")
    return variants


def mentioned_variants(path: Path, type_name: str, function_name: str) -> set[str]:
    text = strip_comments(path.read_text(encoding="utf-8"))
    body = function_body(text, function_name)
    return set(re.findall(rf"\b{re.escape(type_name)}::([A-Z][A-Za-z0-9_]*)\b", body))


def main() -> int:
    root = Path(sys.argv[1])
    checks = [
        (
            "JournalError",
            root / "crates/vb_storage/src/error/mod.rs",
            [
                Oracle(root / "fuzz/src/lib.rs", "assert_typed_journal_error"),
                Oracle(root / "fuzz/fuzz_targets/decode_record.rs", "assert_typed_journal_error"),
                Oracle(root / "fuzz/fuzz_targets/journal_decode.rs", "assert_typed_journal_error"),
                Oracle(
                    root / "fuzz/tests/proptest_journal_error_exhaustiveness.rs",
                    "assert_known_journal_error",
                ),
            ],
        ),
        (
            "IpcError",
            root / "crates/vb_ipc/src/error.rs",
            [Oracle(root / "fuzz/src/lib.rs", "assert_typed_ipc_error")],
        ),
        (
            "ValidationError",
            root / "crates/vb_validate/src/lib.rs",
            [Oracle(root / "fuzz/src/lib.rs", "assert_typed_validation_error")],
        ),
    ]

    failures: list[str] = []
    for type_name, enum_path, oracles in checks:
        try:
            variants = enum_variants(enum_path, type_name)
        except ValueError as error:
            failures.append(f"{type_name} enum parse failed in {enum_path}: {error}")
            continue
        for oracle in oracles:
            rel = oracle.path.relative_to(root)
            try:
                mentions = mentioned_variants(oracle.path, type_name, oracle.function)
            except ValueError as error:
                failures.append(f"{type_name} oracle parse failed in {rel}::{oracle.function}: {error}")
                continue
            missing = sorted(variants - mentions)
            if missing:
                failures.append(
                    f"{type_name} missing in {rel}::{oracle.function}: {', '.join(missing)}"
                )
            else:
                print(f"  OK {type_name} in {rel}::{oracle.function}: {len(variants)} variants")

    if failures:
        print("=== Error exhaustiveness failures ===", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1

    print("=== All error exhaustiveness checks passed ===")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
PY
