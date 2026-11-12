#!/usr/bin/env python3
"""Fail-closed vb-jpq7 closure evidence checker."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


REQUIRED_ROW_FIELDS = (
    "bead_id",
    "command",
    "cwd",
    "commit_sha",
    "tool_version",
    "timestamp",
    "raw_log_path",
    "stdout_summary",
    "stderr_summary",
    "exit_code",
    "status",
    "evidence_kind",
)
REJECTED_MARKERS = (
    "summary-only",
    "summary only",
    "cached-only",
    "cached only",
    "skipped-only",
    "skipped only",
    "subagent-only",
    "subagent only",
    "delegated-only",
    "delegated only",
)
SHA_RE = re.compile(r"^[0-9a-f]{7,64}$")


@dataclass(frozen=True)
class Issue:
    bead_id: str
    status: str
    close_reason: str
    notes: str


@dataclass(frozen=True)
class ManifestRow:
    row_number: int
    data: dict[str, Any]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="fail closed when closed vb-jpq7 children lack raw manifest evidence"
    )
    parser.add_argument("--parent", default="vb-jpq7")
    parser.add_argument("--manifest", default=".beads/vb-jpq7/closure-evidence-manifest.jsonl")
    parser.add_argument("--bd-workdir", default=".")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return run_self_test()
    root = Path.cwd()
    issues_result = load_issues(args.parent, None, Path(args.bd_workdir))
    if not isinstance(issues_result, list):
        print(f"CHECK FAILED: {issues_result}", file=sys.stderr)
        return 2
    if not issues_result:
        print("CHECK FAILED: bd children returned no vb-jpq7 children", file=sys.stderr)
        return 2
    rows, row_errors = load_manifest(resolve_path(root, args.manifest))
    failures = list(row_errors)
    failures.extend(validate_closed_children(root, issues_result, rows))
    if failures:
        print("VB_JPQ7_CLOSURE_EVIDENCE_FAIL")
        for failure in failures:
            print(f"- {failure}")
        return 1
    closed_count = len([issue for issue in issues_result if issue.status == "closed"])
    print(f"VB_JPQ7_CLOSURE_EVIDENCE_PASS closed_children={closed_count}")
    return 0


def resolve_path(root: Path, raw_path: str) -> Path:
    path = Path(raw_path)
    if path.is_absolute():
        return path
    return root.joinpath(path)


def load_issues(parent: str, fixture: str | None, bd_workdir: Path) -> list[Issue] | str:
    if fixture is not None:
        text = Path(fixture).read_text(encoding="utf-8")
    else:
        # `bd children` is a thin alias for `bd list --parent <id> --status all`
        # but its stub does not forward the `-n/--limit` flag. With vb-jpq7
        # carrying 53 closed children, the default 50-row pagination cap hid
        # vb-jpq7.3, vb-jpq7.4, and vb-jpq7.53 from this checker, so the
        # corresponding manifest rows were never validated. Calling `bd list`
        # directly with `-n 0` (unlimited) restores the full closed-children
        # set. The output schema is identical to `bd children --json`.
        result = subprocess.run(
            ["bd", "list", "--parent", parent, "--status", "all", "--json", "-n", "0"],
            cwd=bd_workdir,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if result.returncode != 0:
            return f"bd list failed exit={result.returncode} stderr={compact(result.stderr)}"
        text = result.stdout
    try:
        data = json.loads(text)
    except json.JSONDecodeError as exc:
        return f"children JSON is invalid: {exc}"
    if not isinstance(data, list):
        return "children JSON must be a list"
    issues: list[Issue] = []
    for raw in data:
        if not isinstance(raw, dict):
            return "children JSON contains a non-object issue"
        bead_id = string_field(raw, "id")
        status = string_field(raw, "status")
        if bead_id is None or status is None:
            return "child issue is missing id or status"
        issues.append(
            Issue(
                bead_id=bead_id,
                status=status,
                close_reason=string_field(raw, "close_reason") or "",
                notes=string_field(raw, "notes") or "",
            )
        )
    return issues


def load_manifest(path: Path) -> tuple[list[ManifestRow], list[str]]:
    if not path.exists():
        return [], [f"manifest missing: {path}"]
    rows: list[ManifestRow] = []
    errors: list[str] = []
    with path.open("r", encoding="utf-8") as handle:
        for number, line in enumerate(handle, start=1):
            stripped = line.strip()
            if not stripped:
                continue
            try:
                data = json.loads(stripped)
            except json.JSONDecodeError as exc:
                errors.append(f"{path}:{number}: invalid JSONL row: {exc}")
                continue
            if not isinstance(data, dict):
                errors.append(f"{path}:{number}: row is not an object")
                continue
            rows.append(ManifestRow(number, data))
    if not rows:
        errors.append(f"manifest has no evidence rows: {path}")
    return rows, errors


def validate_closed_children(root: Path, issues: list[Issue], rows: list[ManifestRow]) -> list[str]:
    failures: list[str] = []
    by_bead: dict[str, list[ManifestRow]] = {}
    seen_followups: dict[tuple[str, str], int] = {}
    for row in rows:
        bead_id = string_field(row.data, "bead_id")
        if bead_id is not None:
            by_bead.setdefault(bead_id, []).append(row)
        failures.extend(validate_row_shape(root, row))
        failures.extend(validate_resolution(row, seen_followups))
    for issue in issues:
        if issue.status != "closed":
            continue
        issue_rows = by_bead.get(issue.bead_id, [])
        if not issue_rows:
            failures.append(f"{issue.bead_id}: closed child lacks manifest row")
            continue
        if not any(row_closes_or_links_followup(row.data) for row in issue_rows):
            failures.append(
                f"{issue.bead_id}: no passing closure row or waiver/split-linked failure row"
            )
        closure_text = f"{issue.close_reason}\n{issue.notes}".lower()
        if (marker := rejected_marker(closure_text)) is not None:
            failures.append(f"{issue.bead_id}: closure prose contains rejected evidence marker {marker}")
    return failures


def validate_row_shape(root: Path, row: ManifestRow) -> list[str]:
    failures: list[str] = []
    data = row.data
    for field in REQUIRED_ROW_FIELDS:
        if missing(data.get(field)):
            failures.append(f"manifest row {row.row_number}: missing required field {field}")
    commit = string_field(data, "commit_sha")
    if commit is not None and SHA_RE.fullmatch(commit) is None:
        failures.append(f"manifest row {row.row_number}: invalid commit_sha {commit}")
    timestamp = string_field(data, "timestamp")
    if timestamp is not None and not valid_timestamp(timestamp):
        failures.append(f"manifest row {row.row_number}: timestamp is not RFC3339 UTC")
    if not isinstance(data.get("exit_code"), int):
        failures.append(f"manifest row {row.row_number}: exit_code must be integer")
    cwd = string_field(data, "cwd")
    if cwd is not None and data.get("exit_code") == 0 and not Path(cwd).is_dir():
        failures.append(f"manifest row {row.row_number}: cwd is not a directory {cwd}")
    raw_log = string_field(data, "raw_log_path")
    if raw_log is not None:
        raw_path = resolve_path(root, raw_log)
        if not raw_path.is_file():
            failures.append(f"manifest row {row.row_number}: raw_log_path missing file {raw_path}")
        else:
            failures.extend(validate_raw_log(row, raw_path))
    combined = "\n".join(str(value) for value in data.values()).lower()
    if (marker := rejected_marker(combined)) is not None:
        failures.append(f"manifest row {row.row_number}: rejected evidence marker present: {marker}")
    return failures


def row_is_closure_pass(data: dict[str, Any]) -> bool:
    if data.get("exit_code") != 0:
        return False
    status = string_field(data, "status")
    evidence_kind = string_field(data, "evidence_kind")
    if status is None or evidence_kind is None:
        return False
    return status.upper() in {"PASS", "CLOSURE_PASS", "EXECUTED_PASS"} and evidence_kind in {
        "raw-command",
        "command",
        "raw-log",
        "live-command",
    }


def row_closes_or_links_followup(data: dict[str, Any]) -> bool:
    if row_is_closure_pass(data):
        return True
    if data.get("exit_code") == 0:
        return False
    resolution = string_field(data, "resolution_kind")
    rationale = string_field(data, "resolution_rationale")
    if missing(rationale):
        return False
    if resolution == "approved_waiver":
        return not missing(string_field(data, "waiver_id"))
    if resolution == "split_followup":
        return not missing(string_field(data, "split_bead_id"))
    return False


def validate_resolution(
    row: ManifestRow, seen_followups: dict[tuple[str, str], int]
) -> list[str]:
    data = row.data
    if data.get("exit_code") == 0:
        return []
    failures: list[str] = []
    resolution = string_field(data, "resolution_kind")
    rationale = string_field(data, "resolution_rationale")
    if resolution not in {"split_followup", "approved_waiver"}:
        failures.append(f"manifest row {row.row_number}: nonzero row lacks valid resolution_kind")
    if missing(rationale):
        failures.append(f"manifest row {row.row_number}: nonzero row lacks resolution_rationale")
    if resolution == "split_followup":
        split = string_field(data, "split_bead_id")
        if missing(split):
            failures.append(f"manifest row {row.row_number}: split_followup lacks split_bead_id")
        else:
            key = (split, rationale or "")
            if key in seen_followups:
                failures.append(
                    f"manifest row {row.row_number}: duplicate split_followup rationale also used on row {seen_followups[key]}"
                )
            seen_followups[key] = row.row_number
    if resolution == "approved_waiver" and missing(string_field(data, "waiver_id")):
        failures.append(f"manifest row {row.row_number}: approved_waiver lacks waiver_id")
    return failures


def string_field(data: dict[str, Any], field: str) -> str | None:
    value = data.get(field)
    if isinstance(value, str):
        return value
    return None


def missing(value: Any) -> bool:
    if value is None:
        return True
    if isinstance(value, str):
        return value.strip() == ""
    return False


def valid_timestamp(value: str) -> bool:
    if not value.endswith("Z"):
        return False
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return False
    return parsed.tzinfo is not None and parsed.astimezone(timezone.utc).tzinfo is not None


def validate_raw_log(row: ManifestRow, path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8", errors="replace")
    if text.strip() == "":
        return [f"manifest row {row.row_number}: raw_log_path is empty {path}"]
    failures: list[str] = []
    for field in ("command", "cwd", "timestamp", "exit_code"):
        expected = f"{field}: {row.data.get(field)}"
        if expected not in text:
            failures.append(
                f"manifest row {row.row_number}: raw_log_path lacks provenance binding {expected}"
            )
    return failures


def contains_rejected_marker(text: str) -> bool:
    return rejected_marker(text) is not None


def rejected_marker(text: str) -> str | None:
    for marker in REJECTED_MARKERS:
        if marker in text:
            return marker
    return None


def compact(text: str) -> str:
    return " ".join(text.split())[:500]


def run_self_test() -> int:
    root = Path.cwd() / ".evidence/vb-jpq7.48/scratch/self-test"
    root.mkdir(parents=True, exist_ok=True)
    fixture = make_self_test_fixture(root)
    failures = run_self_test_cases(fixture)
    if not failures:
        print("SELF_TEST_PASS")
        return 0
    print("SELF_TEST_FAIL", file=sys.stderr)
    for failure in failures:
        print(f"- {failure}", file=sys.stderr)
    return 1


@dataclass(frozen=True)
class SelfTestFixture:
    root: Path
    children: Path
    manifest: Path
    good_row: dict[str, Any]


def make_self_test_fixture(root: Path) -> SelfTestFixture:
    raw = root / "raw.log"
    children = root / "children.json"
    children.write_text(json.dumps(closed_child_fixture()), encoding="utf-8")
    row = valid_row(root, raw)
    raw.write_text(raw_log_text(row), encoding="utf-8")
    return SelfTestFixture(root, children, root / "manifest.jsonl", row)


def closed_child_fixture() -> list[dict[str, str]]:
    return [
        {"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"},
        {"id": "vb-jpq7.2", "status": "open"},
    ]


def valid_row(root: Path, raw_log: Path) -> dict[str, Any]:
    return {
        "bead_id": "vb-jpq7.1",
        "command": "cargo test -p xtask",
        "cwd": str(root),
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-05-23T00:00:00Z",
        "raw_log_path": str(raw_log),
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command",
    }


def raw_log_text(row: dict[str, Any]) -> str:
    return (
        f"command: {row['command']}\n"
        f"cwd: {row['cwd']}\n"
        f"timestamp: {row['timestamp']}\n"
        f"exit_code: {row['exit_code']}\n"
        "stdout: command output observed\n"
        "stderr: command error stream observed\n"
    )


def run_self_test_cases(fixture: SelfTestFixture) -> list[str]:
    failures: list[str] = []
    failures.extend(expect_code("valid raw row", fixture, [fixture.good_row], 0))
    failures.extend(expect_bad_marker_cases(fixture))
    failures.extend(expect_shape_failure_cases(fixture))
    failures.extend(expect_manifest_parse_failures(fixture))
    failures.extend(expect_split_linked_failure(fixture))
    failures.extend(expect_resolution_failure_cases(fixture))
    return failures


def expect_bad_marker_cases(fixture: SelfTestFixture) -> list[str]:
    failures: list[str] = []
    for marker in ("subagent-only", "summary-only", "cached-only", "skipped-only"):
        row = mutate_row(fixture.good_row, stdout_summary=marker)
        failures.extend(expect_failure(f"reject {marker}", fixture, [row], marker))
    return failures


def expect_shape_failure_cases(fixture: SelfTestFixture) -> list[str]:
    cases = [
        ("required field", row_without(fixture.good_row, "command"), "missing required field command"),
        ("bad sha", mutate_row(fixture.good_row, commit_sha="not-a-sha"), "invalid commit_sha"),
        ("bad exit", mutate_row(fixture.good_row, exit_code="0"), "exit_code must be integer"),
        ("bad time", mutate_row(fixture.good_row, timestamp="yesterday"), "timestamp is not"),
        ("non utc", mutate_row(fixture.good_row, timestamp="2026-05-23T01:00:00+01:00"), "timestamp is not"),
        ("bad cwd", mutate_row(fixture.good_row, cwd=str(fixture.root / "missing-dir")), "cwd is not"),
        ("missing status", row_without(fixture.good_row, "status"), "missing required field status"),
        ("missing evidence kind", row_without(fixture.good_row, "evidence_kind"), "missing required field evidence_kind"),
        ("missing raw", mutate_row(fixture.good_row, raw_log_path=str(fixture.root / "missing.log")), "raw_log_path missing"),
        ("empty raw", empty_log_row(fixture), "raw_log_path is empty"),
        ("no matching child", mutate_row(fixture.good_row, bead_id="vb-jpq7.9"), "closed child lacks"),
    ]
    failures: list[str] = []
    for label, row, expected in cases:
        failures.extend(expect_failure(label, fixture, [row], expected))
    return failures


def expect_manifest_parse_failures(fixture: SelfTestFixture) -> list[str]:
    fixture.manifest.write_text("{not-json}\n", encoding="utf-8")
    failures = expect_current_failure("invalid json", fixture, "invalid JSONL row")
    fixture.manifest.write_text("", encoding="utf-8")
    failures.extend(expect_current_failure("empty manifest", fixture, "manifest has no evidence rows"))
    return failures


def empty_log_row(fixture: SelfTestFixture) -> dict[str, Any]:
    empty = fixture.root / "empty.log"
    empty.write_text("", encoding="utf-8")
    return mutate_row(fixture.good_row, raw_log_path=str(empty))


def expect_split_linked_failure(fixture: SelfTestFixture) -> list[str]:
    raw = fixture.root / "split.log"
    row = mutate_row(
        fixture.good_row,
        exit_code=1,
        status="FAIL",
        split_bead_id="vb-rud5",
        resolution_kind="split_followup",
        resolution_rationale="self-test split followup for vb-jpq7.1",
        raw_log_path=str(raw),
    )
    raw.write_text(raw_log_text(row), encoding="utf-8")
    return expect_code("split-linked nonzero", fixture, [row], 0)


def expect_resolution_failure_cases(fixture: SelfTestFixture) -> list[str]:
    row = mutate_row(fixture.good_row, exit_code=1, status="FAIL")
    failures = expect_failure("missing resolution", fixture, [row], "resolution_kind")
    first = split_row(fixture, "vb-jpq7.1", "same reason", "first-split.log")
    second = split_row(fixture, "vb-jpq7.2", "same reason", "second-split.log")
    failures.extend(expect_failure("duplicate split", fixture, [first, second], "duplicate split_followup"))
    no_rationale = mutate_row(first, resolution_rationale="")
    failures.extend(expect_failure("missing rationale", fixture, [no_rationale], "resolution_rationale"))
    return failures


def split_row(fixture: SelfTestFixture, bead_id: str, rationale: str, name: str) -> dict[str, Any]:
    raw = fixture.root / name
    row = mutate_row(
        fixture.good_row,
        bead_id=bead_id,
        exit_code=1,
        status="FAIL",
        split_bead_id="vb-rud5",
        resolution_kind="split_followup",
        resolution_rationale=rationale,
        raw_log_path=str(raw),
    )
    raw.write_text(raw_log_text(row), encoding="utf-8")
    return row


def mutate_row(row: dict[str, Any], **updates: Any) -> dict[str, Any]:
    changed = dict(row)
    changed.update(updates)
    return changed


def row_without(row: dict[str, Any], key: str) -> dict[str, Any]:
    changed = dict(row)
    changed.pop(key, None)
    return changed


def expect_failure(
    label: str, fixture: SelfTestFixture, rows: list[dict[str, Any]], expected: str
) -> list[str]:
    write_manifest(fixture.manifest, rows)
    return expect_current_failure(label, fixture, expected)


def expect_current_failure(label: str, fixture: SelfTestFixture, expected: str) -> list[str]:
    code, failures = run_checker_details(fixture.root, fixture.children, fixture.manifest)
    if code == 1 and any(expected in failure for failure in failures):
        return []
    return [f"{label}: expected failure containing {expected!r}, got {code} {failures}"]


def expect_code(
    label: str, fixture: SelfTestFixture, rows: list[dict[str, Any]], expected: int
) -> list[str]:
    write_manifest(fixture.manifest, rows)
    code, failures = run_checker_details(fixture.root, fixture.children, fixture.manifest)
    if code == expected:
        return []
    return [f"{label}: expected code {expected}, got {code} {failures}"]


def write_manifest(path: Path, rows: list[dict[str, Any]]) -> None:
    path.write_text("".join(f"{json.dumps(row)}\n" for row in rows), encoding="utf-8")


def run_checker_for_test(root: Path, children: Path, manifest: Path) -> int:
    code, _failures = run_checker_details(root, children, manifest)
    return code


def run_checker_details(root: Path, children: Path, manifest: Path) -> tuple[int, list[str]]:
    issues = load_issues("vb-jpq7", str(children), root)
    if not isinstance(issues, list):
        return 2, [issues]
    rows, row_errors = load_manifest(manifest)
    failures = list(row_errors)
    failures.extend(validate_closed_children(root, issues, rows))
    if failures:
        return 1, failures
    return 0, []


if __name__ == "__main__":
    sys.exit(main())
