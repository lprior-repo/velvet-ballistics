#!/usr/bin/env python3
"""vb-3iflr independent measurement review: v3 actual-collector assertion verification.

Imports the actual collector parsers, reads the published evidence, and asserts
every measurable claim against the real raw data. Non-zero exit on ANY mismatch.
"""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path("/home/lewis/src/isoloated/velvet-ballistics-vb-j3e3s-local3090-measurement-review")

# Import actual collector parsers
sys.path.insert(0, str(ROOT / ".beads/vb-j3e3s"))
from collect_benchmark_evidence import (
    parse_criterion_sample,
    parse_criterion_estimates,
    parse_perf_csv,
    parse_heaptrack_trace,
    percentile,
    CollectorError,
)

# Paths
EVIDENCE_JSONL = ROOT / "evidence/benchmark-evidence.jsonl"
V3_SCRATCH = ROOT / ".beads/vb-j3e3s/evidence/actual-collection-v3/scratch"
CRITERION_HOME = V3_SCRATCH / "criterion-home"
PERF_DIR = ROOT / ".beads/vb-j3e3s/evidence/actual-collection-v3/perf"
HEAPTRACK_DIR = ROOT / ".beads/vb-j3e3s/evidence/actual-collection-v3/heaptrack"
HEAPTRACK_MANIFEST = ROOT / ".beads/vb-j3e3s/evidence/actual-collection-v3/heaptrack-manifest.json"

EXPECTED_COMMIT = "534aa40434b2704ad63666130557937087796259"

BENCHMARK_DIRS = [
    ("bench_engine_step_once_save_const_single_transition",
     "criterion-bench_engine_step_once_save_const_single_transition",
     "runtime_core/bench_engine_step_once_save_const_single_transition;profile=benc/new/"),
    ("ipc_frame_decode",
     "criterion-ipc_frame_decode",
     "storage_ipc/ipc_frame_decode;profile=bench;tool=criterion-0.8;durability=mix/new/"),
    ("engine_run_until_blocked_budget_10_small_workflow",
     "criterion-engine_run_until_blocked_budget_10_small_workflow",
     "runtime_core/engine_run_until_blocked_budget_10_small_workflow;profile=bench;/new/"),
]

errors = []
checks = 0

def assert_eq(actual, expected, label):
    global checks
    checks += 1
    if actual != expected:
        errors.append(f"FAIL {label}: expected {expected}, got {actual}")
        return False
    print(f"  PASS {label}: {actual}")
    return True

def assert_close(actual, expected, tolerance, label):
    global checks
    checks += 1
    if abs(actual - expected) > tolerance:
        errors.append(f"FAIL {label}: expected {expected} +/- {tolerance}, got {actual}")
        return False
    print(f"  PASS {label}: {actual} (within {tolerance} of {expected})")
    return True

# Load evidence records
print("=== Loading evidence records ===")
records = {}
for line in EVIDENCE_JSONL.read_text().splitlines():
    line = line.strip()
    if not line:
        continue
    rec = json.loads(line)
    records[rec["metric"]] = rec

assert_eq(len(records), 3, "evidence record count")

# Load heaptrack manifest
print("\n=== Loading heaptrack manifest ===")
manifest = json.loads(HEAPTRACK_MANIFEST.read_text())
assert_eq(len(manifest), 3, "heaptrack manifest entry count")

# Check 1: Commit SHA validation
print("\n=== Check 1: Commit SHA validation ===")
for metric, rec in records.items():
    commit = rec.get("commit", "")
    assert_eq(len(commit), 40, f"{metric} commit SHA length")
    assert_eq(commit, EXPECTED_COMMIT, f"{metric} commit SHA value")

# Check commit is ancestor of current HEAD
print(f"\n=== Check 1b: Commit ancestry (git merge-base) ===")
current_head = subprocess.run(
    ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True
).stdout.strip()
try:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", EXPECTED_COMMIT, current_head],
        cwd=ROOT, capture_output=True, text=True, check=True
    )
    checks += 1
    print(f"  PASS measured commit {EXPECTED_COMMIT[:8]} is ancestor of HEAD {current_head[:8]}")
except subprocess.CalledProcessError as e:
    errors.append(f"FAIL measured commit not ancestor of HEAD: {e}")

# Check 2: Criterion measurements
print("\n=== Check 2: Criterion measurements ===")
for metric, criterion_dir, profile_path in BENCHMARK_DIRS:
    print(f"\n  {metric}:")
    rec = records[metric]
    sample_path = CRITERION_HOME / criterion_dir / profile_path / "sample.json"
    estimates_path = CRITERION_HOME / criterion_dir / profile_path / "estimates.json"
    benchmark_path = CRITERION_HOME / criterion_dir / profile_path / "benchmark.json"

    # Parse sample
    sample_json = json.loads(sample_path.read_text())
    per_iter_ns, sample_count = parse_criterion_sample(sample_json)
    assert_eq(sample_count, 10, f"{metric} sample count")

    # Parse estimates
    estimates_json = json.loads(estimates_path.read_text())
    mean_ns = parse_criterion_estimates(estimates_json)
    assert_close(mean_ns, sum(per_iter_ns) / len(per_iter_ns), 0.1, f"{metric} mean vs computed")

    # Verify evidence picoseconds = round(mean_ns * 1000)
    expected_ps = round(mean_ns * 1000)
    assert_eq(rec["result_value"], expected_ps, f"{metric} result_value picoseconds")
    assert_eq(rec["baseline_value"], expected_ps, f"{metric} baseline_value = result (intentional)")
    # Verify percentiles (parse "16.440891 ns sample-derived" strings)
    for p, field in [(50, "p50_latency"), (95, "p95_latency"), (99, "p99_latency")]:
        computed = percentile(per_iter_ns, p)
        raw = rec[field]
        record_val = float(raw.split()[0])
        assert_close(computed, record_val, 1e-6, f"{metric} {field}")

    # Verify full_id matches benchmark name (v3 format: benchmark path with metadata)
    benchmark_json = json.loads(benchmark_path.read_text())
    full_id = benchmark_json.get("full_id", "")
    if metric not in full_id:
        errors.append(f"FAIL {metric} not in full_id: {full_id}")
    else:
        checks += 1
        print(f"  PASS {metric} metric name present in full_id")

# Check 3: Perf instruction counts
print("\n=== Check 3: Perf instruction counts ===")
for metric in records:
    print(f"\n  {metric}:")
    rec = records[metric]
    csv_path = PERF_DIR / f"{metric}.csv"
    csv_content = csv_path.read_text()
    instruction_count = parse_perf_csv(csv_content)
    assert_eq(instruction_count, rec["instruction_count"], f"{metric} instruction_count")

# Check 4: Heaptrack SHA validation
print("\n=== Check 4: Heaptrack SHA validation ===")
for metric, trace_info in manifest.items():
    filename = f"ht-{metric}.zst"
    expected_sha = trace_info["sha256"]
    filepath = HEAPTRACK_DIR / filename
    computed_sha = hashlib.sha256(filepath.read_bytes()).hexdigest()
    assert_eq(computed_sha, expected_sha, f"{filename} SHA256")

# Check 5: Heaptrack content validation
print("\n=== Check 5: Heaptrack content validation ===")
for metric, trace_info in manifest.items():
    filename = f"ht-{metric}.zst"
    filepath = HEAPTRACK_DIR / filename
    rec = records[metric]
    print(f"\n  {metric}:")

    # Decompress with zstd (check=True for failure)
    result = subprocess.run(
        ["zstd", "-d", "-c", str(filepath)],
        capture_output=True, text=True, check=True
    )
    trace_text = result.stdout

    # Parse trace
    count, total_bytes, peak_bytes = parse_heaptrack_trace(trace_text)

    # Compare count and total to manifest
    assert_eq(count, trace_info["allocation_count"], f"{metric} allocation_count vs manifest")
    assert_eq(total_bytes, trace_info["bytes_allocated"], f"{metric} bytes_allocated vs manifest")

    # Compare to evidence record
    assert_eq(count, rec["allocation_count"], f"{metric} allocation_count vs record")
    assert_eq(total_bytes, rec["bytes_allocated"], f"{metric} bytes_allocated vs record")

# Check 6: Thresholds
print("\n=== Check 6: Threshold validation ===")
expected_thresholds = {
    "bench_engine_step_once_save_const_single_transition": 3,
    "ipc_frame_decode": 5,
    "engine_run_until_blocked_budget_10_small_workflow": 3,
}
for metric, rec in records.items():
    assert_eq(rec["threshold_percent"], expected_thresholds[metric], f"{metric} threshold")

# Check 7: Baselines = results (intentional non-speedup claim)
print("\n=== Check 7: Baseline = result (intentional) ===")
for metric, rec in records.items():
    assert_eq(rec["baseline_value"], rec["result_value"], f"{metric} baseline = result")

# Summary
print(f"\n{'='*60}")
print(f"Total checks: {checks}")
print(f"Errors: {len(errors)}")
if errors:
    print("\nERRORS:")
    for e in errors:
        print(f"  {e}")
    sys.exit(1)
else:
    print("ALL CHECKS PASSED")
    sys.exit(0)