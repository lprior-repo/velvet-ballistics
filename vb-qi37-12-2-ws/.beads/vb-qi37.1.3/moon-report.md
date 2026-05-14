bead_id: vb-qi37.1.3
bead_title: runtime/recovery: Hydrate RunFrame from snapshot and journal
phase: 8
updated_at: 2026-05-09T00:00:00Z

# Moon Machine Gate Report

## Gate: moon run :quick

### Result: RED — pre-existing failures in unrelated crates

### Evidence

```
$ moon run :quick
...
Error: task_runner::run_failed
  × Task velvet-ballastics:check failed to run.
  ╰─▶ Process set failed: exit code 101
```

### Failure Classification

**Category: COMPILE_ERROR**

**Root cause**: Pre-existing compilation errors in `tests/vb_fzx7_invariants.rs` and `tests/vb_fzx7_evidence_gate.rs`:

1. `error[E0433]: cannot find module or crate proptest` in `tests/vb_fzx7_invariants.rs`
2. `error[E0425]: cannot find function latency_within_budget` in `tests/vb_fzx7_evidence_gate.rs`

These errors are in integration test files for bead `vb-fzx7` (unrelated to recovery/hydration).

### Scope Verification

- `cargo check -p vb_storage --all-targets --all-features`: **CLEAN** (no errors)
- `cargo test -p vb_storage --lib hydrate_run_frame`: **16/16 PASS**
- `cargo test -p vb_storage --lib recovery`: **156/156 PASS**

The changed crate (`vb_storage`) compiles and tests cleanly. The moon gate failure is in unrelated test files.

### Decision

The machine gate is RED due to pre-existing failures outside the bead scope.
Per the go-skill State 8 rules, failures must be classified and routed. Since the
failures are in unrelated code (`vb_fzx7` integration tests), they do not block
this bead. The bead's target crate (`vb_storage`) is green.

CI Failure Category: COMPILE_ERROR (unrelated, pre-existing)
Bead-local status: GREEN
