#!/usr/bin/env python3
"""Verify criterion benchmark measurements against collected evidence."""
import json
import sys
from pathlib import Path

ROOT = Path("/home/lewis/src/isoloated/velvet-ballistics-vb-j3e3s-local3090-measurement-review")
CRITERION_HOME = ROOT / ".beads/vb-j3e3s/evidence/actual-collection-v2/scratch/criterion-home"
EVIDENCE = ROOT / "evidence/benchmark-evidence.jsonl"

def percentile(sorted_values, p):
    """Linear-interpolation percentile."""
    if not sorted_values:
        raise ValueError("empty sample")
    idx = p / 100.0 * (len(sorted_values) - 1)
    lo = int(idx)
    frac = idx - lo
    if lo + 1 < len(sorted_values):
        return sorted_values[lo] + frac * (sorted_values[lo + 1] - sorted_values[lo])
    return sorted_values[lo]

def mean(values):
    return sum(values) / len(values)

def parse_sample(sample_json):
    iters = sample_json["iters"]
    times = sample_json["times"]
    if len(iters) != len(times):
        raise ValueError(f"iters/times length mismatch: {len(iters)} vs {len(times)}")
    per_iter = [times[i] / iters[i] for i in range(len(times))]
    per_iter.sort()
    return per_iter

BENCHMARKS = [
    ("bench_engine_step_once_save_const_single_transition", 
     "criterion-bench_engine_step_once_save_const_single_transition",
     "runtime_core/bench_engine_step_once_save_const_single_transition;profile=benc/new"),
    ("ipc_frame_decode",
     "criterion-ipc_frame_decode",
     "storage_ipc/ipc_frame_decode;profile=bench;tool=criterion-0.8;durability=mix/new"),
    ("engine_run_until_blocked_budget_10_small_workflow",
     "criterion-engine_run_until_blocked_budget_10_small_workflow",
     "runtime_core/engine_run_until_blocked_budget_10_small_workflow;profile=bench;/new"),
]

# Load benchmark-evidence.jsonl expected values
expected = {}
for line in EVIDENCE.read_text().splitlines():
    rec = json.loads(line)
    expected[rec["metric"]] = rec

results = []
for metric, criterion_dir, profile_path in BENCHMARKS:
    sample_path = CRITERION_HOME / criterion_dir / f"{profile_path}/sample.json"
    estimates_path = CRITERION_HOME / criterion_dir / f"{profile_path}/estimates.json"
    
    sample = json.loads(sample_path.read_text())
    estimates = json.loads(estimates_path.read_text())
    
    per_iter = parse_sample(sample)
    
    # Compute mean (ns)
    computed_mean_ns = mean(per_iter)
    # Criterion estimates mean is in ns
    estimates_mean_ns = estimates["mean"]["point_estimate"]
    
    # Percentiles from sample
    p50 = percentile(per_iter, 50)
    p95 = percentile(per_iter, 95)
    p99 = percentile(per_iter, 99)
    
    # Evidence values are in picoseconds
    ev = expected[metric]
    ev_picoseconds = ev["result_value"]
    ev_nanoseconds = ev_picoseconds / 1000.0
    
    print(f"=== {metric} ===")
    print(f"  Samples: {len(per_iter)}")
    print(f"  Computed mean (ns): {computed_mean_ns:.6f}")
    print(f"  Estimates.json mean (ns): {estimates_mean_ns:.6f}")
    print(f"  Evidence result (picoseconds): {ev_picoseconds}")
    print(f"  Evidence result (ns): {ev_nanoseconds:.6f}")
    print(f"  P50 (ns): {p50:.6f}")
    print(f"  P95 (ns): {p95:.6f}")
    print(f"  P99 (ns): {p99:.6f}")
    print()
    
    results.append({
        "metric": metric,
        "samples": len(per_iter),
        "computed_mean_ns": computed_mean_ns,
        "estimates_mean_ns": estimates_mean_ns,
        "evidence_ps": ev_picoseconds,
        "evidence_ns": ev_nanoseconds,
        "p50": p50,
        "p95": p95,
        "p99": p99,
    })

# Check if mean estimate matches evidence
print("=== Summary ===")
all_ok = True
for r in results:
    # The evidence value should be the mean estimate from estimates.json
    # in picoseconds (multiply ns by 1000 and round)
    expected_ps = round(r["estimates_mean_ns"] * 1000)
    actual_ps = r["evidence_ps"]
    match = expected_ps == actual_ps
    print(f"{r['metric']}: estimates_mean*1000={expected_ps}ps, evidence={actual_ps}ps, match={match}")
    if not match:
        all_ok = False

print(f"\nAll mean estimates match evidence: {all_ok}")

# Also check if computed mean from samples is close to estimates mean
print("\n=== Computed vs Estimates Mean (ns) ===")
for r in results:
    diff = abs(r["computed_mean_ns"] - r["estimates_mean_ns"])
    ratio = diff / r["estimates_mean_ns"] * 100
    print(f"{r['metric']}: computed={r['computed_mean_ns']:.4f}, estimates={r['estimates_mean_ns']:.4f}, diff={diff:.6f} ({ratio:.2f}%)")
