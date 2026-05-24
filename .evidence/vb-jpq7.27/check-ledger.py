#!/usr/bin/env python3
"""Fail-closed validator for vb-jpq7.27 proof obligation ledger."""

import json
import pathlib
import sys


REQUIRED_FIELDS = {
    "obligation_id",
    "status",
    "artifact_path",
    "command",
    "cwd",
    "commit_sha",
    "tool_version",
    "timestamp_utc",
    "raw_log_path",
    "exit_code",
    "scope",
}

VALID_STATUSES = {"PASS", "FAIL", "NON_EVIDENCE", "BLOCKED"}


def main() -> int:
    base = pathlib.Path(__file__).resolve().parent
    ledger = base / "proof-obligation-ledger.jsonl"
    failures = []

    if not ledger.is_file():
        failures.append(f"missing ledger: {ledger}")
    else:
        with ledger.open("r", encoding="utf-8") as handle:
            for line_number, line in enumerate(handle, start=1):
                stripped = line.strip()
                if not stripped:
                    failures.append(f"line {line_number}: blank lines forbidden")
                    continue
                try:
                    row = json.loads(stripped)
                except json.JSONDecodeError as exc:
                    failures.append(f"line {line_number}: invalid json: {exc}")
                    continue

                missing = sorted(REQUIRED_FIELDS.difference(row.keys()))
                if missing:
                    failures.append(f"line {line_number}: missing fields {missing}")

                status = row.get("status")
                if status not in VALID_STATUSES:
                    failures.append(f"line {line_number}: invalid status {status!r}")

                raw_log = row.get("raw_log_path")
                if not raw_log:
                    failures.append(f"line {line_number}: raw_log_path is empty")
                else:
                    raw_path = (base.parent / raw_log).resolve()
                    if not raw_path.is_file():
                        failures.append(f"line {line_number}: missing raw log {raw_log}")

                evidence_class = row.get("evidence_class")
                if status == "PASS":
                    if evidence_class == "NON_EVIDENCE":
                        failures.append(f"line {line_number}: PASS row marked NON_EVIDENCE")
                    if row.get("exit_code") != 0:
                        failures.append(f"line {line_number}: PASS row has non-zero exit_code")
                    notes = str(row.get("notes", "")).lower()
                    if "placeholder" in notes or "stale summary" in notes:
                        failures.append(f"line {line_number}: PASS row notes indicate non-evidence")

                if status in {"FAIL", "BLOCKED"} and not row.get("child_bead"):
                    failures.append(f"line {line_number}: {status} row lacks child_bead")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1

    print("PASS: vb-jpq7.27 ledger is structurally valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
