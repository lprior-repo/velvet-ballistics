# vb-qi37.2.1 STATE

- Current State: State 5 (Test repair — vb-qi37.2.1)
- Title: runtime: Define aggregate resource budget model
- Branch/Workspace: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`
- Bookmark: `femdation-p0-p1-25`
- Claim Evidence: `bd update vb-qi37.2.1 --claim` succeeded from `/home/lewis/src/Velvet-ballistics`
- Next Gate: Fix implementation bugs in `crates/vb_core/src/budget.rs` (remove `unwrap_or(u16::MAX)` and `saturating_add` forbidden patterns), then re-run `cargo nextest run -p vb_core --test aggregate_resource_budget_red`

## State 5 Repair Summary

Two test bugs were fixed; two remaining failures are implementation bugs:

### FIXED — snapshot red test (`aggregate_resource_budget_snapshot_red`)
**File:** `crates/vb_core/tests/aggregate_resource_budget_snapshot_red.rs`
**Bug:** `.map(|present| present.to_string())` on `bool` array produces `"true"|"false"` strings, not variant names.
**Fix:** zip booleans with variant names, filter `true`, collect names, join.
**Result:** 2/2 PASS.

### FIXED — properties red import (`aggregate_resource_budget_properties_red`)
**File:** `crates/vb_core/tests/aggregate_resource_budget_properties_red.rs`
**Bug:** `use proptest::test_runner::ProptestConfig` — wrong module path.
**Fix:** `use proptest::prelude::{ProptestConfig, *}`.
**Result:** 5/5 PASS.

### NOT FIXED — implementation bugs (do not weaken assertions)
`aggregate_resource_budget_red` 2 failures are production code bugs, not test bugs:

1. **`aggregate_budget_does_not_saturate_branch_count`** — `unwrap_or(u16::MAX)` still in `budget.rs`
2. **`aggregate_budget_does_not_saturate_add_action_tickets`** — `saturating_add(1)` still in `budget.rs`

**Repair instruction:** In `crates/vb_core/src/budget.rs`, replace all `unwrap_or(u16::MAX)`, `unwrap_or(u32::MAX)`, `saturating_add`, and similar legacy saturating/wrap patterns in aggregate budget/capacity/usage code paths with explicit `checked_add`/`checked_sub` that returns `AggregateBudgetError::Overflow { resource: "..." }` or `Underflow { resource: "..." }`.

### Command Evidence

```
cargo nextest run -p vb_core --test aggregate_resource_budget_snapshot_red --no-fail-fast
# 2 tests run: 2 passed, 0 skipped

cargo nextest run -p vb_core --test aggregate_resource_budget_properties_red --no-fail-fast
# 5 tests run: 5 passed, 0 skipped

cargo nextest run -p vb_core --test aggregate_resource_budget_red --no-fail-fast
# 97 tests run: 95 passed, 2 failed, 0 skipped
# FAIL: aggregate_budget_does_not_saturate_add_action_tickets
# FAIL: aggregate_budget_does_not_saturate_branch_count
```
