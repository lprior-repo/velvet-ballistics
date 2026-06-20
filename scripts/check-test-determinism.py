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
from collections import Counter
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
    # NOTE: bare `TempDir` (return-type name) and `tempdir()` (isolated
    # `tempfile::tempdir()` calls) were removed as false positives — they flag
    # isolated temp directories, not shared state. Only `/tmp/` literals and
    # explicit `.tempdir(`/`.tmpdir(` builder calls remain; the `/tmp/` literal
    # catches the actual shared-state cases.
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


# Markers that indicate a test seeds its RNG explicitly. When any of these is
# present in a file, a `use rand::...` / `use fastrand::...` import line is not
# treated as an UncontrolledRandom source (the RNG is constructed deterministically).
SEEDING_PATTERNS: tuple[str, ...] = (
    "StdRng::seed_from_u64",
    "StdRng::from_seed",
    "SmallRng::from_seed",
    "SeedableRng::seed_from_u64",
)


def is_rust_comment(line: str) -> bool:
    """Return True for Rust comment lines that must be skipped before matching.

    Covers line comments (``//``, ``///``, ``//!``), block-comment opens
    (``/*``) and block-comment continuation lines (``*``). Rust attributes
    (``#[...]`` / ``#![...]``) are intentionally NOT comments and are kept.
    """
    stripped = line.lstrip()
    if not stripped:
        return False
    if stripped.startswith("//"):
        return True
    if stripped.startswith("/*"):
        return True
    if stripped.startswith("*"):
        return True
    return False


def is_rng_import(line: str) -> bool:
    """Return True if the line is a ``use`` import of an RNG crate."""
    stripped = line.lstrip()
    if not stripped.startswith("use "):
        return False
    return (
        "rand::" in stripped
        or "fastrand::" in stripped
        or "rand_core" in stripped
    )


def scan_file(path: Path) -> Iterator[Finding]:
    """Scan a single file for determinism violations."""
    try:
        content = path.read_text()
    except (OSError, UnicodeDecodeError):
        return

    has_seeding = any(marker in content for marker in SEEDING_PATTERNS)

    for kind, pattern, _critical_only in PATTERNS:
        regex = re.compile(pattern)
        for i, line in enumerate(content.splitlines(), start=1):
            if is_rust_comment(line):
                continue
            if not regex.search(line):
                continue
            if kind == "UncontrolledRandom" and has_seeding and is_rng_import(line):
                continue
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


def render_stable_label(finding: Finding) -> str:
    return f"{finding.kind}|{finding.path}|{finding.detail}"


def stable_label_from_rendered(label: str) -> str:
    parts = label.split("|", 3)
    if len(parts) < 4:
        return label
    return f"{parts[0]}|{parts[1]}|{parts[3]}"


def load_baseline_labels() -> tuple[set[str], int]:
    if not BASELINE.exists():
        return set(), 0

    labels = set()
    raw_label_rows = 0
    for line in BASELINE.read_text().splitlines():
        if line.count("|") >= 3:
            raw_label_rows += 1
            labels.add(line)
    return labels, raw_label_rows


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
    stable_counts = Counter(stable_label_from_rendered(label) for label in labels)
    baseline_labels, baseline_raw_label_rows = load_baseline_labels()
    baseline_stable_counts = Counter(
        stable_label_from_rendered(label) for label in baseline_labels
    )

    if not findings:
        print("test determinism: PASS — no non-deterministic patterns detected")
        return 0

    if baseline_labels:
        new_labels = sorted(
            label
            for label in labels
            if stable_counts[stable_label_from_rendered(label)]
            > baseline_stable_counts.get(stable_label_from_rendered(label), 0)
        )
        resolved_labels = sorted(
            label
            for label in baseline_labels
            if baseline_stable_counts[stable_label_from_rendered(label)]
            > stable_counts.get(stable_label_from_rendered(label), 0)
        )
        if not new_labels:
            mode = "CI" if ci_mode else "dev"
            print(
                "test determinism: PASS — "
                f"{len(labels)} distinct labels / {len(findings)} raw findings "
                f"are within archived baseline ({mode} mode)",
                file=sys.stderr,
            )
            if resolved_labels:
                print(
                    f"  Resolved baseline distinct labels: {len(resolved_labels)}",
                    file=sys.stderr,
                )
            return 0

        render_summary(
            findings,
            "test determinism: FAIL — new distinct labels exceed archived baseline",
        )
        print(
            f"  Baseline: {len(baseline_labels)} distinct labels / "
            f"{baseline_raw_label_rows} raw findings",
            file=sys.stderr,
        )
        print(f"  New distinct labels: {len(new_labels)}", file=sys.stderr)
        if resolved_labels:
            print(
                f"  Resolved baseline distinct labels: {len(resolved_labels)}",
                file=sys.stderr,
            )
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
