# Independent Local RTX3090 Measurement Review: vb-j3e3s (v3)

**Bead:** vb-3iflr (independent measurement review)  
**Parent:** vb-j3e3s (actual benchmark collection)  
**Reviewer:** Local RTX3090 agent  
**Worktree:** /home/lewis/src/isoloated/velvet-ballistics-vb-j3e3s-local3090-measurement-review  
**Measured commit:** 534aa40434b2704ad63666130557937087796259  
**Current HEAD:** 33f00081 (measured commit is ancestor via git merge-base --is-ancestor)

## Methodology

Bounded actual-collector assertion verification. Imported production collector parsers via sys.path, read published evidence, asserted every measurable claim against raw data. Non-zero exit on any mismatch. No VCS mutations, no product edits, no new measurements.

**Validation command:** `python3 /tmp/vb-run-gate.py 180 .beads/vb-3iflr/evidence/final-v3-assertion-verification-v1 python3 .beads/vb-3iflr/evidence/final-v3-assertion-verification-v1.py`  
**Exit code:** 0  
**Duration:** 6.427s  
**Log:** `.beads/vb-3iflr/evidence/final-v3-assertion-verification-v1.stdout`  
**Script:** `.beads/vb-3iflr/evidence/final-v3-assertion-verification-v1.py`

## Results: ALL CHECKS PASSED (57/57)

### Commit validation
- All 3 records carry full 40-char SHA: `534aa40434b2704ad63666130557937087796259`
- Measured commit is ancestor of current HEAD (git merge-base)

### Criterion measurements (recomputed from raw sample.json)
| Benchmark | Samples | Computed mean (ns) | Evidence (ps) | Match |
|-----------|---------|-------------------|---------------|-------|
| bench_engine_step_once_save_const_single_transition | 10 | 16.448097 | 16448 | ✓ |
| ipc_frame_decode | 10 | 79.983292 | 79983 | ✓ |
| engine_run_until_blocked_budget_10_small_workflow | 10 | 16.561120 | 16561 | ✓ |

All percentiles (p50/p95/p99) recomputed from raw samples match evidence within 1e-6 ns. Baselines equal results intentionally (no speedup claim).

### Perf instruction counts (parsed from raw CSV)
| Benchmark | Perf CSV | Evidence | Match |
|-----------|----------|----------|-------|
| bench_engine | 42057614923 | 42057614923 | ✓ |
| ipc_frame_decode | 55366692311 | 55366692311 | ✓ |
| engine_run | 41875073468 | 41875073468 | ✓ |

### Heaptrack trace validation
All 3 compressed traces verified:
- SHA256 matches manifest
- Decompressed and parsed via collector's parse_heaptrack_trace()
- Allocation counts and total bytes match manifest and evidence records

| Benchmark | Allocs | Bytes | Match |
|-----------|--------|-------|-------|
| bench_engine | 603186 | 115639383 | ✓ |
| ipc_frame_decode | 14173949 | 407808347 | ✓ |
| engine_run | 600322 | 115488741 | ✓ |

### Thresholds
- bench_engine: 3% ✓
- ipc_frame_decode: 5% ✓
- engine_run: 3% ✓

### Collector nested discovery fix
Commit c1109ccfe introduced one-line fix: `home.glob("*/new/benchmark.json")` → `home.glob("*/**/new/benchmark.json")`. Previously reviewed and verified unchanged. Separate 80-test regression suite in v1 log remains valid.

## Limits
1. **No speedup claim**: Baseline equals result intentionally; no regression comparison required.
2. **Whole-process counts**: Instruction and allocation totals are process-wide, not per-operation.
3. **10 samples**: Derived from per-iteration sample timings; not individual-operation tail observations.
4. **No re-measurement**: Verified collected evidence only.
5. **Heaptrack peak not compared**: Task specifies total allocated bytes; peak values parsed but manifest lacks peak field.

## Conclusion
**PASS** — All 57 actual-collector assertions verified against real raw data. Evidence is correct, complete, and consistent with the published benchmark-evidence.jsonl records.