# Architectural Drift Report: `vb_core/src/budget.rs`

**File**: `crates/vb_core/src/budget.rs`
**Total Lines**: 2716
**Limit**: 300
**Ratio**: 9.05x over limit

---

## 1. LINE COUNT VERDICT

| Metric | Value |
|--------|-------|
| Total lines | 2716 |
| Maximum allowed | 300 |
| Excess | 2416 (9.05x) |
| **Status** | **CRITICAL VIOLATION** |

---

## 2. DDD COHESION ANALYSIS

**Filename promise**: `budget.rs` → single domain concept (budget)

**Reality**: File contains **6 distinct domain concepts** crammed into one module:

| Concept | Lines | Structures |
|---------|-------|------------|
| Budget computation (IR walking) | 1–166 | `WholeWorkflowBudget`, `BudgetTraversalError`, `compute_budget_local` |
| Small-linear budget optimization | 191–322 | `compute_small_linear_budget`, `SmallLinearMetrics` |
| Boundedness policy | 324–514 | `BoundednessPolicy`, `BudgetError`, `validate_*` helpers |
| Aggregate budget | 554–758 | `AggregateResourceBudget`, `from_workflow`, `from_whole_workflow_budget` |
| Aggregate capacity/usage/reservation | 582–1092 | `AggregateResourceCapacity`, `AggregateResourceUsage`, `AggregateReservation`, `AggregateBudgetError`, `try_add_budget`, `try_subtract_budget`, `fits_within`, `check_policy` |
| Graph traversal (step counting) | 1304–2384 | `count_total_steps`, `visit_node_for_total_steps`, `compute_fanout_and_depth`, `update_workflow_metrics`, `push_*` helpers |

**DDD Smell Detected**: **YES — MULTIPLE**

Symptoms:
- `budget.rs` filename is a blob, not a bounded context
- Internal helpers leak across concern boundaries (e.g., `count_total_steps` and `validate_aggregate_budget` live at same level as data structures)
- `BudgetTraversalError` is budget-computation-only but mixed with `AggregateBudgetError` which is aggregation-only

---

## 3. ALL VIOLATIONS

### 3.1 Line Count Violation
- **Lines 1–2716**: 2716 total — violates 300-line hard limit

### 3.2 Oversized Functions

| Function | Lines | Size |
|----------|-------|------|
| `compute_budget_local` | 75–165 | 91 lines |
| `try_add_budget` | 771–854 | 84 lines |
| `try_subtract_budget` | 856–939 | 84 lines |
| `fits_within` | 941–1030 | 90 lines |
| `validate_aggregate_budget` | 1094–1193 | 100 lines |
| `visit_node_for_total_steps` | 1385–1531 | 147 lines |
| `compute_fanout_and_depth` | 2182–2270 | 89 lines |
| `update_workflow_metrics` | 2330–2384 | 55 lines |

### 3.3 Inline Verification Code
- **Lines 2391–2710**: 320-line `#[cfg(kani)] mod kani_harnesses` embedded in production source
- **Lines 2712–2713**: `#[cfg(test)] mod tests;`
- **Lines 2715–2716**: `#[cfg(test)] mod vb_qi37_2_4_state8_tests;`

These should live in `verification/kani/` or `proofs/` directories, not inline.

### 3.4 Duplicate Validation Logic
Three separate validation paths for the same budget dimensions:

1. `BoundednessPolicy::validate` (lines 384–441)
2. `AggregateResourceUsage::check_policy` (lines 1035–1091)
3. `validate_aggregate_budget` (lines 1094–1193)

This is `Parse, don't validate` violation — the same constraints are validated three times in different places.

### 3.5 Missing Module Separation

Should be a `budget/` directory with:
```
budget/
├── lib.rs          (reexports + module declarations)
├── computation.rs  (WholeWorkflowBudget, traversal, count_*)
├── policy.rs       (BoundednessPolicy, BudgetError, validate_*)
├── aggregate.rs    (AggregateResourceBudget, Capacity, Usage, Reservation, AggregateBudgetError)
└── proofs/         (kani_harnesses → separate from production)
    └── budget_kani_proofs.rs
```

---

## 4. SPECIFIC LINE COUNTS

| Section | Lines | Content |
|---------|-------|---------|
| 1–59 | 59 | Module header, imports, `WholeWorkflowBudget` struct |
| 60–166 | 107 | `WholeWorkflowBudget` impl + `BudgetTraversalError` |
| 167–322 | 156 | Small-linear optimization (`compute_small_linear_budget`, helpers) |
| 323–514 | 192 | `BoundednessPolicy`, `BudgetError`, validation helpers |
| 515–758 | 244 | Aggregate types + impls |
| 759–1092 | 334 | `AggregateResourceUsage` methods + `validate_aggregate_budget` |
| 1093–1293 | 201 | `From` impls, `validate_step_ceilings` |
| 1294–2384 | 1091 | Graph traversal functions (counting, fanout, depth) |
| 2385–2385 | 1 | Comment separator |
| 2386–2710 | 325 | `#[cfg(kani)] mod kani_harnesses` |
| 2711–2716 | 6 | Test module references |

---

## 5. REMEDIATION PRIORITY

**Priority: CRITICAL (P0)**

### Immediate Actions (Required)
1. **Split `budget.rs` into `budget/` directory** — 5 submodules minimum
2. **Extract Kani harnesses** to `verification/kani/vb_core_budget_*` per skill rules
3. **Extract test modules** to `tests/budget_tests.rs` and `tests/budget_vb_qi37_2_4_state8_tests.rs`
4. **Deduplicate validation** — collapse `BoundednessPolicy::validate`, `AggregateResourceUsage::check_policy`, and `validate_aggregate_budget` into single canonical validation path

### Refactoring Order
1. Create `crates/vb_core/src/budget/` directory
2. Move `WholeWorkflowBudget`, `BudgetTraversalError`, traversal functions → `budget/computation.rs`
3. Move `BoundednessPolicy`, `BudgetError`, validation helpers → `budget/policy.rs`
4. Move aggregate types + impls → `budget/aggregate.rs`
5. Create `budget/lib.rs` with reexports
6. Extract Kani proofs to `verification/kani/` per skill convention
7. Extract tests to `tests/`
8. Update `mod.rs` or `lib.rs` to use new module structure

### Estimated Post-Split Sizes
| Module | Est. Lines |
|--------|-----------|
| `budget/computation.rs` | ~600 |
| `budget/policy.rs` | ~250 |
| `budget/aggregate.rs` | ~500 |
| `budget/traversal.rs` | ~500 (extracted from computation) |
| `budget/lib.rs` | ~50 |
| `verification/kani/budget_kani.rs` | ~325 |

---

## 6. GOD RULES COMPLIANCE

| Rule | Status | Notes |
|------|--------|-------|
| No `unsafe` | ✅ PASS | File uses `#![forbid(unsafe_code)]` |
| No `unwrap`/expect | ✅ PASS | All `Result` handling uses `?` or explicit `match` |
| No `panic`/`todo`/`unimplemented` | ✅ PASS | Clean error handling |
| No unchecked indexing | ✅ PASS | All indexing uses `.get()` + explicit error |
| Bounded verification | ✅ PASS | Kani harnesses use `#[kani::proof]` with `kani::any()` + `assume()` |

---

## 7. SUMMARY

```
STATUS: CRITICAL DRIFT
LINES: 2716 / 300 (9.05x VIOLATION)
DDD SMELL: YES
VIOLATIONS: 5 categories
REMEDIATION: P0 — immediate split required
```

**Recommendation**: Do not land any new work touching this file until split is complete. The file is a monolithic grab-bag of budget-related concepts that violates the 300-line hard limit by nearly 10x and breaks DDD cohesion principles.
