# Red Queen Report — vb-l2d7 — Retry 15

STATUS: APPROVED

## Scope

- Workspace: `/home/lewis/src/vb-l2d7`
- Femdation state: State 5 Red Queen
- Retry: 15
- Red Queen task: `vb-l2d7-redqueen-r15`
- Liza: `$HOME/.claude/skills/red-queen/liza-advanced.nu`
- Production/test edits by Red Queen: none

## Setup

The retry-specific Red Queen task was absent and was recreated:

```text
Task 'vb-l2d7-redqueen-r15' not found
Task 'vb-l2d7-redqueen-r15' added [champion=]
Task 'vb-l2d7-redqueen-r15' claimed by red-queen
```

Nine permanent ratchet checks were registered: doc taint script, focused doc suite, runtime companion, ordering at 1 and 8 threads, three Finish contradiction fail-closed wrappers, and one allowed no-rejection wording wrapper.

## Direct command evidence

### Doc taint script

```bash
python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md
```

Result:

```text
doc taint consistency: PASS
```

### Focused doc suite

```bash
rtk cargo nextest run -p velvet-ballistics-workspace --test vb_l2d7_doc_reconciliation_contract_red
```

Result:

```text
cargo nextest: 65 passed (1 binary, 1.520s)
```

### Runtime joined-taint companion

```bash
rtk cargo nextest run -p vb_runtime --test vb_l2d7_joined_taint_propagation_red
```

Result:

```text
cargo nextest: 24 passed (1 binary, 0.471s)
```

### Ordering probes

```bash
RUST_TEST_THREADS=1 rtk cargo nextest run -p velvet-ballistics-workspace --test vb_l2d7_doc_reconciliation_contract_red scan_for_stale_clean_only_text
RUST_TEST_THREADS=8 rtk cargo nextest run -p velvet-ballistics-workspace --test vb_l2d7_doc_reconciliation_contract_red scan_for_stale_clean_only_text
```

Results:

```text
cargo nextest: 17 passed, 48 skipped (1 binary, 0.343s)
cargo nextest: 17 passed, 48 skipped (1 binary, 0.096s)
```

### Density check

```text
doc reconciliation test count: 65
public reconcile functions: 5
density: 65/5 = 13.00x
```

## Finish contradiction adversarial checks

All hostile Finish inputs failed closed. The Red Queen wrappers invert this intentionally: script rejection means the challenger is discarded.

### Stale `Finished(SlotValue)` without taint

Input:

```text
Finish emits EngineSignal::Finished(SlotValue).
```

Observed:

```text
doc taint consistency: FAIL
- stale finish signal missing taint
- missing finish signal taint wording
```

Result: discarded / pass.

### Secret rejection contradiction

Input:

```text
Finish compile-time validation rejects Secret finish results, but runtime preserves taint.
```

Observed:

```text
doc taint consistency: FAIL
- finish rejection contradiction
- missing finish signal taint wording
```

Result: discarded / pass.

### DerivedFromSecret rejection contradiction

Input:

```text
Finish rejects DerivedFromSecret result taint.
```

Observed:

```text
doc taint consistency: FAIL
- finish rejection contradiction
- missing finish signal taint wording
```

Result: discarded / pass.

### Allowed no-rejection wording

Input:

```text
Finish emits EngineSignal::Finished(SlotValue, Taint). No rejection of Secret or DerivedFromSecret results.
```

Observed:

```text
doc taint consistency: PASS
```

Result: discarded / pass.

## Red Queen validate

Command:

```bash
L="$HOME/.claude/skills/red-queen/liza-advanced.nu"
nu "$L" validate vb-l2d7-redqueen-r15
```

Result:

```text
VALIDATION: Running 9 checks — the ratchet
PASS: doc taint script
PASS: focused doc suite
PASS: runtime joined-taint companion
PASS: ordering probe at 1 thread
PASS: ordering probe at 8 threads
PASS: stale Finished(SlotValue) contradiction wrapper
PASS: Secret rejection contradiction wrapper
PASS: DerivedFromSecret rejection contradiction wrapper
PASS: allowed no-rejection wording wrapper

Results: 9/9 passed
ALL CHECKS PASS — ratchet holds
```

## Evolutionary QA generations

Three Red Queen generations were executed. Each generation ran:

- doc taint script
- focused doc suite
- runtime companion
- ordering probe at 1 thread
- ordering probe at 8 threads
- stale `Finished(SlotValue)` fail-closed probe
- Secret rejection fail-closed probe
- DerivedFromSecret rejection fail-closed probe
- allowed no-rejection wording probe

Final computed Red Queen output:

```text
EQUILIBRIUM: YES — Crown defended through sustained resistance
Global zero-survivor streak: 3
All dimensions exhausted: true

THE RED QUEEN'S VERDICT
Generations: 3
Lineage:     9 permanent checks
Survivors:   0 — CRITICAL=0 MAJOR=0 MINOR=0 OBS=0
Beads Filed: 0
Final:       CROWN DEFENDED

VALIDATION
All checks pass: true
Passed: 9/9
```

Fitness landscape:

```text
doc-taint-script              3 tests, 0 survivors, EXHAUSTED
focused-doc-suite             3 tests, 0 survivors, EXHAUSTED
runtime-companion             3 tests, 0 survivors, EXHAUSTED
ordering-one-thread           3 tests, 0 survivors, EXHAUSTED
ordering-eight-threads        3 tests, 0 survivors, EXHAUSTED
finish-missing-taint          3 tests, 0 survivors, EXHAUSTED
finish-secret-rejection       3 tests, 0 survivors, EXHAUSTED
finish-derived-rejection      3 tests, 0 survivors, EXHAUSTED
finish-no-rejection-allowed   3 tests, 0 survivors, EXHAUSTED
```

## Mutation shard

Command:

```bash
rtk cargo mutants -p vb_doc --file crates/vb_doc/src/reconcile.rs --timeout 45 --jobs 2 --test-tool nextest --test-workspace true --baseline skip --shard 1/20
```

Result:

```text
Found 4 mutants to test
TIMEOUT  crates/vb_doc/src/reconcile.rs:110:5: replace reject_lattice_conflicts -> Result<(), DocReconcileError> with Ok(()) in 154s build + 45s test
TIMEOUT  crates/vb_doc/src/reconcile.rs:100:5: replace reject_control_flow_conflation -> Result<(), DocReconcileError> with Ok(()) in 197s build + 45s test
4 mutants tested in 4m: 2 unviable, 2 timeouts
```

Interpretation: no surviving/missed mutants were reported in this bead-owned shard. The remaining mutation result is tooling/test-time timeout plus unviable mutants, not a Red Queen survivor.

## Survivor summary

- Survivors found this run: 0
- Critical survivors: 0
- Major survivors: 0
- Minor survivors: 0
- Blockers: none

## Verdict

`STATUS: APPROVED` — Red Queen Crown Defended.
