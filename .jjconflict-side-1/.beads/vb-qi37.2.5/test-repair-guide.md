# test-repair-guide.md — vb-qi37.2.5

## REJECTED — Coverage Gaps

vb-qi37.2.5 test suite rejected at Tier 2. Coverage must reach ≥90% overall and ≥95% Calc layer before resubmission.

---

## Priority 1: budget.rs (67.48% → target ≥95%)

**332 lines missed.** Focus areas:

### A. `count_and_push_loop_body` checked_mul branches
The function has multiple `checked_mul` / `checked_add` paths that return `WorkflowError::StepCountOverflow`. These are the core of B11. The existing `test_step_count_overflow` (line 420) only triggers `StepOutOfBounds`, not genuine overflow.

**Required tests:**
```rust
// Add to crates/vb_core/src/budget/tests.rs

#[test]
fn count_total_steps_overflows_u64_returns_error() {
    // Build a workflow that causes checked_mul to overflow
    // e.g., nested ForEachStart loops with limit=u32::MAX
    // Expected: WorkflowError::StepCountOverflow
}

#[test]
fn count_and_push_loop_body_overflow_propagates() {
    // Directly test overflow in checked_mul path
    // Expected: WorkflowError::StepCountOverflow with exact error
}
```

### B. Blackhat tests verification
Verify that BH-BUD-01 through BH-BUD-13 genuinely exercise their named behaviors:
- BH-BUD-01: `max_steps_executable` saturation — verify it actually saturates
- BH-BUD-04: `ForEachStart limit=0` counts as 1 iteration — verify this is intentional vs bug
- BH-BUD-06/07: `saturating_add` overflow paths — these are more observational than functional

### C. Additional WholeWorkflowBudget compute paths
- `WholeWorkflowBudget` fields not covered: `max_parallel_in_flight`, `max_action_tickets`, `max_gather_pages`, `max_gather_items`
- `compute` error paths: multiple `WorkflowError` variants not exercised

---

## Priority 2: signals.rs (83.26% → target ≥90%)

**39 lines missed.** Focus areas:

### A. `from_env` error paths
```rust
// Add to crates/vb_core/src/engine/signals.rs tests

#[test]
fn from_env_rejects_non_numeric_value() {
    // Set VB_BENCH_LATENCY_BUDGET_US to "abc"
    // Expected: Err(EngineError::BudgetParse { reason: "invalid u64 value" })
}

#[test]
fn from_env_accepts_valid_u64() {
    // Set VB_BENCH_LATENCY_BUDGET_US to "5000"
    // Expected: Ok(StepBudget { remaining: 5000 })
}

#[test]
fn from_env_clamps_above_max() {
    // Set VB_BENCH_LATENCY_BUDGET_US to "100000"
    // Expected: Ok(StepBudget { remaining: MAX_STEP_BUDGET })
}
```

### B. Debug formatting paths
- `EngineSignal` debug format branches for each variant
- `StepBudget` debug format

### C. `EngineError::BudgetParse` variant coverage
The error variant exists in the enum but no test exercises it.

---

## Priority 3: value_store.rs (84.64% → target ≥95%)

**232 lines missed.** Focus areas:

### A. Taint propagation paths
- `list_taints` and `object_taint_index` access paths
- Taint variants: `Taint::Clean`, `Taint::Secret`, `Taint::DerivedFromSecret`

### B. Accessor paths
- `symbol`, `list`, `object`, `blob` — verify all error variants are exercised
- `list_item`, `object_field` — boundary conditions beyond arena cap

### C. Blob operations
- `insert_blob` success path with non-empty data
- `blob` accessor with various blob sizes

### D. `total_arena_count` and `max_arena_entries` accessor coverage
These methods are called in assertions but their own logic paths may not be fully tested.

---

## Priority 4: Overall Coverage (88.90% → ≥90%)

The missing 2.1 percentage points (~360 lines) are distributed across all modules.

**Strategy**: Focus on unexercised code paths in:
- `workflow/mod.rs` (80.25%)
- `engine/choose.rs` (93.11% but 18 functions missed)
- `replay/choose.rs` (58.54% line coverage — major gap but outside boundedness scope)

---

## Verification Commands

After each fix, run:
```bash
# Full test suite
cargo test --package vb_core --all-features -- --test-threads=1

# Coverage check
cargo llvm-cov --package vb_core report --summary-only

# Specific module coverage
cargo llvm-cov --package vb_core report --value-store crates/vb_core/src/value_store.rs

# Proptest invariants
cargo test --package vb_core --lib engine::signals::tests::property_ -- --nocapture
```

---

## Don't Break Existing Tests

All 1401 existing tests must continue to pass. The additional tests must:
1. Not break deterministic ordering (run with --test-threads=1 and --test-threads=8)
2. Not add `static mut`, `lazy_static!`, or shared mutable state
3. Use `ensure_equal` pattern or `assert_eq!` with exact values — no `assert!(result.is_ok())` as sole assertion
4. Be in `#[cfg(test)]` modules within the relevant source files, or in vb_core integration tests without `use crate::`

---

## Fuzz Target Note

The `step_budget_new` fuzz target cannot be compiled due to pre-existing vb_runtime `chunk_001.rs` build failure (DEFERRED_GLOBAL per contract.md). This is **not** a test suite issue — it's a workspace infrastructure issue outside vb-qi37.2.5 scope. The fuzz corpus and target definition are correct in the test plan.
