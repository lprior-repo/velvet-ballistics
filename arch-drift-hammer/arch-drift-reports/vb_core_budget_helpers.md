# Architectural Drift Report: `vb_core/src/budget.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_core/src/budget.rs`  
**Analysis Date**: 2026-05-29  
**Agent**: architectural-drift  

---

## Executive Summary

| Metric | Value | Status |
|--------|-------|--------|
| **Total Lines** | 2,716 | 🔴 CRITICAL VIOLATION (limit: 300) |
| **DDD Cohesion** | LOW | 🔴 Multiple domain concepts co-located |
| **Priority** | P0 | Immediate refactoring required |

**STATUS: REFACTORED** — File requires mandatory split.

---

## 1. Line Count Analysis

| Metric | Value | Limit | Violation |
|--------|-------|-------|-----------|
| Total lines | 2,716 | 300 | **+2,416 over** |
| Violation ratio | 9.05x | — | CRITICAL |

**Verdict**: File is **9x over the 300-line mandate**. This is a structural emergency.

---

## 2. DDD Cohesion Analysis

### 2.1 Domain Concepts Identified

This single file contains **6+ distinct domain concepts** that should be separated:

| Concept | Lines | Responsibility | Smell |
|---------|-------|----------------|-------|
| `WholeWorkflowBudget` | ~60 | Value object for computed workflow budget | ✓ Cohesive |
| `BoundednessPolicy` | ~120 | Policy validation rules | ✓ Cohesive |
| `BudgetError` / `BudgetTraversalError` | ~90 | Error types | ✓ Cohesive |
| `AggregateResourceBudget/Capacity/Usage` | ~250 | Aggregate accounting | ⚠ Borderline |
| `AggregateBudgetError` | ~80 | Aggregate error handling | ⚠ Borderline |
| `AggregateReservation` | ~5 | Reservation identity | ✓ Cohesive |
| **Traversal logic** (DFS, counting) | ~800 | Algorithm implementation | 🔴 Embedded infra |
| **Kani harnesses** | ~330 | Verification artifacts | 🔴 Test pollution |
| **Tests** | ~500+ | Test modules | 🔴 Test pollution |

### 2.2 Scott Wlaschin DDD Violations

1. **Primitive Obsession**: Uses raw `u64`, `u32`, `u16` throughout without newtypes for budget dimensions.

2. **Mixed Abstraction Levels**: File mixes:
   - Domain types (`WholeWorkflowBudget`, `BoundednessPolicy`)
   - Infrastructure (DFS traversal algorithms)
   - Verification (Kani harnesses)
   - Tests

3. **God File**: 2,716 lines is a classic "god module" anti-pattern.

### 2.3 Cohesion Score: LOW

The file violates:
- Single Responsibility Principle (5+ reasons to change)
- DDD module cohesion (heterogeneous concepts)
- File size mandate (9x over limit)

---

## 3. Architectural Violations

### 3.1 Hard Violations

| Rule | Location | Violation |
|------|----------|-----------|
| **<300 lines/file** | Entire file | 2,716 lines (9.05x over) |
| **Test separation** | Lines 2386-2710 | Embedded Kani harnesses in production module |
| **Test separation** | Lines 2712-2716 | Inline `#[cfg(test)]` modules |

### 3.2 Structural Issues

1. **Verification Pollution**: Kani harnesses (`#[cfg(kani)]`) embedded directly in the production module. These should be in `src/kani/` or `verification/` subdirectory.

2. **Test Pollution**: `#[cfg(test)] mod tests;` and `#[cfg(test)] mod vb_qi37_2_4_state8_tests;` at bottom of production code pollutes the module boundary.

3. **Algorithm/Logic Coupling**: DFS traversal logic (~800 lines) is embedded in the same file as domain types.

---

## 4. Required Refactoring

### 4.1 Proposed Module Split

```
src/budget/
├── mod.rs           (reexports only)
├── types.rs         (~150 lines) — WholeWorkflowBudget, BoundednessPolicy, BudgetError
├── aggregate.rs     (~250 lines) — AggregateResource*, AggregateReservation, AggregateBudgetError
├── traversal.rs     (~500 lines) — count_total_steps, compute_fanout_and_depth, DFS helpers
├── validation.rs    (~200 lines) — validate_* functions, check_* functions
├── policy.rs        (~100 lines) — BoundednessPolicy::validate, validate_step_ceilings
└── tests.rs         (moved from budget/tests.rs)
```

### 4.2 Additional Actions

1. Move all `#[cfg(kani)]` modules to `src/kani/` directory
2. Extract Kani harnesses into separate verification files with proper `#[kani::proof]` modules
3. Create newtype wrappers for budget dimensions (e.g., `MaxSteps(u64)`, `MaxFanout(u16)`)
4. Move `vb_qi37_2_4_state8_tests` to `tests/resource_contract_validation.rs` or similar

---

## 5. Priority Assessment

| Priority | Rationale |
|----------|-----------|
| **P0 — MANDATORY** | File is 9x over size limit; blocks CI/linting gates |
| **P0 — MANDATORY** | Verification artifacts pollute production module boundary |
| **P1 — HIGH** | DDD cohesion violations cause maintenance burden |
| **P2 — MEDIUM** | Primitive obsession reduces type safety |

---

## 6. Evidence

```
File: crates/vb_core/src/budget.rs
Lines: 2,716
Status: MUST SPLIT
```

---

## 7. Summary

**vb_core/src/budget.rs** is in **CRITICAL architectural drift**. The file:
- Exceeds the 300-line mandate by 2,416 lines (9x over)
- Contains 6+ distinct domain concepts violating DDD cohesion
- Embeds Kani verification harnesses in production code
- Co-locates tests with production logic

**Immediate action required**: Split into `budget/` submodule per the proposed architecture above.
