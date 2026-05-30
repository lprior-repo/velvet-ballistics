# Architectural Drift Report: `vb_core/src/budget.rs`

**File Analyzed:** `/home/lewis/src/velvet-ballistics/crates/vb_core/src/budget.rs`  
**Analysis Date:** 2026-05-29  
**Status:** ❌ CRITICAL DRIFT DETECTED

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | **2716** | 300 | ❌ **806% OVER LIMIT** |

---

## 2. DDD Cohesion Analysis

**DDD Smell Detected:** ✅ YES — **God Module Anti-Pattern**

The filename `budget.rs` implies a single bounded context, but the file contains **8+ distinct domain concepts** that should be separated:

| Concept | Lines | Should Be |
|---------|-------|-----------|
| `WholeWorkflowBudget` + computation | 1–166 | `workflow_budget.rs` |
| `BudgetTraversalError` | 168–189 | `workflow_budget.rs` |
| `BoundednessPolicy` + validation | 324–514 | `policy.rs` |
| `BudgetError` | 516–552 | `policy.rs` |
| `AggregateResourceBudget` | 554–768 | `aggregate_budget.rs` |
| `AggregateResourceCapacity` | 582–604 | `aggregate_budget.rs` |
| `AggregateResourceUsage` | 606–1092 | `aggregate_usage.rs` |
| `AggregateReservation` | 630–635 | `aggregate_usage.rs` |
| `AggregateBudgetError` | 637–714 | `aggregate_error.rs` |
| Traversal helpers (count_total_steps, etc.) | 1304–2384 | `traversal.rs` |
| Kani harnesses | 2386–2710 | `budget_kani.rs` or `tests/kani_harnesses.rs` |
| Inline test modules | 2712–2716 | `tests/unit_tests.rs` |

**Filename vs. Content Mismatch:** `budget.rs` is a "kitchen sink" that violates Single Responsibility Principle.

---

## 3. Violations List

### 3.1 Critical: File Size (806% Over Limit)

- **Lines 1–2716**: 2716 total lines vs. 300 max
- This is a **God Module** — one file containing multiple bounded contexts

### 3.2 Oversized Functions

| Function | Lines | Parameters | Issue |
|----------|-------|------------|-------|
| `compute_budget_local` | 75–165 | 4 | 90-line function with complex branching |
| `visit_node_for_total_steps` | 1383–1531 | 9 | **9 parameters** — violates "too many arguments" rule |
| `compute_fanout_and_depth` | 2180–2270 | 12 | **12 parameters** — severe smell |
| `update_workflow_metrics` | 2329–2384 | 9 | **9 parameters** — violates parameter limit |
| `AggregateResourceUsage::try_add_budget` | 770–854 | 2 | 84 lines, repetitive field-by-field matching |
| `AggregateResourceUsage::fits_within` | 941–1030 | 2 | 89 lines, repetitive capacity checks |
| `validate_aggregate_budget` | 1094–1193 | 2 | 99 lines, repetitive policy checks |
| `validate_step_ceilings` | 1197–1232 | 1 | 35 lines, magic number constants inline |

### 3.3 Inline Tests

| Location | Lines | Issue |
|----------|-------|-------|
| `#[cfg(test)] mod tests;` | 2712–2713 | Test module declaration in production file |
| `#[cfg(test)] mod vb_qi37_2_4_state8_tests;` | 2715–2716 | Second test module in same file |

**Recommendation:** Move all tests to `tests/budget_tests.rs` in the crate root or `tests/unit/` subdirectory.

### 3.4 Embedded Kani Harnesses

| Location | Lines | Issue |
|----------|-------|-------|
| `#[cfg(kani)] mod kani_harnesses` | 2386–2710 | **324 lines** of verification code in production module |

**Recommendation:** Move to `verification/kani/budget_harnesses.rs` or behind a feature flag.

### 3.5 Missing Module Separation

The following domain concepts are all mixed in one file:
- [ ] Budget computation (`WholeWorkflowBudget`)
- [ ] Budget traversal error types (`BudgetTraversalError`)
- [ ] Policy enforcement (`BoundednessPolicy`, `BudgetError`)
- [ ] Aggregate resource accounting (`AggregateResourceBudget`, `AggregateResourceCapacity`, `AggregateResourceUsage`, `AggregateReservation`)
- [ ] Aggregate errors (`AggregateBudgetError`)
- [ ] Graph traversal algorithms
- [ ] Verification harnesses

---

## 4. Remediation Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0 - CRITICAL** | Split file into `workflow_budget.rs`, `aggregate_budget.rs`, `policy.rs`, `traversal.rs` | High |
| **P0 - CRITICAL** | Extract Kani harnesses to `verification/kani/` behind `kani` feature | Medium |
| **P1 - HIGH** | Extract `#[cfg(test)]` modules to `tests/budget_tests.rs` | Low |
| **P1 - HIGH** | Refactor 9+ parameter functions into trait objects or config structs | High |
| **P2 - MEDIUM** | Extract magic number constants (`HARD_MAX_STEP_BUDGET_PER_TICK = 1_000_000`) to a constants module | Medium |

---

## 5. Summary

```yaml
file: vb_core/src/budget.rs
lines: 2716
limit: 300
over_by_pct: 806%
ddd_smell: true
smell_type: God Module
violations:
  - file_size
  - god_module
  - oversized_functions
  - inline_tests
  - embedded_verification
remediation_priority: P0
```

**Conclusion:** This file is a **structural liability**. It violates every architectural constraint in the skill mandate and must be decomposed before it accrues more technical debt.
