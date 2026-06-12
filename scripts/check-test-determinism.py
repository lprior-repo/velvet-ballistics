#!/usr/bin/env python3
"""
Determinism gate: rejects non-deterministic patterns in critical tests.

Patterns detected:
  - thread::sleep (sleeps-as-sync anti-pattern)
  - Instant::now / SystemTime (uncontrolled clocks)
  - rand RNGs without explicit seed (uncontrolled random seeds)
  - shared temp state (non-isolated temp directories)
  - hidden mutable global state (static mut, Once, Cell/RefCell in shared contexts)

Evidence labels cached/skipped/flaky results as non-release.
"""
import sys
import subprocess
import re
import os
from pathlib import Path
from typing import Iterator, NamedTuple


class Finding(NamedTuple):
    kind: str
    path: str
    line: int
    detail: str


ROOT = Path(subprocess.check_output(
    ["git", "rev-parse", "--show-toplevel"],
    text=True
).strip())
BASELINE = ROOT / ".evidence" / "test-determinism" / "baseline.txt"


# Patterns: (kind, regex, is_critical_only)
PATTERNS = [
    # sleeps-as-sync
    ("SleepAsSync", r'\bthread::sleep\b', False),
    ("SleepAsSync", r'\bstd::thread::sleep\b', False),
    ("SleepAsSync", r'\btokio::time::sleep\b', False),

    # uncontrolled clocks
    ("UncontrolledClock", r'\bInstant::now\b', False),
    ("UncontrolledClock", r'\bSystemTime::now\b', False),
    ("UncontrolledClock", r'\bunix_time\(\)', False),
    ("UncontrolledClock", r'\bstd::time::SystemTime::now\b', False),

    # uncontrolled random seeds
    ("UncontrolledRandom", r'\brand::', False),
    ("UncontrolledRandom", r'\bfastrand::', False),
    ("UncontrolledRandom", r'\brand_core', False),

    # shared temp state
    ("SharedTempState", r'tempdir\(\)', False),
    ("SharedTempState", r'TempDir', False),
    ("SharedTempState", r'\.tempdir\(', False),
    ("SharedTempState", r'\.tmpdir\(', False),
    ("SharedTempState", r'\/tmp\/', False),

    # hidden mutable global state
    ("GlobalMutableState", r'\bstatic\s+mut\b', True),
    ("GlobalMutableState", r'\bstd::sync::Once\b', True),
    ("GlobalMutableState", r'\bOnce\b', True),
    ("GlobalMutableState", r'\bstd::cell::(Cell|RefCell)\b.*\bmut\b', True),
    ("GlobalMutableState", r'\bCell::<.*>\b', True),
    ("GlobalMutableState", r'\bRefCell::<.*>\b', True),
    ("GlobalMutableState", r'\bMutex\b', False),
    ("GlobalMutableState", r'\bRwLock\b', False),

    # controlled clock seeding
    ("ControlledClock", r'\bInstant::now_with_clock\b', False),
    ("ControlledClock", r'\bSystemTime::now_with_clock\b', False),

    # seeded random
    ("SeededRandom", r'\brand::(?:seed|from_seed|SmallRng)', False),
    ("SeededRandom", r'\brand::rngs::StdRng::from_seed', False),
]


def is_test_file(path: Path) -> bool:
    """Only scan test and example files."""
    path_str = str(path)
    return (
        path_str.endswith(".rs")
        and (
            "/tests/" in path_str
            or "/benches/" in path_str
            or "/examples/" in path_str
            or "/fuzz/" in path_str
            or "workspace_tests" in path_str
        )
    )


def scan_file(path: Path) -> Iterator[Finding]:
    """Scan a single file for determinism violations."""
    try:
        content = path.read_text()
    except Exception:
        return

    for kind, pattern, _critical_only in PATTERNS:
        regex = re.compile(pattern)
        for i, line in enumerate(content.splitlines(), start=1):
            if regex.search(line):
                yield Finding(
                    kind=kind,
                    path=str(path.relative_to(ROOT)),
                    line=i,
                    detail=line.strip()[:120],
                )


def gather_findings() -> list[Finding]:
    """Gather all findings from test files in the workspace."""
    findings = []
    for crate in (ROOT / "crates").iterdir():
        if not crate.is_dir():
            continue
        for pattern in ["tests", "benches", "examples"]:
            for test_dir in crate.rglob(pattern):
                if test_dir.is_dir():
                    for rs_file in test_dir.rglob("*.rs"):
                        if is_test_file(rs_file):
                            findings.extend(scan_file(rs_file))

    # Also scan workspace_tests directly
    wt = ROOT / "crates" / "workspace_tests"
    if wt.exists():
        for rs_file in wt.rglob("*.rs"):
            if is_test_file(rs_file):
                findings.extend(scan_file(rs_file))

    return findings


def render_label(finding: Finding) -> str:
    return f"{finding.kind}|{finding.path}|{finding.line}|{finding.detail}"


def load_baseline_labels() -> set[str]:
    if not BASELINE.exists():
        return set()

    labels = set()
    for line in BASELINE.read_text().splitlines():
        if line.count("|") >= 3:
            labels.add(line)
    return labels


def render_summary(findings: list[Finding], prefix: str) -> None:
    by_kind: dict[str, list[Finding]] = {}
    for finding in findings:
        by_kind.setdefault(finding.kind, []).append(finding)

    print(prefix, file=sys.stderr)
    print(f"  Total findings: {len(findings)}", file=sys.stderr)
    for kind, group in sorted(by_kind.items()):
        print(f"  {kind}: {len(group)}", file=sys.stderr)
    print(file=sys.stderr)


def main() -> int:
    ci_mode = "--ci" in sys.argv or os.environ.get("MOON_CI") == "1" or os.environ.get("CI") == "1"
    findings = gather_findings()
    labels = {render_label(finding) for finding in findings}
    baseline_labels = load_baseline_labels()

    if not findings:
        print("test determinism: PASS — no non-deterministic patterns detected")
        return 0

    if baseline_labels:
        new_labels = sorted(labels - baseline_labels)
        resolved_labels = sorted(baseline_labels - labels)
        if not new_labels:
            mode = "CI" if ci_mode else "dev"
            print(
                f"test determinism: PASS — {len(labels)} findings are within archived baseline ({mode} mode)",
                file=sys.stderr,
            )
            if resolved_labels:
                print(f"  Resolved baseline findings: {len(resolved_labels)}", file=sys.stderr)
            return 0

        render_summary(findings, "test determinism: FAIL — new findings exceed archived baseline")
        print(f"  Baseline: {len(baseline_labels)} labels", file=sys.stderr)
        print(f"  New findings: {len(new_labels)}", file=sys.stderr)
        if resolved_labels:
            print(f"  Resolved baseline findings: {len(resolved_labels)}", file=sys.stderr)
        print(file=sys.stderr)
        for label in new_labels:
            print(label, file=sys.stderr)
        return 1

    # Group by kind
    render_summary(findings, "test determinism: FAIL")

    for f in findings:
        print(render_label(f), file=sys.stderr)

    print(file=sys.stderr)
    print("Evidence: label findings as cached/skipped/flaky in non-release CI.", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
