# Architectural Drift Report: vb_core/lib.rs

**File Analyzed:** `crates/vb_core/src/lib.rs`
**Date:** 2026-05-29
**Status:** PERFECT (lib.rs itself)

---

## 1. Line Count Analysis

| File | Lines | Limit | Status |
|------|-------|-------|--------|
| `lib.rs` | 126 | 300 | ✓ PASS |
| `ids/mod.rs` | 1101 | 300 | ✗ **VIOLATION** |
| `action.rs` | 2287 | 300 | ✗ **VIOLATION** |
| `budget.rs` | 2716 | 300 | ✗ **VIOLATION** |
| `errors.rs` | 2055 | 300 | ✗ **VIOLATION** |
| `frame.rs` | 2081 | 300 | ✗ **VIOLATION** |
| `value_store.rs` | 2552 | 300 | ✗ **VIOLATION** |
| `value.rs` | 1253 | 300 | ✗ **VIOLATION** |
| `limits.rs` | 462 | 300 | ✗ **VIOLATION** |

**lib.rs Conclusion:** 126 lines — **UNDER LIMIT**. No refactoring needed for this file.

---

## 2. DDD Cohesion Analysis

The `lib.rs` itself is a clean re-export facade. However, the **submodules it references** show cohesion issues:

### Cohesive Elements in lib.rs
- Clean module declarations (lines 17-34)
- Organized re-exports by domain (action, budget, capability, engine, etc.)
- Kani verification gated behind `#[cfg(kani)]`

### DDD Smells Detected in Submodules

| Smell | Location | Description |
|-------|----------|-------------|
| **Fat Module** | `budget.rs` (2716 lines) | Budget domain has workflow budget, aggregate budget, and resource capacity all in one file |
| **Fat Module** | `errors.rs` (2055 lines) | CoreError, EngineError, and related types lack separation |
| **Fat Module** | `frame.rs` (2081 lines) | RunFrame and StepState crammed together |
| **Fat Module** | `value_store.rs` (2552 lines) | ObjectField and ValueStore not separated |
| **Primitive Obsession** | `value.rs` | Taint enum (5 variants), FiniteF64 newtype, and SlotValue coexist — possible "Value" god type |
| **Macro-Centric** | `ids/mod.rs` | DRY macro generates 20+ ID types but 1101 lines suggests the file bundles parsing/validation logic for all IDs together |

---

## 3. Violations Summary

### Hard Violations (>300 lines)
1. `ids/mod.rs` — 1101 lines
2. `action.rs` — 2287 lines
3. `budget.rs` — 2716 lines (highest)
4. `errors.rs` — 2055 lines
5. `frame.rs` — 2081 lines
6. `value_store.rs` — 2552 lines
7. `value.rs` — 1253 lines
8. `limits.rs` — 462 lines

### Total Violating Files: 8 submodules
### Total Excess Lines: ~11,500 lines above 300-line threshold

---

## 4. Priority Assessment

| Priority | Reason |
|----------|--------|
| **LOW** | `lib.rs` itself is perfect — 126 lines, clean facade |
| **CRITICAL** | `budget.rs`, `errors.rs`, `frame.rs`, `value_store.rs` are 2000+ lines each — these are architectural gravity wells |
| **HIGH** | `action.rs`, `value.rs`, `ids/mod.rs` — 1000+ lines, likely candidates for domain splitting |
| **MEDIUM** | `limits.rs` — 462 lines, only 162 over limit |

---

## 5. Recommendations

1. **Split `budget.rs`**: Separate `AggregateResourceBudget`, `WholeWorkflowBudget`, and `AggregateReservation` into `budget/aggregate.rs`, `budget/workflow.rs`, `budget/reservation.rs`
2. **Split `errors.rs`**: Separate `CoreError`, `EngineError`, and error construction helpers
3. **Split `value_store.rs`**: Separate `ObjectField` and `ValueStore` into distinct modules
4. **Split `frame.rs`**: `RunFrame` and `StepState` are distinct concepts
5. **Trim `ids/mod.rs`**: Move kani-specific bounds to `ids/kani_*.rs` files, keep only core ID types

---

## 6. Conclusion

**lib.rs**: `STATUS: PERFECT` — No edits required.

**Submodule architecture**: `STATUS: REFACTORED` (required) — The facade is clean, but 8 submodules exceed the 300-line limit and show fat-module DDD smells.

**Next Action**: Decompose the 8 violating submodule files into focused domain modules per Scott Wlaschin DDD principles.
