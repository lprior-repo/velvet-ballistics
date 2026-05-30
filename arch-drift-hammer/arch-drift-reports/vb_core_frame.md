# Architectural Drift Report: `vb_core/src/frame.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_core/src/frame.rs`
**Analyzed:** 2026-05-29
**Skill:** `architectural-drift` v1

---

## Summary

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | **2081** | 300 | **VIOLATION** |
| Production Code | ~461 | 300 | VIOLATION |
| Inline Tests | ~840 | 0 (should be separate) | VIOLATION |
| Kani Harnesses | ~763 | 0 (should be separate) | VIOLATION |

---

## 1. File Size Violation (CRITICAL)

**Lines:** 2081
**Limit:** 300
**Excess:** 1781 lines (693% over limit)

This file is **6.9x larger** than the maximum allowed file size.

### Line Breakdown

| Section | Lines | Content |
|---------|-------|---------|
| 1–3 | 3 | Module docs, forbid unsafe |
| 9–29 | 21 | `StepState` enum definition |
| 31–64 | 34 | `is_valid_step_state_transition` function |
| 66–461 | 396 | `RunFrame` struct + impl (production code) |
| 463–473 | 11 | `initialized_slot_entry` helper function |
| **475–1314** | **840** | **`mod tests` — inline integration tests** |
| **1316–2048** | **733** | **`mod frame_kani_harnesses` — Kani proofs** |
| **2050–2081** | **32** | **`mod parallel_in_flight_kani` — Kani proofs** |

---

## 2. DDD Cohesion Analysis

**Filename:** `frame.rs`
**Single Domain Concept:** `RunFrame` — "Bounded run-frame state for one shard-owned workflow run"

### Verdict: **COHESIVE but OVERLOADED**

The core domain concept (`RunFrame`, `StepState`, step-state transitions) is coherent and maps well to the filename. However, the file is **overloaded** with three orthogonal concerns that violate the Single Responsibility Principle:

1. **Domain Model** (lines 9–461): `StepState`, `RunFrame`, state machine logic
2. **Integration Tests** (lines 475–1314): BDD-style behavioral tests
3. **Formal Proof Harnesses** (lines 1316–2081): Kani verification artifacts

---

## 3. All Violations

### V-001: FILE SIZE EXCEEDED (CRITICAL)
- **Location:** Entire file
- **Lines:** 2081
- **Required:** ≤300
- **Remediation:** Extract tests and Kani harnesses to separate files

### V-002: INLINE TESTS BLOCK (MAJOR)
- **Location:** Lines 475–1314 (`#[cfg(test)] mod tests`)
- **Size:** ~840 lines
- **Violation:** Repository structure mandates `crates/workspace_tests/` for integration tests
- **Content:** 30+ test functions covering:
  - Frame initialization/reinitialization
  - Step state transitions
  - Slot read/write bounds
  - Taint tracking
  - Parallel in-flight tracking
  - Security regression tests
- **Remediation:** Move to `crates/workspace_tests/src/vb_core_frame_tests.rs` or similar

### V-003: KANI HARNESS BLOCKS (MAJOR)
- **Location:** Lines 1316–2081 (`#[cfg(kani)] mod frame_kani_harnesses`, `#[cfg(kani)] mod parallel_in_flight_kani`)
- **Size:** ~765 lines
- **Violation:** Formal verification artifacts should live in `verification/` or dedicated proof modules
- **Content:** 11 Kani proofs (K-F1 through K-F5, K-PC1 through K-PC3, K-S1, K-S2, plus parallel in-flight proofs)
- **Remediation:** Move to `verification/kani/vb_core_frame/` or `crates/vb_core/verification/kani/`

### V-004: MISSING MODULE SEPARATION (MODERATE)
- **Location:** `StepState` enum and `is_valid_step_state_transition` function
- **Lines:** 12–64 (52 lines)
- **Issue:** These domain types are embedded in `frame.rs` but could form a cohesive `step_state` submodule
- **Remediation:** Consider extracting to `frame/step_state.rs` if the module grows

### V-005: OVERSIZED IMPL BLOCK (MINOR)
- **Location:** Lines 81–461 (`impl RunFrame`)
- **Size:** 381 lines
- **Issue:** 26 methods on a single impl block exceeds recommended 10–15 method guideline
- **Methods:** `new`, `reinitialize`, `run_id`, `pc`, `executed`, `step_count`, `slot_count`, `max_parallel_in_flight`, `set_max_parallel_in_flight`, `parallel_in_flight`, `add_parallel_in_flight`, `sub_parallel_in_flight`, `set_pc`, `increment_executed`, `read_slot`, `write_slot`, `write_slot_with_taint`, `initialized_slots`, `slots_snapshot`, `taint_snapshot`, `states_snapshot`, `read_taint`, `find_handle_taint`, `write_taint`, `mark_running`, `mark_pending`, `mark_succeeded`, `mark_failed`, `mark_skipped`, `mark_waiting`, `mark_asking`, `mark_cancelled`, `step_state`, `write_step_state`, `validate_transition`
- **Remediation:** Split into logical impl groups: constructors, PC/execution, slots, taint, step states

---

## 4. Specific Line-by-Line Violations

| Line(s) | Violation | Severity |
|---------|-----------|----------|
| 1–2081 | File 693% over size limit | CRITICAL |
| 475–1314 | 840-line inline test module | MAJOR |
| 1319–2048 | 730-line Kani harness module | MAJOR |
| 2050–2081 | 32-line Kani harness module | MAJOR |
| 81–461 | 381-line impl block (>20 methods) | MODERATE |
| 12–64 | `StepState` + transition function not in separate module | MINOR |

---

## 5. DDD Smell Detected

**YES — Feature Envy / Overloaded File**

The `frame.rs` file exhibits **Feature Envy** toward its own domain concept. It bundles:
- Domain types (`StepState`)
- Domain logic (`is_valid_step_state_transition`)
- Behavioral tests (30+ test functions)
- Formal verification proofs (11 Kani harnesses)

This violates: "A module should contain things that change together, for the same reasons, at the same time."

**Test code and verification code change for different reasons than domain logic**:
- Domain logic: changes when business rules evolve
- Tests: change when behavior is added/modified
- Kani proofs: change when verification obligations are updated

---

## 6. Remediation Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0 (Immediate)** | Extract `#[cfg(test)] mod tests` to `crates/workspace_tests/` | Medium |
| **P0 (Immediate)** | Extract `#[cfg(kani)]` modules to `verification/kani/vb_core_frame/` | Medium |
| **P1 (High)** | Split `impl RunFrame` into logical groups | Low |
| **P2 (Medium)** | Extract `StepState` + `is_valid_step_state_transition` to `frame/step_state.rs` | Low |

---

## 7. Target File Structure After Remediation

```
crates/vb_core/src/
├── frame/
│   ├── mod.rs          # Re-exports, thin wrapper
│   ├── step_state.rs   # StepState enum + transition predicate
│   └── run_frame.rs    # RunFrame struct + impl
...
crates/workspace_tests/
├── vb_core_frame_tests.rs   # Migrated integration tests
...
verification/
└── kani/
    └── vb_core_frame/       # Migrated Kani harnesses
        ├── frame_kani_harnesses.rs
        └── parallel_in_flight_kani.rs
```

---

## 8. Enforcement Gate Status

**Current Status:** ❌ FAIL

The file **MUST be refactored** before it can pass the architectural drift gate. The 300-line hard limit is a structural constraint, not a guideline.

**Recommended Enforcement:** Add a `moon` task or CI check that:
1. Fails if any `.rs` file in `crates/*/src/` exceeds 300 lines
2. Excludes `tests/` and `benches/` directories (but not `#[cfg(test)]` modules within source files)
3. Optionally flags `#[cfg(kani)]` modules for separation
