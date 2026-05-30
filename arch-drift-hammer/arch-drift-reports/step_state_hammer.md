# Architectural Drift Report: `step_state.rs`

**File**: `crates/vb_proof_kernels/src/step_state.rs`
**Total Lines**: 512 (exceeds 300-line limit by 212 lines)
**Status**: `REFACTOR REQUIRED`

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 512 | 300 | ❌ OVER BUDGET (+212) |
| Production code | ~135 | 300 | ✅ OK |
| Test code | ~375 | 300 | ❌ OVER BUDGET (+75) |

**Verdict**: File MUST be split. Production code is clean; the test module at 375 lines is the primary offender.

---

## 2. Primitive Obsession Violations

### 2.1 `&'static str` Error Type (Line 70)
```rust
pub fn validate_transition(from: StepState, to: StepState) -> Result<StepState, &'static str> {
    Err("invalid_state_transition")
}
```
**Violation**: `&'static str` is a primitive. Should be a typed error enum.
**DDD Fix**: Create `StepStateError` enum with specific variants:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStateError {
    InvalidTransition { from: StepState, to: StepState },
}
```

### 2.2 Raw Tuple Array for Transition Table (Line 28)
```rust
const VALID_TRANSITIONS: &[(StepState, StepState)] = &[
    (StepState::Pending, StepState::Running),
    // ...
]
```
**Violation**: Tuples are primitives. The transition relationship should be a typed `Transition` struct.
**DDD Fix**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    pub from: StepState,
    pub to: StepState,
}

const VALID_TRANSITIONS: &[Transition] = &[
    Transition { from: StepState::Pending, to: StepState::Running },
    // ...
];
```

### 2.3 Untyped `Vec<StepState>` Return (Lines 74-81)
```rust
pub fn next_states(from: StepState) -> Vec<StepState>
```
**Violation**: Returning a `Vec` implies ownership semantics that may not be needed. Could be `&'static [StepState]` for lookups.
**DDD Fix**: Return a slice reference or use an iterator.

---

## 3. DDD Structure Analysis

### What's Good
- `StepState` enum is a proper **Value Object** with exhaustive variants
- `is_terminal()` is a clean **Query** on the state
- State machine is well-documented with comments
- Terminal vs non-terminal separation is explicit

### What Needs Fixing
1. **Missing `Transition` type** - transition is a first-class domain concept, not a tuple
2. **Missing `StepStateError` type** - invalid transition is a domain failure mode
3. **Excessive test verbosity** - tests at 375 lines duplicate transition table encoding

---

## 4. Recommended File Split

### Option A: Three-File Split (Recommended)

| File | Contents | Est. Lines |
|------|----------|------------|
| `step_state.rs` | Enum + `is_terminal()` + `terminal_states()` + `non_terminal_states()` | ~100 |
| `step_state_transitions.rs` | `Transition` type + `VALID_TRANSITIONS` + `is_valid_transition()` + `validate_transition()` + `next_states()` | ~90 |
| `step_state_tests.rs` | All tests (split further if >300) | ~375 |

### Option B: Four-File Split (For Strict Compliance)

| File | Contents | Est. Lines |
|------|----------|------------|
| `step_state.rs` | Enum + basic queries | ~55 |
| `step_state_transitions.rs` | Transition table + validators | ~80 |
| `step_state_invariants.rs` | `terminal_cannot_transition_to_non_terminal()`, `all_transitions_exhaustive()` | ~40 |
| `step_state_tests.rs` | Tests | ~375 (still >300!) |

### Option C: Five-File split (Test Module)
Further split tests into:
- `step_state_tests_validity.rs` (~100 lines)
- `step_state_tests_next_states.rs` (~80 lines)
- `step_state_tests_invariants.rs` (~50 lines)
- `step_state_tests_derived_traits.rs` (~80 lines)

---

## 5. Action Items

| Priority | Action | Complexity |
|----------|--------|------------|
| P0 | Split test module into `step_state_tests/*.rs` | Medium |
| P1 | Create `Transition` newtype | Low |
| P1 | Create `StepStateError` enum | Low |
| P2 | Refactor `validate_transition` to use `StepStateError` | Low |
| P2 | Change `next_states` return to `&'static [StepState]` | Low |
| P2 | Update `mod.rs` exports | Low |

---

## 6. Risk Assessment

| Risk | Level | Notes |
|------|-------|-------|
| Breaking API surface | Medium | Transition to typed errors is a breaking change |
| Test duplication | Low | Splitting tests doesn't change behavior |
| Verification impact | Low | Kani/Verus harnesses likely use `is_valid_transition` directly |

---

## 7. Formal Verification Artifacts

If this file has Kani harnesses, verify:
- `kani::Arbitrary` for `StepState` (not hardcoded)
- Transition property proofs still pass after `Transition` type introduction
- `StepStateError` variant coverage in error-path harnesses

---

**Generated**: 2026-05-29
**Enforcer**: architectural-drift
**Next Session**: Implement Option A or C split
