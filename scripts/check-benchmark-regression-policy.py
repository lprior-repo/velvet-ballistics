#!/usr/bin/env python3
"""Check the benchmark regression policy gate.

Stdlib-only port of the deleted xtask gate (xtask/src/benchmark_policy.rs,
benchmark_freshness.rs and benchmark_budget.rs as of b24d9122^1, parent
2e3174f8). Same checks, same messages, same exit contract: exit 0 with
``BenchmarkRegressionPolicyOk budget=... evidence=... claims=N`` iff every
budgeted metric has exactly one evidence row that is fresh, within its
regression threshold, fully annotated and backed by a raw log; exit 1
otherwise.

Differences from the Rust original, all deliberate: the budget YAML is read
by a strict subset parser below (no PyYAML dependency, so the gate runs
wherever python3 and git exist) instead of serde_yaml; JSON is read by the
stdlib json module; u64 bounds are enforced explicitly. The perf-relevant
path list is verbatim, including ``xtask/``, which no longer exists on main
and therefore matches nothing; it is retained so the list stays a faithful
copy of the original conservative set.

Usage:
  python3 scripts/check-benchmark-regression-policy.py [--budget contracts/perf-budget.yaml]
      [--evidence evidence/benchmark-evidence.jsonl] [repo-root]
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

CURRENT_MILESTONE = "ir-interpreter"

REQUIRED_EVIDENCE_FIELDS = [
    "metric",
    "claim",
    "baseline_value",
    "result_value",
    "unit",
    "threshold_percent",
    "raw_log",
    "command",
    "commit",
    "rustc_version",
    "nightly_date",
    "cpu_model",
    "cpu_governor",
    "kernel_version",
    "build_profile",
    "rustflags",
    "benchmark_tool_version",
    "sample_count_or_instruction_count",
    "input_fixture_digest",
    "durability_profile",
    "mode",
    "p50_latency",
    "p95_latency",
    "p99_latency",
    "instruction_count",
    "allocation_count",
    "bytes_allocated",
    "fjall_write_latency",
    "direct_api_latency",
    "ipc_latency",
]

FORBIDDEN_EVIDENCE_MARKERS = [
    "instructions=not-collected",
    "instruction_count=not-collected",
    "allocations=allocator-external",
    "allocation_count=allocator-external",
]

PERF_RELEVANT_PATHS = [
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rust-toolchain",
    ".cargo/",
    "crates/",
    "xtask/",
    "contracts/perf-budget.yaml",
    "fixtures/",
    "scripts/",
    ".moon/",
]

UNTRACKED_EXCLUDES = ["target", ".git", ".jj", ".moon/cache"]

U64_MAX = (1 << 64) - 1


class GateError(Exception):
    pass


def normalized(value):
    return value.strip().lower()


def normalized_set(values):
    return {normalized(value) for value in values}


def has_deferred_marker(value):
    text = normalized(value)
    return "generated" in text or "maxperf" in text


def parse_scalar(raw, where):
    text = raw.strip()
    if not text:
        raise GateError(f"{where}: empty scalar")
    if len(text) >= 2 and text[0] == '"' and text[-1] == '"':
        body = text[1:-1]
        out = []
        i = 0
        while i < len(body):
            char = body[i]
            if char == "\\":
                if i + 1 >= len(body):
                    raise GateError(f"{where}: trailing backslash in quoted scalar")
                nxt = body[i + 1]
                mapping = {"n": "\n", "t": "\t", '"': '"', "\\": "\\"}
                if nxt not in mapping:
                    raise GateError(f"{where}: unsupported escape \\{nxt}")
                out.append(mapping[nxt])
                i += 2
            else:
                out.append(char)
                i += 1
        return "".join(out)
    if len(text) >= 2 and text[0] == "'" and text[-1] == "'":
        return text[1:-1].replace("''", "'")
    if text in ("true", "True", "TRUE"):
        return True
    if text in ("false", "False", "FALSE"):
        return False
    if text.lstrip("-").isdigit():
        try:
            return int(text, 10)
        except ValueError:
            raise GateError(f"{where}: integer out of range: {text}")
    return text


def parse_budget_yaml(text):
    lines = []
    for number, raw in enumerate(text.splitlines(), start=1):
        if "\t" in raw:
            raise GateError(f"performance budget line {number}: tab indentation is not allowed")
        if not raw.strip() or raw.strip().startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip(" "))
        if indent % 2 != 0:
            raise GateError(f"performance budget line {number}: odd indentation")
        lines.append((number, indent // 2, raw.strip()))
    if not lines:
        raise GateError("performance budget is empty")
    if lines[0][1] != 0:
        raise GateError(f"performance budget line {lines[0][0]}: first key must be top-level")
    budget = {}
    index = 0
    while index < len(lines):
        number, level, content = lines[index]
        if level != 0 or content.startswith("- "):
            raise GateError(f"performance budget line {number}: expected a top-level key")
        if ":" not in content:
            raise GateError(f"performance budget line {number}: expected 'key: value'")
        key, _, rest = content.partition(":")
        key = key.strip()
        rest = rest.strip()
        if not key:
            raise GateError(f"performance budget line {number}: empty key")
        if key in budget:
            raise GateError(f"performance budget line {number}: duplicate key {key}")
        index += 1
        if rest:
            budget[key] = parse_scalar(rest, f"performance budget line {number}")
            continue
        if index >= len(lines) or lines[index][1] != 1:
            raise GateError(f"performance budget line {number}: key {key} needs a nested block")
        if lines[index][2].startswith("- "):
            items = []
            while index < len(lines) and lines[index][1] == 1 and lines[index][2].startswith("- "):
                item_number = lines[index][0]
                items.append(parse_scalar(lines[index][2][2:], f"performance budget line {item_number}"))
                index += 1
            budget[key] = items
        else:
            nested = {}
            while index < len(lines) and lines[index][1] == 1 and not lines[index][2].startswith("- "):
                item_number, _, item_content = lines[index]
                if ":" not in item_content:
                    raise GateError(f"performance budget line {item_number}: expected 'key: value'")
                item_key, _, item_rest = item_content.partition(":")
                item_key = item_key.strip()
                item_rest = item_rest.strip()
                if not item_key:
                    raise GateError(f"performance budget line {item_number}: empty key")
                if item_key in nested:
                    raise GateError(f"performance budget line {item_number}: duplicate key {item_key}")
                index += 1
                if item_rest:
                    nested[item_key] = parse_scalar(item_rest, f"performance budget line {item_number}")
                    continue
                if index >= len(lines) or lines[index][1] != 2:
                    raise GateError(
                        f"performance budget line {item_number}: key {item_key} needs a nested block"
                    )
                inner = {}
                while index < len(lines) and lines[index][1] == 2:
                    inner_number, _, inner_content = lines[index]
                    if ":" not in inner_content:
                        raise GateError(
                            f"performance budget line {inner_number}: expected 'key: value'"
                        )
                    inner_key, _, inner_rest = inner_content.partition(":")
                    inner_key = inner_key.strip()
                    inner_rest = inner_rest.strip()
                    if not inner_key:
                        raise GateError(f"performance budget line {inner_number}: empty key")
                    if inner_key in inner:
                        raise GateError(
                            f"performance budget line {inner_number}: duplicate key {inner_key}"
                        )
                    if not inner_rest:
                        raise GateError(
                            f"performance budget line {inner_number}: key {inner_key} needs a value"
                        )
                    inner[inner_key] = parse_scalar(
                        inner_rest, f"performance budget line {inner_number}"
                    )
                    index += 1
                nested[item_key] = inner
            budget[key] = nested
    return budget


class PerfBudget:
    def __init__(self, raw):
        self.current_milestone = raw.get("current_milestone")
        self.generated_maxperf_deferred = raw.get("generated_maxperf_deferred")
        self.deferred_modes = raw.get("deferred_modes")
        self.required_evidence_fields = raw.get("required_evidence_fields")
        self.benchmarks = raw.get("benchmarks")


def parse_budget(text):
    raw = parse_budget_yaml(text)
    budget = PerfBudget(raw)
    validate_budget(budget)
    return budget


def validate_budget(budget):
    if budget.current_milestone != CURRENT_MILESTONE:
        raise GateError(
            f"performance budget milestone must be {CURRENT_MILESTONE}, "
            f"got {budget.current_milestone}"
        )
    if budget.generated_maxperf_deferred is not True:
        raise GateError("performance budget must mark generated/maxperf claims as deferred")
    if not isinstance(budget.deferred_modes, list) or not all(
        isinstance(item, str) for item in budget.deferred_modes
    ):
        raise GateError("performance budget deferred_modes must be a list of strings")
    deferred = normalized_set(budget.deferred_modes)
    if "generated" not in deferred or "maxperf" not in deferred:
        raise GateError("deferred_modes must include generated and maxperf")
    if not isinstance(budget.required_evidence_fields, list) or not all(
        isinstance(item, str) for item in budget.required_evidence_fields
    ):
        raise GateError("performance budget required_evidence_fields must be a list of strings")
    required = normalized_set(budget.required_evidence_fields)
    missing = [field for field in REQUIRED_EVIDENCE_FIELDS if field not in required]
    if missing:
        raise GateError(f"performance budget missing evidence fields: {', '.join(missing)}")
    if not isinstance(budget.benchmarks, dict) or not budget.benchmarks:
        raise GateError("performance budget must define at least one benchmark threshold")
    for name, threshold in budget.benchmarks.items():
        if has_deferred_marker(name):
            raise GateError(f"deferred generated/maxperf benchmark must not be budgeted: {name}")
        if not isinstance(threshold, dict):
            raise GateError(f"benchmark {name} must define max_regression_percent")
        percent = threshold.get("max_regression_percent")
        if not isinstance(percent, int) or isinstance(percent, bool) or percent <= 0:
            raise GateError(f"benchmark {name} must define a positive max_regression_percent")


def run_git(root, args):
    try:
        return subprocess.run(
            ["git", *args],
            cwd=str(root),
            stdin=subprocess.DEVNULL,
            capture_output=True,
        )
    except OSError as exc:
        raise GateError(f"run git {' '.join(args)}: {exc}")


def validate_commit_format(commit, label):
    length = len(commit)
    if not 7 <= length <= 40 or not all(char in "0123456789abcdefABCDEF" for char in commit):
        raise GateError(f"{label} must be 7-40 ASCII hex chars")


def resolve_commit(root, commit, line_number):
    output = run_git(root, ["rev-parse", "--verify", f"{commit}^{{commit}}"])
    if output.returncode != 0:
        stderr = output.stderr.decode("utf-8", "replace").strip()
        raise GateError(
            f"benchmark evidence line {line_number} commit {commit} is not a valid commit: {stderr}"
        )
    sha = output.stdout.decode("utf-8", "replace").strip()
    if len(sha) != 40 or not all(char in "0123456789abcdef" for char in sha):
        raise GateError(
            f"benchmark evidence line {line_number} git rev-parse returned unexpected value "
            f"for commit {commit}: {sha}"
        )
    return sha


def is_ancestor(root, ancestor, descendant):
    output = run_git(root, ["merge-base", "--is-ancestor", ancestor, descendant])
    if output.returncode == 0:
        return True
    if output.returncode == 1:
        return False
    stderr = output.stderr.decode("utf-8", "replace").strip()
    raise GateError(f"git merge-base --is-ancestor {ancestor} {descendant} failed: {stderr}")


def changed_paths_vs_working_tree(root, measured_commit):
    output = run_git(root, ["diff", "--no-renames", "-z", "--name-only", measured_commit])
    if output.returncode not in (0, 1):
        stderr = output.stderr.decode("utf-8", "replace").strip()
        raise GateError(f"git diff against working tree for {measured_commit} failed: {stderr}")
    try:
        text = output.stdout.decode("utf-8")
    except UnicodeDecodeError:
        raise GateError(
            f"decode git diff paths for {measured_commit} vs working tree (path not UTF-8)"
        )
    return [path for path in text.split("\0") if path]


def is_perf_relevant_path(path):
    for pattern in PERF_RELEVANT_PATHS:
        if pattern.endswith("/"):
            if path.startswith(pattern):
                return True
        elif path == pattern:
            return True
    return False


def find_untracked_relevant_files(root):
    args = ["ls-files", "--others", "-z"]
    for exclude in UNTRACKED_EXCLUDES:
        args.extend(["--exclude", exclude])
    args.append("--")
    args.extend(pattern.rstrip("/") for pattern in PERF_RELEVANT_PATHS)
    output = run_git(root, args)
    if output.returncode != 0:
        stderr = output.stderr.decode("utf-8", "replace").strip()
        raise GateError(f"git ls-files untracked files failed: {stderr}")
    try:
        text = output.stdout.decode("utf-8")
    except UnicodeDecodeError:
        raise GateError("decode git ls-files untracked paths (path not UTF-8)")
    return [path for path in text.split("\0") if path]


def validate_evidence_freshness(root, evidence_commit, current_commit, line_number):
    resolved = resolve_commit(root, evidence_commit, line_number)
    if not is_ancestor(root, resolved, current_commit):
        raise GateError(
            f"benchmark evidence line {line_number} commit {evidence_commit} is not an ancestor "
            f"of current HEAD {current_commit}"
        )
    changed = changed_paths_vs_working_tree(root, resolved)
    relevant_changed = [path for path in changed if is_perf_relevant_path(path)]
    if relevant_changed:
        raise GateError(
            f"benchmark evidence line {line_number} is stale: performance-relevant files differ "
            f"from measured commit {evidence_commit}: {', '.join(relevant_changed)}"
        )
    untracked = find_untracked_relevant_files(root)
    if untracked:
        raise GateError(
            f"benchmark evidence line {line_number} is stale: untracked performance-relevant "
            f"files exist: {', '.join(untracked)}"
        )


def current_head_commit(root):
    output = run_git(root, ["rev-parse", "HEAD"])
    if output.returncode != 0:
        stderr = output.stderr.decode("utf-8", "replace").strip()
        raise GateError(f"git rev-parse HEAD failed: {stderr}")
    commit = output.stdout.decode("utf-8", "replace").strip()
    validate_commit_format(commit, "current HEAD")
    return commit


def validate_no_forbidden_markers(text, line_number, label):
    for marker in FORBIDDEN_EVIDENCE_MARKERS:
        if marker in text:
            raise GateError(f"{label} {line_number} contains forbidden placeholder metadata: {marker}")


def evidence_value_present(value):
    if value is None:
        return False
    if isinstance(value, str):
        return bool(value.strip())
    if isinstance(value, (list, dict)):
        return bool(value)
    return True


def as_u64(value, line_number, field):
    if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= U64_MAX:
        raise GateError(f"decode benchmark evidence line {line_number}: {field} must be a u64")
    return value


def as_string(value, line_number, field):
    if not isinstance(value, str):
        raise GateError(f"decode benchmark evidence line {line_number}: {field} must be a string")
    return value


def validate_evidence(root, budget, text, current_commit):
    count = 0
    observed = set()
    for line_index, raw_line in enumerate(text.splitlines()):
        line_number = line_index + 1
        line = raw_line.strip()
        if not line:
            continue
        validate_no_forbidden_markers(line, line_number, "benchmark evidence line")
        try:
            claim_value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise GateError(f"parse benchmark evidence line {line_number}: {exc}")
        if not isinstance(claim_value, dict):
            raise GateError(f"benchmark evidence line {line_number} must be a JSON object")
        missing = [
            field
            for field in budget.required_evidence_fields
            if field not in claim_value or not evidence_value_present(claim_value[field])
        ]
        if missing:
            raise GateError(
                f"benchmark evidence line {line_number} missing required metadata fields: "
                f"{', '.join(missing)}"
            )
        claim = {
            "metric": as_string(claim_value["metric"], line_number, "metric"),
            "claim": as_string(claim_value["claim"], line_number, "claim"),
            "baseline_value": as_u64(claim_value["baseline_value"], line_number, "baseline_value"),
            "result_value": as_u64(claim_value["result_value"], line_number, "result_value"),
            "unit": as_string(claim_value["unit"], line_number, "unit"),
            "threshold_percent": as_u64(
                claim_value["threshold_percent"], line_number, "threshold_percent"
            ),
            "raw_log": as_string(claim_value["raw_log"], line_number, "raw_log"),
            "command": as_string(claim_value["command"], line_number, "command"),
            "commit": as_string(claim_value["commit"], line_number, "commit"),
            "mode": as_string(claim_value["mode"], line_number, "mode"),
        }
        validate_claim(root, budget, claim, line_number, current_commit)
        if claim["metric"] in observed:
            raise GateError(
                f"benchmark evidence line {line_number} duplicates metric {claim['metric']}"
            )
        observed.add(claim["metric"])
        count += 1
    missing = [metric for metric in budget.benchmarks if metric not in observed]
    if missing:
        raise GateError(f"benchmark evidence missing budgeted metrics: {', '.join(missing)}")
    return count


def validate_claim(root, budget, claim, line_number, current_commit):
    required = [
        ("metric", claim["metric"]),
        ("claim", claim["claim"]),
        ("unit", claim["unit"]),
        ("raw_log", claim["raw_log"]),
        ("command", claim["command"]),
        ("commit", claim["commit"]),
        ("mode", claim["mode"]),
    ]
    empty = [field for field, value in required if not value.strip()]
    if empty:
        raise GateError(
            f"benchmark evidence line {line_number} has empty required fields: {', '.join(empty)}"
        )
    if has_deferred_marker(claim["claim"]):
        raise GateError(
            f"benchmark evidence line {line_number} contains deferred generated/maxperf claim"
        )
    mode = normalized(claim["mode"])
    deferred = normalized_set(budget.deferred_modes)
    if mode in deferred or has_deferred_marker(mode):
        raise GateError(f"benchmark evidence line {line_number} uses deferred mode {mode}")
    if mode != CURRENT_MILESTONE:
        raise GateError(
            f"benchmark evidence line {line_number} mode must be {CURRENT_MILESTONE}, got {mode}"
        )
    validate_commit_format(claim["commit"], f"benchmark evidence line {line_number} commit")
    validate_evidence_freshness(root, claim["commit"], current_commit, line_number)
    raw_log = Path(claim["raw_log"])
    if raw_log.is_absolute():
        raise GateError(f"benchmark evidence line {line_number} raw_log must be workspace-relative")
    joined = root / raw_log
    if not joined.is_file():
        raise GateError(
            f"benchmark evidence line {line_number} raw_log does not exist: {joined}"
        )
    try:
        log_text = joined.read_text(encoding="utf-8")
    except OSError as exc:
        raise GateError(f"read benchmark evidence raw log {joined}: {exc}")
    except UnicodeDecodeError as exc:
        raise GateError(f"read benchmark evidence raw log {joined}: {exc}")
    validate_no_forbidden_markers(log_text, line_number, "benchmark evidence raw log")
    metric_budget = budget.benchmarks.get(claim["metric"])
    if metric_budget is None:
        raise GateError(
            f"benchmark evidence line {line_number} references unbudgeted metric {claim['metric']}"
        )
    max_regression = metric_budget["max_regression_percent"]
    if claim["threshold_percent"] != max_regression:
        raise GateError(
            f"benchmark evidence line {line_number} threshold {claim['threshold_percent']} does "
            f"not match budget {max_regression} for {claim['metric']}"
        )
    if result_exceeds_threshold(claim["result_value"], claim["baseline_value"], max_regression):
        raise GateError(
            f"benchmark evidence line {line_number} regressed: metric={claim['metric']} "
            f"baseline={claim['baseline_value']} result={claim['result_value']} "
            f"threshold_percent={max_regression}"
        )


def result_exceeds_threshold(result, baseline, threshold_percent):
    threshold_delta = baseline * threshold_percent // 100
    return result > baseline + threshold_delta


def main(argv=None):
    parser = argparse.ArgumentParser(description="Check the benchmark regression policy gate.")
    parser.add_argument("root", nargs="?", default=".", help="repo root (default: .)")
    parser.add_argument("--budget", default="contracts/perf-budget.yaml")
    parser.add_argument("--evidence", default="evidence/benchmark-evidence.jsonl")
    args = parser.parse_args(argv)
    root = Path(args.root)
    budget_path = root / args.budget
    evidence_path = root / args.evidence
    try:
        try:
            budget_text = budget_path.read_text(encoding="utf-8")
        except OSError as exc:
            raise GateError(f"read performance budget {budget_path}: {exc}")
        try:
            evidence_text = evidence_path.read_text(encoding="utf-8")
        except OSError as exc:
            raise GateError(f"read benchmark evidence {evidence_path}: {exc}")
        budget = parse_budget(budget_text)
        current_commit = current_head_commit(root)
        claims = validate_evidence(root, budget, evidence_text, current_commit)
    except GateError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    print(
        f"BenchmarkRegressionPolicyOk budget={budget_path} evidence={evidence_path} claims={claims}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
