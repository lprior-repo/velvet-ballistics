# vb-j3e3s Benchmark Evidence Collection Runbook

## Prerequisites

- Final frozen source merged to `main` (policy/format/checker changes complete)
- Clean workspace: `git status` shows no uncommitted changes
- Compiled benchmark binary available:
  ```bash
  cargo bench --package velvet-ballistics-workspace-tests \
    --bench velvet_ballistics --no-run --release
  ```
- Required tools installed: `perf`, `heaptrack`, `sha256sum`, `python3`
- CPU governor set to `performance` for reproducible measurements

## Step 1: Preserve Historical Evidence

Before writing fresh measurements, preserve the original evidence:

```bash
mkdir -p .beads/vb-j3e3s/evidence/raw-historical
cp evidence/benchmark-evidence.jsonl \
   .beads/vb-j3e3s/evidence/raw-historical/benchmark-evidence-historical.jsonl
cp evidence/benchmark-logs/*.log \
   .beads/vb-j3e3s/evidence/raw-historical/
```

## Step 2: Run the Collector

Invoke the collector from the isolated workspace root:

```bash
python3 .beads/vb-j3e3s/collect_benchmark_evidence.py \
  --commit <FROZEN_COMMIT_SHA> \
  --root $(pwd) \
  --output-dir .beads/vb-j3e3s/evidence \
  --binary target/release/deps/velvet_ballistics-<hash>
```

If binary path is unknown, omit `--binary` and the collector will auto-discover it via `cargo bench --no-run`.

### Collector Behavior

For each of the three budgeted benchmarks:
1. **Criterion timing**: Runs with `--sample-size 10 --warm-up-time 1 --measurement-time 1`
2. **perf instruction count**: Runs `perf stat -e instructions` on the same invocation
3. **heaptrack allocations**: Runs `heaptrack --record-only` + `heaptrack_print`

Outputs a JSONL file at `--output-dir/benchmark-evidence.jsonl` with all required fields.

## Step 3: Validate Output

After collection, validate the evidence file:

```bash
# Check forbidden markers are absent
grep -E "not-collected|allocator-external" .beads/vb-j3e3s/evidence/benchmark-evidence.jsonl && echo "FAIL: markers present" || echo "PASS: no markers"

# Verify record count (should be 3)
wc -l .beads/vb-j3e3s/evidence/benchmark-evidence.jsonl

# Validate JSON syntax
python3 -c "
import json
with open('.beads/vb-j3e3s/evidence/benchmark-evidence.jsonl') as f:
    for i, line in enumerate(f, 1):
        json.loads(line)
        print(f'Record {i}: OK')
"
```

## Step 4: Copy Evidence to Canonical Location

After validation, move the fresh evidence to the canonical location:

```bash
cp .beads/vb-j3e3s/evidence/benchmark-evidence.jsonl evidence/
```

The policy checker will verify `evidence/benchmark-evidence.jsonl` at the repository root.

## Dry-Run Mode

To validate setup without running actual benchmarks:

```bash
python3 .beads/vb-j3e3s/collect_benchmark_evidence.py \
  --commit <SHA> --root $(pwd) --dry-run
```

This verifies:
- Required tools are available
- Benchmark binary exists and is valid
- System metadata collection works

## Error Handling

The collector will fail fast and exit nonzero if:
- Any required tool is missing
- Benchmark binary is not found
- Criterion benchmark fails to complete
- Forbidden placeholder markers found in raw logs
- perf or heaptrack output cannot be parsed

Partial results are not written; the entire collection must succeed.

## Expected Duration

- Criterion timing: ~30s per benchmark (3 benchmarks = ~90s)
- perf stat: ~30s per benchmark (~90s)
- heaptrack: ~60s per benchmark (~180s)
- **Total estimated: 5-7 minutes**
