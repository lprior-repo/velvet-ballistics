#!/usr/bin/env python3
"""State 8 preliminary audit harness for vb-scxh.

This script is intentionally *not* a State 11/12 evidence producer.  It creates
deterministic failing-first preflight output so later evidence capture can be
replayed without subagent narrative, stale prose, or accidental close/unblock
approval.
"""

from __future__ import annotations

import argparse
import shlex
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path("/home/lewis/src/vb-scxh")
BEAD = ROOT / ".beads" / "vb-scxh"
GVMT = ROOT / ".beads" / "vb-gvmt"
SAFETY_COMMAND = (
    "git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle "
    "&& git show-ref rescue-vb-scxh-ci-green-20260513T030158Z"
)


@dataclass(frozen=True)
class LaneResult:
    name: str
    obligation_ids: tuple[str, ...]
    status: str
    command: str
    expected: str
    actual: str
    error: str | None


def run_shell(command: str) -> tuple[int, str, str]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        shell=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    return completed.returncode, completed.stdout, completed.stderr


def file_nonempty(path: Path) -> bool:
    return path.is_file() and path.stat().st_size > 0


def read_text(path: Path) -> str:
    if not path.is_file():
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def workspace_lane() -> LaneResult:
    code, out, err = run_shell("pwd -P")
    actual = f"exit={code}\nstdout={out!r}\nstderr={err!r}"
    status = "PASS_PRELIM" if code == 0 and out.strip() == str(ROOT) else "RED_PRELIM"
    return LaneResult(
        "workspace_path",
        ("PATH-SCXH-001", "SCOPEWRITE-SCXH-001", "ERR-SCXH-001"),
        status,
        "pwd -P",
        f"stdout exactly {ROOT}",
        actual,
        None if status == "PASS_PRELIM" else "Error::WrongWorkspace",
    )


def approval_lane() -> LaneResult:
    files = [
        BEAD / "test-plan.md",
        BEAD / "proof-review.md",
        BEAD / "contract-verification-review.md",
    ]
    missing = [str(path.relative_to(ROOT)) for path in files if not file_nonempty(path)]
    proof_review = read_text(BEAD / "proof-review.md")
    contract_review = read_text(BEAD / "contract-verification-review.md")
    missing_status = []
    if "\nSTATUS: APPROVED\n" not in f"\n{proof_review}":
        missing_status.append("proof-review.md STATUS: APPROVED")
    if "\nSTATUS: APPROVED\n" not in f"\n{contract_review}":
        missing_status.append("contract-verification-review.md STATUS: APPROVED")
    failures = missing + missing_status
    status = "PASS_PRELIM" if not failures else "RED_PRELIM"
    return LaneResult(
        "approved_inputs",
        ("ART-SCXH-001",),
        status,
        "test -s approved inputs and grep STATUS: APPROVED",
        "test-plan non-empty and both upstream reviews approved",
        "failures=" + repr(failures),
        None if status == "PASS_PRELIM" else "Error::MissingRecoveryInput",
    )


def artifact_presence_lane() -> LaneResult:
    paths = [
        BEAD / "STATE.md",
        BEAD / "delivery-scope.jsonl",
        BEAD / "codebase-map.md",
        GVMT / "moon-ci-or-static-scan-report.md",
        GVMT / "formal-verification-report.md",
        GVMT / "verification-ledger.jsonl",
        GVMT / "parity-test-report.md",
        GVMT / "mutation-report.md",
    ]
    missing = [str(path.relative_to(ROOT)) for path in paths if not file_nonempty(path)]
    status = "PASS_PRELIM" if not missing else "RED_PRELIM"
    return LaneResult(
        "artifact_presence",
        ("ART-SCXH-001", "ERR-SCXH-002"),
        status,
        " && ".join(f"test -s {shlex.quote(str(path.relative_to(ROOT)))}" for path in paths),
        "all required State 1/2 and referenced vb-gvmt artifacts are non-empty",
        "missing_or_empty=" + repr(missing),
        None if status == "PASS_PRELIM" else "Error::MissingRecoveryInput",
    )


def safety_lane() -> LaneResult:
    code, out, err = run_shell(SAFETY_COMMAND)
    actual = f"exit={code}\nstdout={out}\nstderr={err}"
    status = "PASS_PRELIM" if code == 0 else "RED_PRELIM"
    return LaneResult(
        "safety_anchor_preflight",
        ("SAFETY-SCXH-001", "ERR-SCXH-006"),
        status,
        SAFETY_COMMAND,
        "bundle verifies and rescue bookmark/ref resolves",
        actual,
        None if status == "PASS_PRELIM" else "Error::SafetyAnchorMissing; failure_classification=BLOCK_LOCAL",
    )


def moon_lane() -> LaneResult:
    report = GVMT / "moon-ci-or-static-scan-report.md"
    text = read_text(report)
    required = [
        "moon ci",
        "Status: PASS",
        "Tasks: 19 completed",
        "8276 tests run: 8276 passed",
        "Runtime:",
    ]
    missing = [marker for marker in required if marker not in text]
    artifact_path_markers = (
        "Artifact path:",
        "Artifact:",
        "artifacts/",
        ".beads/vb-scxh/moon-ci-evidence-audit.md",
        ".beads/vb-gvmt/moon-ci-or-static-scan-report.md",
    )
    fresh_rerun_markers = (
        "Fresh rerun:",
        "Fresh re-run:",
        "fresh_rerun=true",
        "fresh-rerun=true",
        "rerun_utc=",
        "Generated UTC:",
    )
    if not any(marker in text for marker in artifact_path_markers):
        missing.append("artifact path evidence marker")
    if not any(marker in text for marker in fresh_rerun_markers):
        missing.append("fresh rerun marker")
    status = "PASS_PRELIM" if file_nonempty(report) and not missing else "RED_PRELIM"
    return LaneResult(
        "moon_ci_marker_audit",
        ("CI-SCXH-001", "ERR-SCXH-003"),
        status,
        "audit .beads/vb-gvmt/moon-ci-or-static-scan-report.md markers; require artifact path and fresh rerun marker before PASS_PRELIM; rerun moon ci in State 11 if stale/missing",
        "raw command moon ci, PASS, 19 completed tasks, 8276/8276 passed, runtime marker, artifact path evidence, fresh rerun marker",
        "missing_markers=" + repr(missing),
        None if status == "PASS_PRELIM" else "Error::MissingRawEvidence",
    )


def mutation_lane() -> LaneResult:
    report = read_text(GVMT / "mutation-report.md")
    ledger = read_text(GVMT / "verification-ledger.jsonl")
    missing = []
    for marker in ("FAIL_UNVIABLE", "DEFERRED", "35/35 unviable"):
        if marker not in report:
            missing.append(f"mutation-report missing {marker}")
    for marker in ("FAIL_UNVIABLE", "35/35 unviable"):
        if marker not in ledger:
            missing.append(f"verification-ledger missing {marker}")
    forbidden = []
    if "mutation adequacy" in report and "not mutation adequacy evidence" not in report:
        forbidden.append("ambiguous mutation adequacy wording")
    status = "PASS_PRELIM" if not missing and not forbidden else "RED_PRELIM"
    return LaneResult(
        "mutation_marker_audit",
        ("MUT-SCXH-001", "TLA-SCXH-003", "ERR-SCXH-007"),
        status,
        "audit .beads/vb-gvmt/mutation-report.md and verification-ledger.jsonl",
        "FAIL_UNVIABLE/DEFERRED preserved; 35/35 unviable; no adequacy PASS",
        "missing=" + repr(missing) + "\nforbidden=" + repr(forbidden),
        None if status == "PASS_PRELIM" else "Error::MutationMisclassified",
    )


def tla_path_lane() -> LaneResult:
    tla = BEAD / "tla" / "ScxhRecovery.tla"
    cfg = BEAD / "tla" / "ScxhRecovery.cfg"
    obligations = read_text(BEAD / "proof-obligations.jsonl")
    missing = [str(p.relative_to(ROOT)) for p in (tla, cfg) if not file_nonempty(p)]
    active_specs = []
    for line in obligations.splitlines():
        if '"model":".beads/vb-scxh/specs/' in line or '"config":".beads/vb-scxh/specs/' in line:
            active_specs.append(line[:160])
    status = "PASS_PRELIM" if not missing and not active_specs else "RED_PRELIM"
    return LaneResult(
        "tla_path_preflight",
        ("TLA-SCXH-005", "ERR-SCXH-010"),
        status,
        "test -s .beads/vb-scxh/tla/ScxhRecovery.tla && test -s .beads/vb-scxh/tla/ScxhRecovery.cfg; audit obligation paths",
        "canonical .beads/vb-scxh/tla paths exist and no active specs/ model/config target remains",
        "missing=" + repr(missing) + "\nactive_specs_targets=" + repr(active_specs),
        None if status == "PASS_PRELIM" else "Error::TlaPathMismatch",
    )


def planned_only_lane(name: str, obligations: tuple[str, ...], command: str, expected: str, error: str) -> LaneResult:
    return LaneResult(name, obligations, "NOT_RUN_STATE11_REQUIRED", command, expected, "State 8 scaffold only; raw capture intentionally deferred.", error)


def all_lanes() -> list[LaneResult]:
    return [
        workspace_lane(),
        approval_lane(),
        artifact_presence_lane(),
        planned_only_lane(
            "bd_command_plan",
            ("BD-SCXH-001", "BD-SCXH-002", "ERR-SCXH-005"),
            "bd --db /home/lewis/src/.beads/dolt show vb-scxh --json && bd --db /home/lewis/src/.beads/dolt list --json && per-ID bd show commands",
            "exact 12 false-closure IDs and per-ID raw reopened/linked/follow-up evidence",
            "Error::FalseClosureUnverified or Error::MissingRawEvidence",
        ),
        safety_lane(),
        moon_lane(),
        mutation_lane(),
        planned_only_lane(
            "scope_command_plan",
            ("SCOPE-SCXH-001", "TLA-SCXH-004", "ERR-SCXH-008"),
            "bd --db /home/lewis/src/.beads/dolt show vb-gvmt --json && bd --db /home/lewis/src/.beads/dolt show vb-qi37.10 --json",
            "generated parity remains deferred/owned by vb-gvmt or vb-qi37.10",
            "Error::ScopeConflation",
        ),
        planned_only_lane(
            "laundering_negative_fixture",
            ("TRUTH-SCXH-001", "TLA-SCXH-002", "ERR-SCXH-004"),
            "State 12 review of assurance-bundle classifications",
            "SUBAGENT_CLAIM without distinct raw backing is rejected/blocked",
            "Error::LaunderedSubagentClaim",
        ),
        tla_path_lane(),
        planned_only_lane(
            "final_gate_negative_fixture",
            ("TRUTH-SCXH-001", "TLA-SCXH-001", "ERR-SCXH-009"),
            "State 12 final-evidence-decision review after State 11 reports exist",
            "APPROVE_CLOSE_OR_UNBLOCK forbidden while any required lane is missing/blocked",
            "Error::BlockedEngineUnblock",
        ),
    ]


def render_markdown(results: list[LaneResult]) -> str:
    lines = [
        "# State 8 Preliminary Audit Harness Output: vb-scxh",
        "",
        f"Generated UTC: {datetime.now(timezone.utc).isoformat()}",
        "",
        "STATUS: RED_PRELIMINARY_NOT_STATE11_EVIDENCE",
        "",
        "This file is failing-first scaffolding output only. It is not `.beads/vb-scxh/assurance-bundle.md`, not `.beads/vb-scxh/truth-serum-report.md`, and not `.beads/vb-scxh/final-evidence-decision.md`.",
        "",
        "## Lane Results",
        "",
    ]
    for result in results:
        lines.extend(
            [
                f"### {result.name}",
                "",
                f"- Status: {result.status}",
                f"- Proof obligations: {', '.join(result.obligation_ids)}",
                f"- Command/check: `{result.command}`",
                f"- Expected: {result.expected}",
                f"- Error mapping: {result.error or 'none'}",
                "- Actual/raw/prelim:",
                "",
                "```text",
                result.actual.rstrip(),
                "```",
                "",
            ]
        )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--red-preflight", action="store_true", help="run preliminary checks and return non-zero on red lanes")
    parser.add_argument("--out", default=".beads/vb-scxh/state8-red-preflight.md", help="output markdown path under /home/lewis/src/vb-scxh")
    args = parser.parse_args()

    if Path.cwd().resolve() != ROOT:
        sys.stderr.write(f"must run from {ROOT}; got {Path.cwd().resolve()}\n")
        return 2
    if not args.red_preflight:
        sys.stderr.write("refusing to run without --red-preflight; this harness is preliminary only\n")
        return 2

    out = (ROOT / args.out).resolve()
    if ROOT not in out.parents or out.name in {"assurance-bundle.md", "truth-serum-report.md", "final-evidence-decision.md"}:
        sys.stderr.write("output must stay under isolated worktree and must not be a final State 11/12 artifact\n")
        return 2
    results = all_lanes()
    out.write_text(render_markdown(results), encoding="utf-8")
    has_red = any(result.status == "RED_PRELIM" for result in results)
    print(f"wrote {out.relative_to(ROOT)}")
    print(f"red_prelim={has_red}")
    return 1 if has_red else 0


if __name__ == "__main__":
    raise SystemExit(main())
