#!/usr/bin/env python3
"""Fail-closed freshness gate for primitive durability proof documentation.

The master contract requires every current ``CompiledNodeKind`` primitive to
document the journal events that prove completion.  This script compares the
production enum against the table in ``docs/storage-journal.md`` and rejects
missing, extra, or empty rows.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TYPES_PATH = ROOT / "crates/vb_core/src/workflow/types.rs"
DOC_PATH = ROOT / "docs/storage-journal.md"
START_MARKER = "<!-- BEGIN PRIMITIVE DURABILITY PROOF MATRIX -->"
END_MARKER = "<!-- END PRIMITIVE DURABILITY PROOF MATRIX -->"


class CheckError(Exception):
    """Typed script failure rendered as a clear diagnostic."""


def compiled_node_kind_variants(text: str) -> list[str]:
    enum_match = re.search(r"pub\s+enum\s+CompiledNodeKind\s*\{", text)
    if enum_match is None:
        raise CheckError("CompiledNodeKind enum not found")

    body = text[enum_match.end() :]
    depth = 1
    variants: list[str] = []
    seen: set[str] = set()
    for raw_line in body.splitlines():
        line = raw_line.split("//", 1)[0].strip()
        if depth == 1:
            variant_match = re.match(r"([A-Z][A-Za-z0-9_]*)\b", line)
            if variant_match is not None:
                variant = variant_match.group(1)
                if variant not in seen:
                    variants.append(variant)
                    seen.add(variant)
        depth += raw_line.count("{")
        depth -= raw_line.count("}")
        if depth == 0:
            return variants

    raise CheckError("CompiledNodeKind enum did not terminate")


def matrix_rows(text: str) -> dict[str, tuple[str, str, str]]:
    try:
        start = text.index(START_MARKER) + len(START_MARKER)
        end = text.index(END_MARKER, start)
    except ValueError as error:
        raise CheckError("primitive durability proof matrix markers missing") from error

    rows: dict[str, tuple[str, str, str]] = {}
    for line_number, line in enumerate(text[start:end].splitlines(), start=1):
        stripped = line.strip()
        if not stripped.startswith("|") or "`" not in stripped:
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if len(cells) != 4:
            raise CheckError(f"matrix row {line_number} has {len(cells)} columns, expected 4")
        primitive = cells[0].strip("`")
        if primitive == "Primitive":
            continue
        if not re.match(r"^[A-Z][A-Za-z0-9_]*$", primitive):
            continue
        if primitive in rows:
            raise CheckError(f"duplicate primitive matrix row: {primitive}")
        rows[primitive] = (cells[1], cells[2], cells[3])
    return rows


def validate_matrix(variants: list[str], rows: dict[str, tuple[str, str, str]]) -> None:
    variant_set = set(variants)
    row_set = set(rows)
    missing = [variant for variant in variants if variant not in row_set]
    extra = sorted(row_set - variant_set)
    errors: list[str] = []
    if missing:
        errors.append("missing primitive rows: " + ", ".join(missing))
    if extra:
        errors.append("extra primitive rows: " + ", ".join(extra))
    for primitive in variants:
        if primitive not in rows:
            continue
        completion_events, recovery_proof, durable_gate = rows[primitive]
        empty_columns = [
            name
            for name, value in (
                ("completion journal events", completion_events),
                ("recovery proof", recovery_proof),
                ("VerificationProof.durable gate", durable_gate),
            )
            if value.strip() in {"", "TBD", "TODO", "-"}
        ]
        if empty_columns:
            errors.append(f"{primitive} has empty columns: {', '.join(empty_columns)}")
    if errors:
        raise CheckError("; ".join(errors))


def run_check(types_path: Path, doc_path: Path) -> None:
    variants = compiled_node_kind_variants(types_path.read_text(encoding="utf-8"))
    rows = matrix_rows(doc_path.read_text(encoding="utf-8"))
    validate_matrix(variants, rows)
    print(
        "primitive-durability-doc: OK "
        f"variants={len(variants)} rows={len(rows)} source={types_path} doc={doc_path}"
    )


def run_self_test() -> None:
    enum_text = """
pub enum CompiledNodeKind {
    Alpha,
    Beta { value: u8 },
    Gamma {
        value: u8,
    },
}
"""
    doc_text = f"""
{START_MARKER}
| Primitive | Completion journal events | Recovery proof | `VerificationProof.durable` gate |
| --- | --- | --- | --- |
| `Alpha` | StepStarted + StepSucceeded | replay alpha | strict proof |
| `Beta` | StepStarted + SlotWrittenEvent + StepSucceeded | replay beta | strict proof |
| `Gamma` | StepStarted + RunFinished | replay gamma | strict proof |
{END_MARKER}
"""
    variants = compiled_node_kind_variants(enum_text)
    rows = matrix_rows(doc_text)
    validate_matrix(variants, rows)

    broken_doc = doc_text.replace("| `Gamma` | StepStarted + RunFinished | replay gamma | strict proof |\n", "")
    try:
        validate_matrix(variants, matrix_rows(broken_doc))
    except CheckError as error:
        if "missing primitive rows: Gamma" not in str(error):
            raise
    else:
        raise CheckError("self-test expected missing Gamma row to fail")
    print("primitive-durability-doc: self-test OK")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--types", type=Path, default=TYPES_PATH)
    parser.add_argument("--doc", type=Path, default=DOC_PATH)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.self_test:
            run_self_test()
        else:
            run_check(args.types, args.doc)
    except CheckError as error:
        print(f"primitive-durability-doc: ERROR: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
