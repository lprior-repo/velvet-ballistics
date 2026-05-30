# Architectural Drift Report: `vb_storage::recovery::replay::summary`

**File:** `crates/vb_storage/src/recovery/replay/summary.rs`
**Analysis Date:** 2026-05-29
**Status:** CRITICAL VIOLATIONS DETECTED

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | **1576** | 300 | **EXCEEDED BY 425%** |

---

## 2. DDD Cohesion Analysis

### Single Responsibility Violation

This file conflates **5 distinct DDD bounded contexts** into one 1576-line module:

| DDD Concept | Lines | Responsibility |
|-------------|-------|----------------|
| **Summary Event Handlers** | 27–232 | Runtime summary construction from journal events |
| **Frame Seed Builder** | 234–411 | RecoveryFrameSeed construction with accumulator pattern |
| **Action Envelope View** | 412–423 | Value object for ticket/input/output/envelope |
| **Slot Recovery** | 827–954 | RecoveredSlots aggregate reconstruction |
| **Error Translation** | 956–985 | ReplayError → RecoveryError mapping |
| **Tests** | 987–1576 | Inline test suite (589 lines) |

### Domain Boundary Smell

```
summary.rs (file)
├── apply_summary_event()           // Domain event application
├── recover_run_admission_from_events()
├── summarize_recovery_events()
├── RecoveryFrameSeedBuilder        // Type 1: Builder pattern
├── FrameSeedAccumulator            // Type 2: Accumulator pattern
├── ActionEnvelopeView              // Type 3: Value object
├── RecoveredSlots                  // Type 4: Entity
├── RecoveredSlotTaint              // Type 5: Value object
├── replay_error_to_recovery()      // Type 6: Error domain mapper
└── [TESTS]                         // Type 7: Tests inline
```

---

## 3. Violations Catalog

### V1: FILE SIZE (Critical)
- **Rule:** Files must be under 300 lines
- **Actual:** 1576 lines
- **Remediation:** Split into 5+ modules

### V2: Inline Tests Polluting Production Code
- **Rule:** Tests belong in `tests/` or `crates/workspace_tests/`
- **Actual:** 589 lines of `#[cfg(test)]` block inline
- **Impact:** Increases file by 37%, violates production/test separation

### V3: Multiple Builder/Accumulator Patterns
- `RecoveryFrameSeedBuilder` (lines 234–272)
- `FrameSeedAccumulator` (lines 394–686) - 292-line struct with 25 methods
- **Issue:** Both patterns exist but serve different aggregation purposes

### V4: Primitive Obsession in `RecoveryIndex`
- **Lines 756–770:**
```rust
trait RecoveryIndex {
    fn index(self) -> u16;
}
```
- Uses raw `u16` instead of a typed `RecoveryIndex` newtype
- Violates "make illegal states unrepresentable"

### V5: Nested Match Arms (High Cyclomatic Complexity)
- `apply_summary_event` has 15 pattern arms
- `apply_frame_event` has 14 pattern arms
- Both could use trait dispatch or hashmap dispatch

### V6: Long Methods
- `FrameSeedAccumulator::apply()` - 24 lines with nested validation
- `FrameSeedAccumulator::apply_frame_event()` - 59 lines (should be trait)

### V7: `format!` in Error Paths
- **Line 134:** `format!("overflow sentinel sequence {} is not valid", ...)`
- **Line 466:** Same pattern repeated
- Violates "no stringly-typed errors in hot paths"

---

## 4. DDD Smell Summary

| Smell | Severity | Description |
|-------|----------|-------------|
| **God Module** | 🔴 Critical | Single file handles 5+ domain concepts |
| **Feature Envy** | 🟡 Medium | FrameSeedAccumulator envies multiple domain objects |
| ** shotgun Surgery** | 🟡 Medium | Change to slot recovery requires editing summary.rs |
| **Parallel Hierarchies** | 🟡 Medium | RecoveryIndex trait mirrors index types |

---

## 5. Recommended Remediation

### Phase 1: File Split (Required)
```
src/recovery/replay/
├── summary.rs              (1576 lines → ~150 lines)
│   └── Retain: Public API + error translation
├── summary_events.rs       (~250 lines)
│   └── apply_summary_event + recover_run_admission_from_events
├── frame_seed.rs           (~400 lines)
│   └── RecoveryFrameSeed + RecoveryFrameSeedBuilder + FrameSeedAccumulator
├── slot_recovery.rs        (~300 lines)
│   └── RecoveredSlots + recovered_slot_taint + slot recovery
├── action_replay.rs         (~200 lines)
│   └── ActionEnvelopeView + action replay logic
└── tests/                  (move all tests)
    └── summary_tests.rs
    ├── frame_seed_tests.rs
    └── slot_recovery_tests.rs
```

### Phase 2: Type Refinements
- Replace `RecoveryIndex` trait with typed `StepIndex`/`SlotIndex` wrappers
- Extract `ActionEnvelopeView` into proper value object in types module

### Phase 3: Simplification
- Use trait objects or hashmap for event dispatch instead of 14-arm matches
- Consider state machine for `FrameSeedAccumulator`

---

## 6. Remediation Priority

| Priority | Task | Effort |
|----------|------|--------|
| **P0** | Move tests to `tests/` directory | 1 hour |
| **P0** | Split file into 4-5 modules by DDD concept | 2-3 hours |
| **P1** | Replace `RecoveryIndex` trait with typed indices | 1 hour |
| **P1** | Extract event dispatch to reduce match complexity | 2 hours |
| **P2** | Consider state machine for accumulator | 3 hours |

---

## 7. Metrics

```
Maintainability Index: 35/100 (Poor)
Cyclomatic Complexity: 89 (Very High)
Lines per DDD Concept: 315 (Should be <100)
Test Coverage Estimate: 80% (high but inline)
```

---

**CONCLUSION:** File MUST be split before further development. Current structure violates both the 300-line hard limit and basic DDD cohesion principles.

**Next Action:** Create split plan and present to lead architect for approval.
