#!/usr/bin/env python3
"""vb-9qdjg independent LOCAL RTX5090 benchmark measurement review.

Imports actual collector parsers, verifies ALL 3 evidence records against
the real raw data under .beads/vb-oxzbo/evidence/actual-collection-v3.
Non-zero exit on ANY mismatch. No source/gate/exception changes.
"""
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path("/home/lewis/src/isoloated/velvet-ballistics-vb-9qdjg-local5090-benchmark-review")

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
V3_SCRATCH = ROOT / ".beads/vb-oxzbo/evidence/actual-collection-v3/scratch"
CRITERION_HOME = V3_SCRATCH / "criterion-home"
PERF_DIR = V3_SCRATCH / "perf"
HEAPTRACK_DIR = V3_SCRATCH / "heaptrack"
HEAPTRACK_MANIFEST = V3_SCRATCH / "heaptrack-manifest.json"
REPO_HEAPTRACK_MANIFEST = ROOT / ".beads/vb-oxzbo/evidence/actual-collection-v3/repository-heaptrack-manifest.json"

EXPECTED_COMMIT = "2fea0dcec4bc12c9d8c34fcf894398b3e8f38c9d"
EXPECTED_SOURCE_SHA = "ccac03cccff3787bcd57aab6603dd61709c1be85cd236a89c5a511ce84f1d1ba"

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
summary = []

def check(label, actual, expected=None, close=False, tolerance=0.0):
    global checks
    checks += 1
    if close:
        if abs(actual - expected) > tolerance:
            errors.append(f"FAIL {label}: expected {expected} +/- {tolerance}, got {actual}")
            summary.append(f"  ✗ {label}")
            return False
        summary.append(f"  ✓ {label}")
        return True
    if actual != expected:
        errors.append(f"FAIL {label}: expected {expected}, got {actual}")
        summary.append(f"  ✗ {label}")
        return False
    summary.append(f"  ✓ {label}")
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
check("evidence record count", len(records), 3)

# Load heaptrack manifest
print("\n=== Loading heaptrack manifest ===")
manifest = json.loads(HEAPTRACK_MANIFEST.read_text())
check("heaptrack manifest entry count", len(manifest), 3)

# Check 1: Commit SHA validation
print("\n=== Check 1: Commit SHA validation ===")
for metric, rec in records.items():
    commit = rec.get("commit", "")
    check(f"{metric} commit SHA length", len(commit), 40)
    check(f"{metric} commit SHA value", commit, EXPECTED_COMMIT)

# Check commit is ancestor of current HEAD
print(f"\n=== Check 1b: Commit ancestry (git merge-base) ===")
current_head = subprocess.run(
    ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True
).stdout.strip()
try:
    subprocess.run(
        ["git", "merge-base", "--is-ancestor", EXPECTED_COMMIT, current_head],
        cwd=ROOT, capture_output=True, text=True, check=True
    )
    check(f"measured commit {EXPECTED_COMMIT[:8]} is ancestor of HEAD {current_head[:8]}", True, True)
except subprocess.CalledProcessError:
    check(f"measured commit {EXPECTED_COMMIT[:8]} is ancestor of HEAD {current_head[:8]}", False, True)

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
    check(f"{metric} sample count", sample_count, 10)

    # Parse estimates
    estimates_json = json.loads(estimates_path.read_text())
    mean_ns = parse_criterion_estimates(estimates_json)
    check(f"{metric} mean vs computed", mean_ns, sum(per_iter_ns) / len(per_iter_ns), close=True, tolerance=0.1)

    # Verify evidence picoseconds = round(mean_ns * 1000)
    expected_ps = round(mean_ns * 1000)
    check(f"{metric} result_value picoseconds", rec["result_value"], expected_ps)
    check(f"{metric} baseline_value = result (intentional)", rec["baseline_value"], expected_ps)

    # Verify percentiles (parse "16.440891 ns sample-derived" strings)
    for p, field in [(50, "p50_latency"), (95, "p95_latency"), (99, "p99_latency")]:
        computed = percentile(per_iter_ns, p)
        raw = rec[field]
        record_val = float(raw.split()[0])
        check(f"{metric} {field}", computed, record_val, close=True, tolerance=1e-6)

    # Verify full_id matches benchmark name (v3 format: benchmark path with metadata)
    benchmark_json = json.loads(benchmark_path.read_text())
    full_id = benchmark_json.get("full_id", "")
    check(f"{metric} metric name present in full_id", metric in full_id, True)

# Check 3: Perf instruction counts
print("\n=== Check 3: Perf instruction counts ===")
for metric, rec in records.items():
    print(f"\n  {metric}:")
    csv_path = PERF_DIR / f"{metric}.csv"
    csv_content = csv_path.read_text()
    instruction_count = parse_perf_csv(csv_content)
    check(f"{metric} instruction_count", instruction_count, rec["instruction_count"])

# Check 4: Heaptrack SHA validation
print("\n=== Check 4: Heaptrack SHA validation ===")
for metric, trace_info in manifest.items():
    filename = f"ht-{metric}.zst"
    expected_sha = trace_info["sha256"]
    filepath = HEAPTRACK_DIR / filename
    computed_sha = hashlib.sha256(filepath.read_bytes()).hexdigest()
    check(f"{filename} SHA256", computed_sha, expected_sha)

# Check 5: Heaptrack content validation
print("\n=== Check 5: Heaptrack content validation ===")
for metric, trace_info in manifest.items():
    filename = f"ht-{metric}.zst"
    filepath = HEAPTRACK_DIR / filename
    rec = records[metric]
    print(f"\n  {metric}:")

    # Decompress with zstd (check=True for failure) - use tempfile to avoid printing 200MB
    with tempfile.TemporaryFile(mode='w+') as tmp:
        subprocess.run(["zstd", "-d", "-c", str(filepath)], stdout=tmp, check=True)
        tmp.seek(0)
        trace_text = tmp.read()

    # Parse trace
    count, total_bytes, peak_bytes = parse_heaptrack_trace(trace_text)

    # Compare count and total to manifest
    check(f"{metric} allocation_count vs manifest", count, trace_info["allocation_count"])
    check(f"{metric} bytes_allocated vs manifest", total_bytes, trace_info["bytes_allocated"])

    # Compare to evidence record
    check(f"{metric} allocation_count vs record", count, rec["allocation_count"])
    check(f"{metric} bytes_allocated vs record", total_bytes, rec["bytes_allocated"])

# Check 6: Thresholds
print("\n=== Check 6: Threshold validation ===")
expected_thresholds = {
    "bench_engine_step_once_save_const_single_transition": 3,
    "ipc_frame_decode": 5,
    "engine_run_until_blocked_budget_10_small_workflow": 3,
}
for metric, rec in records.items():
    check(f"{metric} threshold", rec["threshold_percent"], expected_thresholds[metric])

# Check 7: Baselines = results (intentional non-speedup claim)
print("\n=== Check 7: Baseline = result (intentional) ===")
for metric, rec in records.items():
    check(f"{metric} baseline = result", rec["baseline_value"], rec["result_value"])

# Check 8: Hardware metadata
print("\n=== Check 8: Hardware metadata ===")
for metric, rec in records.items():
    cpu = rec.get("cpu_model", "")
    check(f"{metric} cpu_model", cpu, "AMD Ryzen 9 9950X3D 16-Core Processor")
    kernel = rec.get("kernel_version", "")
    check(f"{metric} kernel_version", "Linux omarchy 7.2.3-arch1-3" in kernel, True)
    rustc = rec.get("rustc_version", "")
    check(f"{metric} rustc_version", "rustc 1.97.0-nightly" in rustc, True)

# Summary
print(f"\n{'='*60}")
print(f"Total checks: {checks}")
print(f"Errors: {len(errors)}")
print("\nSummary:")
for s in summary:
    print(s)

if errors:
    print("\nERRORS:")
    for e in errors:
        print(f"  {e}")
    sys.exit(1)
else:
    print("\nALL CHECKS PASSED")
    sys.exit(0)