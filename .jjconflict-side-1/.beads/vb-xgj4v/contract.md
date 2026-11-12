# Contract: Pass Criterion baseline options only to actual benchmark executables

## Scope
Only `.moon/tasks/all.yml` `benchmark-proof` task and a small fix to `crates/vb_validate/Cargo.toml` to declare the undeclared Criterion benchmark. No source or proof changes. No gate weakening.

## Problem
Current `benchmark-proof` task runs:
```
cargo bench --workspace --all-features -- --save-baseline vb-current
```
The `--save-baseline vb-current` is a Criterion-specific option. `crates/vb_validate/benches/capability_schema.rs` is a Criterion benchmark that is NOT declared in `Cargo.toml` with `harness = false`, so Cargo treats it as a libtest benchmark. Libtest rejects `--save-baseline` with exit code 101.

Additionally, `--workspace` is imprecise: it targets everything including tests and fuzz targets that are not Criterion benchmarks.

## Current Target Inventory

Criterion benchmarks (harness = false, declared):
- `vb_core`: aggregate_resource_budget
- `workspace_tests`: velvet_ballistics, vb_qi37_1_1_recovery, vb_kkvb_xtask_routing_red, ir_traversal, collect_page, action_dispatch, memory_footprint, cold_start, pagination_cost, action_queuing, timer_wheel_tick, snapshot_save, snapshot_restore, array_queue, rtrb

Criterion benchmark NOT declared (treated as libtest):
- `vb_validate`: capability_schema.rs (uses `use criterion`)

## Acceptance Criteria

### Declare vb-validate benchmark correctly
- Add `[[bench]]` declaration to `crates/vb_validate/Cargo.toml` with `harness = false`
- This allows Criterion options to pass through correctly

### Target explicit Criterion benchmarks
- Replace `--workspace` with explicit target selection
- Run only the current-scope Criterion benchmarks
- Do not pass Criterion options to libtest targets or fuzz targets

### Preserve task semantics
- `runInCI: false` (optional full baseline task)
- `bench-build` and `benchmark-regression-policy` remain blocking and unchanged
- Keep `--save-baseline vb-current` for Criterion targets
- Add timeout <= 300 seconds via the official runner

### Honest benchmark invocation
- Command targets actual Criterion benchmarks only
- No fabricated baseline or performance claims
- Results preserved as before

## Implementation Plan

### Fix vb_validate/Cargo.toml
```toml
[[bench]]
name = "capability_schema"
harness = false
```

### Updated benchmark-proof task
```yaml
benchmark-proof:
  script: |
    set -euo pipefail
    echo "Criterion benchmarks for vb-current baseline..."
    timeout 300 rustup run nightly-2026-04-28 cargo bench --no-default-features \
      -p vb_core --bench aggregate_resource_budget \
      -p vb_validate --bench capability_schema \
      -p workspace_tests --bench velvet_ballistics \
      -- --save-baseline vb-current
  toolchains:
    - rust
  inputs:
    - '@globs(sources)'
    - '@globs(configs)'
  options:
    runInCI: false
```

## Verification
- Run `moon run benchmark-proof` on isolated workspace
- Verify `--save-baseline vb-current` is accepted by all targets
- Verify no libtest or fuzz target rejection
- Verify baseline file `target/criterion/vb-current` is written
- Confirm bench-build and benchmark-regression-policy still run independently

## Constraints
- No new shell scripts
- Bounded to `.moon/tasks/all.yml` and one Cargo.toml fix
- Preserve all other CI tasks unchanged
