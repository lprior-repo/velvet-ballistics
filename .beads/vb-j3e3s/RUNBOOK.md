# vb-j3e3s Benchmark Evidence Collection Runbook

Compact real fail-closed collector. Inspects actual tool formats before writing
evidence. No fallbacks, no invented JSON schemas.

## Actual Tool Formats Verified

- **criterion 0.8.2**: writes to `target/criterion/<dir>/new/`
  - `benchmark.json`: full_id matching benchmark name
  - `sample.json`: `{sampling_mode, iters: [f64], times: [f64]}` (ns)
  - `estimates.json`: `{mean: {point_estimate: f64}, median: {point_estimate: f64}}` (ns)
- **perf stat -x ';'**: CSV format `count;;event;total;percent;;`
- **heaptrack**: produces `.zst` files; `heaptrack_print` text has
  `calls to allocation functions: N` and `peak heap memory consumption: X` lines

## Prerequisites

- Final frozen source merged to `main`
- Clean workspace: `jj status` shows no uncommitted changes
- Compiled benchmark binary (collector auto-discovers via `cargo bench --no-run`)
- Required tools: `perf`, `heaptrack`, `heaptrack_print`, `sha256sum`, `python3`
- CPU governor: `performance`

## Running the Collector

```bash
python3 .beads/vb-j3e3s/collect_benchmark_evidence.py \
  --commit <FROZEN_COMMIT_SHA> \
  --root $(pwd) \
  --output-dir .beads/vb-j3e3s/evidence
```

Optional: `--binary <path>` to override auto-discovery.

## Collector Behavior

For each benchmark from `contracts/perf-budget.yaml`:

1. **Criterion timing**: `cargo bench` with 10 samples, 1s warm-up, 1s measurement
   - Parses `estimates.json` for mean/median (ns)
   - Parses `sample.json` for per-iteration times, computes p50/p95/p99 via linear interpolation
2. **perf stat**: `perf stat -x ';' -e instructions`
   - Parses CSV: first field is count, second field has event name
   - Rejects missing/zero/unknown counts
3. **heaptrack**: `heaptrack --record-only` + `heaptrack_print` text
   - Finds `.zst` output file
   - Parses `calls to allocation functions:` and `peak heap memory consumption:` lines

## Fail-Closed Behavior

Collector exits nonzero on any measurement failure:
- Commit mismatch (expected vs actual HEAD)
- Dirty working tree
- Criterion JSON not found or parse error
- Insufficient samples (< 1)
- Perf CSV parse error or missing instruction counter
- Heaptrack `.zst` file not found or text parse error
- Missing required fields in evidence record

No partial results written. No `None`/`nan`/`skipped` fallback values.

## Output

- `.beads/vb-j3e3s/evidence/benchmark-evidence.jsonl`: one JSONL record per benchmark
- `.beads/vb-j3e3s/evidence/benchmark-raw-logs.txt`: combined raw criterion/perf/heaptrack output
- Old evidence preserved under `.beads/vb-j3e3s/evidence/raw-historical/`

Each record contains all required fields from `contracts/perf-budget.yaml`:
metric, claim, baseline_value, result_value, unit, threshold_percent, raw_log,
command, commit, rustc_version, nightly_date, cpu_model, cpu_governor,
kernel_version, build_profile, rustflags, benchmark_tool_version,
sample_count_or_instruction_count, input_fixture_digest, durability_profile,
mode, p50_latency, p95_latency, p99_latency, instruction_count,
allocation_count, bytes_allocated, fjall_write_latency, direct_api_latency,
ipc_latency.

## Post-Collection Validation

```bash
# Verify record count (should be 3)
wc -l .beads/vb-j3e3s/evidence/benchmark-evidence.jsonl

# Validate JSON syntax
python3 -c "
import json
with open('.beads/vb-j3e3s/evidence/benchmark-evidence.jsonl') as f:
    for i, line in enumerate(f, 1):
        rec = json.loads(line)
        assert rec['commit'] and rec['binary_sha'], f'record {i} missing fields'
        print(f'Record {i}: {rec[\"command\"][:50]}... OK')
"

# Run policy checker
moon run :benchmark-regression-policy
```

## Tests

`python3 .beads/vb-j3e3s/test-collector-synthetic.py` covers:
- Real criterion sample/estimates JSON parsing
- Real perf CSV format parsing
- Real heaptrack text format parsing
- Fail-closed: missing raw log, commit mismatch, dirty tree, missing counter
- All required evidence fields present
- Benchmark IDs match `contracts/perf-budget.yaml`

## Duration Estimate

- Criterion: ~30s per benchmark
- perf stat: ~30s per benchmark
- heaptrack: ~60s per benchmark
- **Total: ~6-8 minutes for 3 benchmarks**