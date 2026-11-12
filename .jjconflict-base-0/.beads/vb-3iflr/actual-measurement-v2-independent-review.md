# Independent Local RTX3090 Measurement Review: vb-j3e3s

**Bead:** vb-3iflr (independent measurement review)  
**Parent:** vb-j3e3s (actual benchmark collection)  
**Reviewer:** Local RTX3090 agent  
**Worktree:** /home/lewis/src/isoloated/velvet-ballistics-vb-j3e3s-local3090-measurement-review  
**Git root:** /home/lewis/src/isoloated/velvet-ballistics-vb-j3e3s-local3090-measurement-review  
**JJ root:** /home/lewis/src/isoloated/velvet-ballistics-vb-j3e3s-local3090-measurement-review  
**Current HEAD:** 172b80dae (ancestor of origin/main)  
**Measured commit:** be63eeb5380b5b20dfbdfb6017c6a00e9857e3dd (ancestor of HEAD)

## Scope

Bounded read-only verification of the published measurement evidence at `evidence/benchmark-evidence.jsonl`. No VCS mutations, no product edits, no new measurements. All evidence is derived from the root agent's actual collection in `.beads/vb-j3e3s/evidence/actual-collection-v2/`.

## Findings

### 1. Commit Lineage: PASS

- Measured commit `be63eeb5380b5b20dfbdfb6017c6a00e9857e3dd` is an ancestor of current HEAD.
- All 3 benchmark-evidence.jsonl records carry the full 40-character SHA.
- No production source code changed between measured commit and current HEAD; only `.beads/` and `evidence/` directories differ.

### 2. Criterion Measurements: PASS

For all 3 benchmarks, recomputed from preserved `sample.json` files in `.beads/vb-j3e3s/evidence/actual-collection-v2/scratch/criterion-home/`:

| Benchmark | Samples | Computed Mean (ns) | Estimates.json Mean (ns) | Evidence (ps) | Match |
|-----------|---------|-------------------|--------------------------|---------------|-------|
| bench_engine_step_once_save_const_single_transition | 10 | 16.410167 | 16.410167 | 16410 | ✓ |
| ipc_frame_decode | 10 | 79.268548 | 79.268548 | 79269 | ✓ |
| engine_run_until_blocked_budget_10_small_workflow | 10 | 16.424707 | 16.424707 | 16425 | ✓ |

- Computed mean matches estimates.json mean exactly (0.00% diff) for all 3.
- Estimates.json mean × 1000 matches evidence picoseconds value for all 3.
- Percentiles from sample.json match evidence records:
  - bench_engine: p50=16.405266, p95=16.458492, p99=16.468526 ns (all match)
  - ipc_frame_decode: p50=79.033471, p95=80.872969, p99=81.266789 ns
  - engine_run: p50=16.432691, p95=16.500834, p99=16.520323 ns

### 3. Perf Instruction Counts: PASS

Perf CSV files in `.beads/vb-j3e3s/evidence/actual-collection-v2/perf/` match evidence records exactly:

| Benchmark | Perf CSV | Evidence | Match |
|-----------|----------|----------|-------|
| bench_engine_step_once_save_const_single_transition | 41966253832 | 41966253832 | ✓ |
| ipc_frame_decode | 54922616044 | 54922616044 | ✓ |
| engine_run_until_blocked_budget_10_small_workflow | 41892067381 | 41892067381 | ✓ |

### 4. Heaptrack Trace SHA Verification: PASS

All 3 compressed heaptrack traces match their manifest SHA256:

| File | Manifest SHA | Computed SHA | Match |
|------|--------------|--------------|-------|
| ht-bench_engine_step_once_save_const_single_transition.zst | 3a6f34a6... | 3a6f34a6... | ✓ |
| ht-ipc_frame_decode.zst | 6cddfd80... | 6cddfd80... | ✓ |
| ht-engine_run_until_blocked_budget_10_small_workflow.zst | 41b55602... | 41b55602... | ✓ |

### 5. Heaptrack Trace Content Verification: PASS

Decompressed all 3 .zst traces and parsed via collector's `parse_heaptrack_trace()` function (v3 format). Allocation counts and total allocated bytes match manifest exactly:

| Benchmark | Manifest Allocs | Parsed Allocs | Manifest Bytes | Parsed Bytes | Match |
|-----------|-----------------|---------------|----------------|--------------|-------|
| bench_engine | 617975 | 617975 | 116410697 | 116410697 | ✓ |
| ipc_frame_decode | 14264657 | 14264657 | 410811387 | 410811387 | ✓ |
| engine_run | 615375 | 615375 | 116251032 | 116251032 | ✓ |

### 6. Bounded Collector Nested Discovery: PASS

Commit c1109ccfe introduced one-line fix to collector:
- Old: `home.glob("*/new/benchmark.json")` (flat)
- New: `home.glob("*/**/new/benchmark.json")` (recursive)

This correctly handles Criterion's nested output structure `{bench}/{group}/{func}/new/benchmark.json`.

Three new test functions verify the fix:
- `t_criterion_nested_group_directory_found`: Recursive discovery finds nested results
- `t_criterion_base_stale_ignored`: Stale `base/` results are rejected
- `t_criterion_multiple_matches_rejected`: Multiple matching `new/` results are rejected

### 7. Test Suite Execution: PASS

Ran `python3 .beads/vb-j3e3s/test-collector-synthetic.py` via `/tmp/vb-run-gate.py`:
- **80 tests passed, 0 failed**
- Exit code: 0
- Execution time: 0.115 seconds

Note: 80 test *invocations* (check/raises calls), not 80 unique test functions. 57 test function definitions; some functions call `raises()` internally for multiple cases, each counted separately.

### 8. Evidence Completeness: PASS

All 3 benchmark-evidence.jsonl records contain required fields:
- metric, claim, baseline_value, result_value, unit, threshold_percent
- raw_log, command, commit (full 40-char SHA)
- instruction_count, allocation_count, bytes_allocated
- p50/p95/p99 latency percentiles
- Sample count (10 each via criterion)
- System metadata (CPU, governor, kernel, rustc version)

## Limits and Assumptions

1. **No speedup claim verified**: Baseline equals result intentionally; no regression comparison required.
2. **Instruction/allocation totals are whole-process**: Not per-operation counts.
3. **Percentiles from 10 samples**: Derived from per-iteration sample timings, not individual-operation tail observations.
4. **No independent re-measurement**: Verified collected evidence only; did not rerun benchmarks.
5. **Raw perf CSV preserved**: Instruction counts verified against raw CSV files, not just JSONL records.
6. **Heaptrack peak values not verified**: Task specifies total allocated bytes, not peak. Peak values available in parsed output but not compared to manifest (manifest lacks peak field).

## Conclusion

**PASS** — All measurement evidence verified against actual collected data. Criterion means match estimates, instruction counts match perf CSV, heaptrack traces match manifest SHA and content, collector fix is correct and tested, test suite passes (80/80).
