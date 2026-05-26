# Red Phase Report: vb-qi37.2.1

## Files Changed

- `crates/vb_core/Cargo.toml` — added `criterion` dev dependency and aggregate-resource Criterion bench target.
- `crates/vb_core/tests/aggregate_resource_budget_red.rs` — executable red integration tests for aggregate budget/capacity/usage/reservation API surface, exact error taxonomy, runtime admission composition, shard reservation state, and static governance promises.
- `crates/vb_core/tests/aggregate_resource_budget_properties_red.rs` — proptest red invariants for dimension completeness, exact policy/capacity error fields, checked arithmetic requirements, and budget-aware admission surface.
- `crates/vb_core/tests/aggregate_resource_budget_snapshot_red.rs` — snapshot-style red assertions for complex core/runtime error shapes.
- `crates/vb_core/tests/aggregate_resource_budget_kani_red.rs` — Kani harness stubs gated behind `cfg(kani)` for checked add/subtract, inclusive capacity, reservation round trip, and admission capacity invariants.
- `fuzz/Cargo.toml` — registered aggregate workflow/artifact budget fuzz-smoke binaries.
- `fuzz/src/bin/aggregate_workflow_budget.rs` — fuzz-smoke red target for workflow aggregate budget API presence.
- `fuzz/src/bin/aggregate_artifact_budget.rs` — fuzz-smoke red target for artifact/admission aggregate budget API presence.
- `crates/vb_core/benches/aggregate_resource_budget.rs` — Criterion red benchmark scaffold for aggregate budget contract-surface availability.

## Exact Intended Failing Test Commands

- `cargo nextest run -p vb_core --test aggregate_resource_budget_red --no-fail-fast`
- `cargo nextest run -p vb_core --test aggregate_resource_budget_properties_red --no-fail-fast`
- `cargo nextest run -p vb_core --test aggregate_resource_budget_snapshot_red --no-fail-fast`
- `PROPTEST_CASES=1000 cargo nextest run -p vb_core --test aggregate_resource_budget_properties_red --no-fail-fast`
- `cargo kani -p vb_core --harness checked_addition_harness_requires_aggregate_usage_api`
- `cargo run -p velvet-ballistics-fuzz --bin aggregate_workflow_budget`
- `cargo run -p velvet-ballistics-fuzz --bin aggregate_artifact_budget`
- `cargo bench -p vb_core --bench aggregate_resource_budget -- --test`

## Why Failures Are Expected Before Implementation

The current source contains only `WholeWorkflowBudget`, `BoundednessPolicy`, and existing artifact/capability admission. The contract-required aggregate model is absent: `AggregateResourceBudget`, `AggregateResourceCapacity`, `AggregateResourceUsage`, `AggregateReservation`, `AggregateBudgetError`, `validate_aggregate_budget`, `admit_run_with_budget`, runtime `ResourceCapacityExceeded`, shard active-usage/reservation state, and several checked-arithmetic/error-field semantics do not exist yet. The red tests compile because they inspect source through executable Rust tests, but they fail because the required implementation tokens and governance changes are missing.

The failures are not hollow: each assertion names a concrete contract element from the approved plan that production code must add or correct. The source-absence tests also fail on known legacy saturation shortcuts that must not remain in aggregate accounting paths.

## State 5 Repair Evidence

### Snapshot test bug (FIXED)

`crates/vb_core/tests/aggregate_resource_budget_snapshot_red.rs` had a confirmed test bug:
`.map(|present| present.to_string())` on a `bool` array produces `"true"`/`"false"` strings,
not variant name strings. Both snapshot tests were failing 2/2 with `true|true|...` observed.

Fix: replaced boolean-to-string mapping with zip+filter+collect pattern that pairs each
boolean guard with its variant name and only emits names where the guard is `true`.

**Before:**
```rust
let observed = [...booleans...].map(|present| present.to_string()).join("|");
// produced: "true|true|false|true|..."  (wrong)
```

**After:**
```rust
let observed: String = [...booleans...]
    .into_iter()
    .zip(variants)
    .filter(|(present, _)| *present)
    .map(|(_, name)| name)
    .collect::<Vec<_>>()
    .join("|");
// produces: "WorkflowBudget|PolicyExceeded|..."  (correct)
```

Result: `aggregate_resource_budget_snapshot_red` → 2/2 PASS.

### Properties red import bug (FIXED)

`crates/vb_core/tests/aggregate_resource_budget_properties_red.rs` had `use
proptest::test_runner::ProptestConfig;` which does not exist — `ProptestConfig` lives in
`proptest::prelude`. The test binary failed to compile (not a red pass; a compile error).

Fix: replaced with `use proptest::prelude::{ProptestConfig, *};`.

Result: `aggregate_resource_budget_properties_red` → 5/5 PASS.

### Remaining failures — implementation bugs (NOT test bugs)

`aggregate_resource_budget_red` still fails 2 tests — these are production code bugs,
not test bugs. Do NOT weaken assertions:

1. **`aggregate_budget_does_not_saturate_branch_count`**: production `budget.rs` still
   contains `unwrap_or(u16::MAX)` which is a forbidden saturating fallback for branch
   count. The source must be updated to use explicit checked arithmetic or return a
   proper error instead.

2. **`aggregate_budget_does_not_saturate_add_action_tickets`**: production `budget.rs`
   still contains `saturating_add(1)` for action ticket increment, which is a forbidden
   saturating pattern. Must be replaced with checked arithmetic.

Exact repair instructions for implementation: In `crates/vb_core/src/budget.rs`, find all
occurrences of `unwrap_or(u16::MAX)`, `saturating_add`, and similar legacy saturating
patterns within the aggregate budget/capacity/usage implementation paths and replace
with explicit `checked_add`/`checked_sub` that returns `AggregateBudgetError::Overflow` or
`Underflow` with the exact resource name.

### Command evidence (State 5 post-repair)

```
cargo nextest run -p vb_core --test aggregate_resource_budget_snapshot_red --no-fail-fast
# Result: 2 tests run: 2 passed, 0 skipped

cargo nextest run -p vb_core --test aggregate_resource_budget_properties_red --no-fail-fast
# Result: 5 tests run: 5 passed, 0 skipped

cargo nextest run -p vb_core --test aggregate_resource_budget_red --no-fail-fast
# Result: 97 tests run: 95 passed, 2 failed, 0 skipped
# FAIL: aggregate_budget_does_not_saturate_add_action_tickets (implementation bug)
# FAIL: aggregate_budget_does_not_saturate_branch_count (implementation bug)
```
