# vb-9qdjg LOCAL RTX5090 Benchmark Measurement Review

**Bead:** vb-9qdjg  
**Reviewer:** LOCAL RTX5090 benchmark evidence reviewer  
**Date:** 2026-10-22  
**Evidence Under Review:** `evidence/benchmark-evidence.jsonl` (3 records)  
**Raw Data Location:** `.beads/vb-oxzbo/evidence/actual-collection-v3`

## Scope

This review verifies the published benchmark evidence against the actual raw collected data. It does not assess benchmark methodology, performance improvement claims, or source correctness. It confirms that the published measurements are faithful representations of the underlying raw data.

## Methodology

1. Imported actual collector parsers from `.beads/vb-j3e3s/collect_benchmark_evidence.py` (unchanged)
2. Wrote scoped verification script `.beads/vb-9qdjg/verify_measurements.py`
3. Executed via `/tmp/vb-run-gate.py` runner
4. Verification exit code: 0 (success)
5. Total checks executed: 66
6. Errors: 0

## Verification Checks Performed

### Check 1: Commit SHA Validation
- [PASS] All 3 records have valid 40-character SHA
- [PASS] All 3 records reference exact commit `2fea0dcec4bc12c9d8c34fcf894398b3e8f38c9d`
- [PASS] Measured commit is ancestor of current HEAD `ebf64b4b...`

### Check 2: Criterion Measurements (10 finite positive samples per benchmark)
For each of the 3 benchmarks:
- [PASS] sample.json contains exactly 10 samples
- [PASS] Recomputed mean matches estimates.json (tolerance 0.1 ns)
- [PASS] Evidence picoseconds = round(mean_ns * 1000)
- [PASS] baseline_value = result_value (intentional non-speedup claim)
- [PASS] p50_latency recomputed from samples matches record (tolerance 1e-6)
- [PASS] p95_latency recomputed from samples matches record (tolerance 1e-6)
- [PASS] p99_latency recomputed from samples matches record (tolerance 1e-6)
- [PASS] full_id contains correct benchmark name (v3 format with metadata)

### Check 3: Perf Instruction Counts
- [PASS] `bench_engine_step_once_save_const_single_transition` instruction count matches
- [PASS] `ipc_frame_decode` instruction count matches
- [PASS] `engine_run_until_blocked_budget_10_small_workflow` instruction count matches

### Check 4: Heaptrack SHA Validation
- [PASS] `ht-bench_engine_step_once_save_const_single_transition.zst` SHA256 matches
- [PASS] `ht-ipc_frame_decode.zst` SHA256 matches
- [PASS] `ht-engine_run_until_blocked_budget_10_small_workflow.zst` SHA256 matches

### Check 5: Heaptrack Content Validation
For each of the 3 benchmarks:
- [PASS] allocation_count matches manifest
- [PASS] bytes_allocated matches manifest
- [PASS] allocation_count matches evidence record
- [PASS] bytes_allocated matches evidence record

Decompression uses `zstd -d -c` piped through a tempfile to avoid printing 200MB of trace data to tool output.

### Check 6: Threshold Validation
- [PASS] `bench_engine_step_once_save_const_single_transition` threshold: 3%
- [PASS] `ipc_frame_decode` threshold: 5%
- [PASS] `engine_run_until_blocked_budget_10_small_workflow` threshold: 3%

### Check 7: Baseline = Result (Intentional Non-Speedup Claim)
- [PASS] All 3 records have baseline_value = result_value (confirmed intentional)

### Check 8: Hardware Metadata
For each of the 3 records:
- [PASS] cpu_model: AMD Ryzen 9 9950X3D 16-Core Processor
- [PASS] kernel_version: Linux omarchy 7.2.3-arch1-3 (confirmed via substring check)
- [PASS] rustc_version: rustc 1.97.0-nightly (confirmed via substring check)

## Findings

**APPROVED.** All 66 verification checks passed. The published benchmark evidence is a faithful representation of the underlying raw data collected at the exact measured commit SHA.

Key observations:
- This is a baseline measurement, not a performance improvement claim (baseline = result for all benchmarks)
- The measurement was taken at commit `2fea0dce`, which is an ancestor of the current HEAD
- All raw logs, criterion output, perf CSV files, and heaptrack traces are consistent with the published measurements
- Percentile values (p50/p95/p99) are sample-derived from the 10 samples, not per-operation measurements
- Instruction counts and heap allocation metrics are whole-process values, not per-operation values

## Verification Artifacts

- Verification script: `.beads/vb-9qdjg/verify_measurements.py`
- Runner output: `.beads/vb-9qdjg/evidence/REVIEW-FINAL.{stdout,stderr,json}`