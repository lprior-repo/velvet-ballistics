#!/usr/bin/env python3
"""Validate the master document's v1 taint consistency contract.

The check is deterministic, local-only, and read-only.  It mirrors the
line-scoped stale joined-taint wording rejected by vb_doc::reconcile:
EvalExpr, BuildObject, BuildList, and Finish must describe joined data-flow
taint, not clean-only output.  It also rejects the vocabulary conflicts
guarded by the Rust reconciliation contract.
"""

from __future__ import annotations

import sys
import re
from pathlib import Path


NODE_RULES: tuple[tuple[str, tuple[str, ...]], ...] = (
    (
        "EvalExpr",
        (
            "Always Clean",
            "always Clean",
            "No taint join",
            "no taint join of expression operands",
        ),
    ),
    (
        "BuildObject",
        ("Always Clean", "always Clean", "no join of field taints"),
    ),
    (
        "BuildList",
        ("Always Clean", "always Clean", "no join of item taints"),
    ),
)

VOCABULARY_CONFLICTS: tuple[tuple[str, str], ...] = (
    ("Clean < Secret < DerivedFromSecret", "wrong lattice order"),
    ("Secret downgrades to Clean", "secret downgrade claim"),
    ("Private", "unknown taint term"),
)

CONTROL_FLOW_CONFLICTS: tuple[str, ...] = (
    "tracks secret branch-condition taint",
    "tracks branch-condition taint",
)

FINISH_REQUIRED = "Finished(SlotValue, Taint)"

FINISH_CONFLICTS: tuple[tuple[str, str], ...] = (
    (r"Finished\(SlotValue\)(?!,)", "stale finish signal missing taint"),
)


def usage() -> int:
    print(
        "usage: python scripts/check-doc-taint-consistency.py "
        "velvet-ballistics-MASTER.md",
        file=sys.stderr,
    )
    return 2


def read_doc(path_text: str) -> tuple[int, str]:
    path = Path(path_text)
    if path.name != "velvet-ballistics-MASTER.md":
        return 2, f"error: expected velvet-ballistics-MASTER.md, got {path}"
    if not path.is_file():
        return 2, f"error: document not found: {path}"
    try:
        return 0, path.read_text(encoding="utf-8")
    except OSError as error:
        return 2, f"error: cannot read {path}: {error}"
    except UnicodeDecodeError as error:
        return 2, f"error: invalid utf-8 in {path}: {error}"


def stale_node_findings(text: str) -> list[str]:
    findings: list[str] = []
    for line_number, line in enumerate(text.splitlines(), start=1):
        for node, phrases in NODE_RULES:
            if node in line:
                findings.extend(
                    format_finding(line_number, node, phrase)
                    for phrase in phrases
                    if phrase in line
                )
            if node in line and "write_slot" in line and "not write_slot_with_taint" in line:
                findings.append(format_finding(line_number, node, "write_slot"))
    return findings


def vocabulary_findings(text: str) -> list[str]:
    findings = [reason for phrase, reason in VOCABULARY_CONFLICTS if phrase in text]
    lower = text.lower()
    findings.extend(
        f"control-flow taint conflation: {phrase}"
        for phrase in CONTROL_FLOW_CONFLICTS
        if phrase in lower
    )
    return findings


def finish_findings(text: str) -> list[str]:
    findings = [reason for pattern, reason in FINISH_CONFLICTS if re.search(pattern, text)]
    findings.extend(finish_rejection_findings(text))
    if FINISH_REQUIRED not in text:
        findings.append("missing finish signal taint wording")
    return findings


def finish_rejection_findings(text: str) -> list[str]:
    return [
        "finish rejection contradiction"
        for line in text.splitlines()
        if is_finish_rejection_contradiction(line)
    ]


def is_finish_rejection_contradiction(line: str) -> bool:
    lower = line.lower()
    has_taint = "secret" in lower or "derivedfromsecret" in lower
    has_reject = "reject" in lower or "rejection" in lower
    allowed = "no rejection" in lower or "does not reject" in lower or "not reject" in lower
    return "finish" in lower and has_taint and has_reject and not allowed


def format_finding(line_number: int, node: str, phrase: str) -> str:
    return f"line {line_number}: {node} stale taint wording: {phrase}"


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        return usage()

    status, result = read_doc(argv[1])
    if status != 0:
        print(result, file=sys.stderr)
        return status

    findings = stale_node_findings(result) + vocabulary_findings(result) + finish_findings(result)
    if findings:
        print("doc taint consistency: FAIL", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1

    print("doc taint consistency: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
