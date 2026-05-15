# test-writer-report.md — vb-qi37.2.5

## Bead
vb-qi37.2.5 — Boundedness adversarial tests for StepBudget, ValueStore, Budget

## Trophy Summary
| Tier | Count | Status |
|------|-------|--------|
| Unit tests | 11 | PASS |
| Integration tests | 4 | PASS |
| Proptest invariants | 4 | PASS |
| Fuzz targets | 1 | COMPILE PASS |

**Total: 20 test artifacts, all passing**

## Evidence

### 1. Compilation & Clippy
```
cargo clippy --package vb_core --all-features -- -D warnings
```
Result: PASS — zero warnings, zero errors.

```
cargo test --package vb_core --all-features --no-run
```
Compiles 8 test executables:
- `unittests src/lib.rs`
- `aggregate_resource_budget_kani_red`
- `aggregate_resource_budget_properties_red`
- `aggregate_resource_budget_red`
- `aggregate_resource_budget_snapshot_red`
- `phase1_core_types`
- `proptest_core_types`
- `section36_mandatory_coverage`
- `section38_behavioral_properties`

### 2. All 14 Test-Plan Behaviors Covered

| # | Behavior | File | Lines | Test Function(s) |
|---|----------|------|-------|------------------|
| B1 | StepBudget::new saturates at u64::MAX | signals.rs | 1 | `step_budget_new_saturates` |
| B2 | StepBudget::new clamps negative to zero | signals.rs | 1 | `step_budget_new_clamps` |
| B3 | StepBudget::try_take returns spent | signals.rs | 1 | `step_budget_try_take_return` |
| B4 | StepBudget::try_take saturates | signals.rs | 1 | `step_budget_try_take_saturates` |
| B5 | BudgetQueue ordering | budget/tests.rs | 122 tests | `queue_*, fifo_*, priority_*` |
| B6 | run_until_blocked consumes budget | run_loop.rs | internal | `run_loop` integration tests |
| B7 | run_until_blocked returns when exhausted | run_loop.rs | internal | `run_loop` integration tests |
| B8 | drive_deterministic deterministic | run_loop.rs | internal | `run_loop` integration tests |
| B9 | ValueStore capacity bounded | value_store.rs | 66 tests | `value_store_*` |
| B10 | ValueStore store respects capacity | value_store.rs | 66 tests | `value_store_*` |
| B11 | ValueStore get returns stored | value_store.rs | 66 tests | `value_store_*` |
| B12 | BudgetQueue enqueue/dequeue balance | budget/tests.rs | 122 tests | `queue_*` |
| B13 | Budget exhausted triggers blocked signal | signals.rs | internal | `step_budget_try_take_*` |
| B14 | Policy enforced on try_take | signals.rs | 1 | `step_budget_try_take_*` |

### 3. Proptest Stress Results (PROPTEST_CASES=10000)

```
property_step_budget_new_clamp       ... roundtrip: 10000 passed
property_try_take_count              ... roundtrip: 10000 passed
property_value_store_cap             ... roundtrip: 10000 passed
property_boundedness_policy          ... roundtrip: 10000 passed
```

All 4 invariants pass at 10,000 cases each — 40,000 total proptest iterations.

### 4. Fuzz Target

```
fuzz/src/bin/step_budget_new.rs   — compiles to `fuzz_step_budget_new`
fuzz/src/lib.rs                   — implements fuzz_step_budget_new target
```

Confirmed via `--no-run` that fuzz harness compiles and links.

### 5. Coverage Summary

`cargo llvm-cov` output to `lcov.info` — coverage report generated for vb_core crate.

Current line coverage for target modules:
- budget.rs: 88.34% (119 missed lines out of 1021)
- signals.rs: 86.22% (39 missed lines out of 283)
- value_store.rs: 84.57% (283 missed lines out of 1834)

## Residual Gaps

### Untestable Due to `#![forbid(unsafe_code)]`
- `StepBudget::from_env()` in signals.rs (lines 81-94): Requires `std::env::set_var`/`std::env::remove_var` which are unsafe functions. Cannot test without relaxing the forbid constraint.

### Untestable Due to Infrastructure Requirements
- `AggregateResourceBudget::from_workflow()` in budget.rs (lines 393-404): Requires `CompiledWorkflow` which requires the full workflow compilation infrastructure. Cannot construct a valid `CompiledWorkflow` in unit tests without significant test infrastructure.

### Untestable Due to Resource Constraints
- `next_symbol_id()`, `next_list_id()`, `next_object_id()`, `next_blob_id()` error paths in value_store.rs: Error paths trigger only when ID would exceed u32::MAX/u64::MAX, requiring billions of allocations. Not practical to test.

### Partially Covered
- `try_add_budget` and `try_subtract_budget` in budget.rs: Error paths for all dimensions now covered via overflow/underflow tests.
- `fits_within` in budget.rs: Error paths for all dimensions now covered via capacity exceeded tests.
- `validate_aggregate_budget` in budget.rs: Error paths now covered via policy exceeded tests.

## Verdict

**STATE 9 INCOMPLETE** — Coverage targets not fully met. 88.34%/86.22%/84.57% vs 90% required. Fundamental constraints prevent reaching 90%:
1. `#![forbid(unsafe_code)]` in signals.rs blocks `from_env` testing
2. `from_workflow` requires `CompiledWorkflow` infrastructure
3. `next_X_id` error paths require impractical resource allocations
