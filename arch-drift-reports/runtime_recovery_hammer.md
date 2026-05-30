# Architectural Drift Report: `vb_runtime/src/recovery.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/recovery.rs`
**Total Lines:** 582
**Line Limit:** 300
**Violation Severity:** CRITICAL

---

## 1. LINE COUNT VIOLATION (CRITICAL)

**Exceeds limit by 282 lines (94% over)**.

| Metric | Value |
|--------|-------|
| Actual Lines | 582 |
| Limit | 300 |
| Overflow | 282 lines |
| Overage % | 94% |

---

## 2. RECOVERY RESPONSIBILITY MAP

```
recovery.rs
├── Trait: RuntimeRecoveryBoundary
│   ├── summary() -> RecoveryRuntimeSummary
│   └── hydrate_run_frame() -> RuntimeResult<RunFrame>
│
├── DurableFrameRecoveryBoundary (implements RuntimeRecoveryBoundary)
│   ├── from_seed(RecoveryFrameSeed) -> Self
│   ├── unsupported_state() -> UnsupportedRecoveryState
│   └── hydrate_run_frame() -> RuntimeResult<RunFrame>
│       ├── reject_unsupported_live_frame_state()
│       ├── empty_recovered_frame()
│       ├── apply_recovered_steps()
│       ├── apply_recovered_slots()
│       └── apply_recovered_pc()
│
├── SummaryRecoveryBoundary (implements RuntimeRecoveryBoundary)
│   ├── from_summary(RecoveryRuntimeSummary) -> Self
│   └── hydrate_run_frame() -> RuntimeError
│
├── recovery_boundary_from_hydration() [FACTORY]
│   └── Dispatches to DurableFrame or Summary based on RecoveryHydration variant
│
└── HELPERS (Private)
    ├── reject_unsupported_live_frame_state()
    ├── empty_recovered_frame()
    ├── apply_recovered_steps()
    ├── apply_recovered_slots()
    ├── apply_recovered_pc()
    ├── apply_recovered_step()
    └── mark_suspended()
```

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### VIOLATION 1: Raw `usize` Arithmetic in `apply_recovered_pc` (Line 110)

```rust
fn apply_recovered_pc(frame: &mut RunFrame, seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    if seed.pc.as_usize() >= usize::from(seed.step_count) {  // <-- RAW USIZE
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    // ...
}
```

**Problem:** Uses raw `as_usize()` and `usize::from()` instead of type-safe index comparison.

**Scott Wlaschin DDD Remedy:** Create a `StepCount` newtype with bounded comparison:
```rust
impl StepCount {
    pub fn contains(&self, idx: StepIdx) -> bool { ... }
}
```

### VIOLATION 2: Catch-All Arm Swallowing Unknown States (Line 168)

```rust
fn apply_recovered_step(...) -> RuntimeResult<()> {
    match state {
        RecoveredStepState::Running => frame.mark_running(step),
        RecoveredStepState::Succeeded => frame.mark_succeeded(step),
        RecoveredStepState::Failed => frame.mark_failed(step),
        RecoveredStepState::Waiting => mark_suspended(frame, step, StepState::Waiting),
        RecoveredStepState::Asking => mark_suspended(frame, step, StepState::Asking),
        _ => return Err(RuntimeError::InvalidRecoveryHydration),  // <-- CATCH-ALL
    }
    .map_err(|_| RuntimeError::InvalidRecoveryHydration)
}
```

**Problem:** The catch-all `_` discards the actual variant, losing information. Should use `RecoveredStepState::UNRECOGNIZED` pattern or exhaustive matching with a dedicated error variant.

### VIOLATION 3: Error Information Annihilation (Multiple Locations)

```rust
.map_err(|_| RuntimeError::InvalidRecoveryHydration)  // Lines 92, 105, 115, 170
```

**Problem:** Every `map_err(|_| ...)` annihilates the actual error source. When hydration fails, there is no diagnostic trail. All errors collapse to a single opaque variant.

**Scott Wlaschin DDD Remedy:** Use `err.into()` or a context-preserving error that carries the source.

### VIOLATION 4: Inline Test Construction with Raw Primitives (Lines 247-581)

```rust
let seed = RecoveryFrameSeed {
    summary,
    first_step: StepIdx::ZERO,
    step_count: 4,
    slot_count: 0,
    pc: StepIdx::new(3),
    steps: vec![...],
    // ...
};
```

**Problem:** Tests construct domain objects with raw numeric values (`StepIdx::new(3)`) scattered throughout 394 lines of inline tests. This pollutes the production module and adds 394 lines to the file size.

**Per Workspace Rules:** Tests belong in `crates/workspace_tests/`, not inline.

---

## 4. ARCHITECTURAL DRIFT FINDINGS

### Finding 1: Tests Inline in Production Module

| Issue | Detail |
|-------|--------|
| **Violation** | Inline tests (lines 188-582) = 394 lines |
| **% of File** | 67.7% of total file |
| **Rule** | Workspace structure mandates separate test files |
| **Location** | Should be `crates/workspace_tests/vb_runtime/` |

### Finding 2: Type-Safe Boundary Leaks

| Location | Issue |
|----------|-------|
| Line 40-41 | `DurableFrameRecoveryBoundary { seed: RecoveryFrameSeed }` — internal `RecoveryFrameSeed` leaks through the boundary |
| Line 120-131 | Factory returns `Box<dyn RuntimeRecoveryBoundary>` — dynamic dispatch for what could be a generic or enum |

### Finding 3: Two-Phase Hydration Coupling

The `hydrate_run_frame` method performs three separate apply operations that should be atomic but are decoupled:
- `apply_recovered_steps` (line 66)
- `apply_recovered_slots` (line 67)
- `apply_recovered_pc` (line 68)

If `apply_recovered_slots` fails after `apply_recovered_steps` succeeds, the frame is left in a partial state.

---

## 5. DDD SCOTT WLASCHIN ASSESSMENT

| DDD Principle | Status | Notes |
|---------------|--------|-------|
| Make Illegal States Unrepresentable | ⚠️ PARTIAL | Catch-all `_` arm allows unknown states to produce errors rather than being unrepresentable |
| Value Objects over Primitives | ❌ VIOLATED | Raw `usize` comparison in `apply_recovered_pc` |
| Single Responsibility | ⚠️ PARTIAL | `apply_recovered_step` handles both state mapping AND suspension logic |
| Error Domain Isolation | ❌ VIOLATED | All errors collapse to `RuntimeError::InvalidRecoveryHydration` |
| Exhaustive Matching | ❌ VIOLATED | Catch-all `_` arm in step state matching |

---

## 6. RECOMMENDED REFACTORING

### Mandatory (to achieve <300 lines):

1. **Extract tests to `crates/workspace_tests/vb_runtime/recovery_tests.rs`** — removes 394 lines
2. **Create `StepCount::contains_idx(StepIdx) -> bool`** for type-safe PC validation
3. **Replace catch-all with exhaustive enum variant** for `RecoveredStepState`
4. **Consider enum-based boundary dispatch** instead of `Box<dyn>` factory

### After Refactoring Target:

| Module | Lines |
|--------|-------|
| `recovery.rs` (production) | ~150 |
| `recovery_tests.rs` (extracted) | ~394 |
| **Total** | ~544 (still over due to test duplication of complex setup) |

---

## 7. VERDICT

```
┌─────────────────────────────────────────────────────────────┐
│  ARCHITECTURAL DRIFT: CONFIRMED                            │
│  SEVERITY: CRITICAL                                         │
│  LINE COUNT: 582 / 300 (94% OVER)                           │
│  PRIMITIVE OBSESSION: 4 VIOLATIONS                          │
│  DDD COMPLIANCE: PARTIAL                                     │
└─────────────────────────────────────────────────────────────┘
```

**IMMEDIATE ACTION REQUIRED:** Extract inline tests, fix primitive obsession in `apply_recovered_pc`, replace catch-all arm with exhaustive matching.
