# Architectural Drift Report: `retry.rs`

**File:** `crates/vb_runtime/src/primitives/retry.rs`
**Total Lines:** 1686
**Limit:** 300
**Ratio:** 5.6× OVER LIMIT

---

## 1. RESPONSIBILITY MAP

### Production Code (~411 lines)

| Lines | Responsibility | Primitive Type | Violation |
|-------|----------------|----------------|-----------|
| 14-24 | `DelayStrategy` | Value Object | None — clean |
| 26-119 | `RetryPolicy` | Value Object | None — clean |
| 121-140 | `RetryPolicyError` | Error Type | None — clean |
| 142-260 | `RetryState` | Value Object / State Machine | **VIOLATION: Frame I/O embedded** |
| 262-280 | `RetryDecision` | Decision Type | None — clean |
| 282-298 | `is_failure_retriable` | Policy Function | None — clean |
| 300-338 | `evaluate_retry` | State Machine Transition | None — clean |
| 340-373 | `compute_delay` | Calculation Function | None — clean |
| 375-379 | `exhaustion_error` | Error Factory | None — clean |
| 381-391 | `retry_start` | Orchestration Handler | **VIOLATION: Mixed domain** |
| 393-410 | `retry_on_failure` | Orchestration Handler | **VIOLATION: Mixed domain** |

### Test Code (~1274 lines)

| Lines | Concern |
|-------|---------|
| 412-1686 | Inline `#[cfg(test)]` module — 1274 lines of BDD/unit tests embedded in production module |

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### Violation A: `RetryState` Knows About `RunFrame` Slots

**Location:** Lines 236-259

```rust
pub fn write_to_slot(&self, frame: &mut RunFrame, slot: SlotIdx) -> Result<(), RetryPolicyError>
pub fn read_from_slot(frame: &RunFrame, slot: SlotIdx) -> Result<Self, RetryPolicyError>
```

**Problem:** `RetryState` is a pure value object representing retry counter state. It should NOT know how to serialize/deserialize itself to/from a `RunFrame` slot. This is **infrastructure coupling** — the domain primitive is polluted with I/O concerns.

**Scott Wlaschin Principle:** "Make illegal states unrepresentable" — but here, `RetryState` is made into a "slot serializer" when it should only be a value.

**Expected Refactor:** Extract slot persistence to a separate adapter:

```rust
// retry_state_persister.rs — infrastructure layer
impl RetryState {
    pub fn write_to_slot(...) — REMOVE from RetryState
    pub fn read_from_slot(...) — REMOVE from RetryState
}
// Introduce: RetryStateSlotPersister / RetryStateCodec
```

### Violation B: `retry_start` / `retry_on_failure` Are Orchestration, Not Primitive

**Location:** Lines 381-410

These two functions wire `RetryPolicy` + `RetryState` + `RunFrame` + `SlotIdx` + `ActionFailure` + `RetrySafety` together. This is **workflow orchestration**, not a primitive.

**Problem:** A "primitive" in the DDD sense is an atomic domain concept. `retry_on_failure` is a **use case** — it orchestrates 3 domain objects and 2 infrastructure objects.

**Expected Refactor:**
- `retry_start` → `TryAgainStartNode` or `RetryWorkflow::start`
- `retry_on_failure` → `TryAgainStep::handle_failure` (in the workflow/engine layer)
- The primitive layer should only have: `RetryPolicy`, `RetryState`, `RetryDecision`, `evaluate_retry`, `compute_delay`

### Violation C: Test Infrastructure Embedded in Production Module

**Location:** Lines 412-1686 (1274 lines)

The `#[cfg(test)] mod tests` occupies **75.5% of the file** and is embedded inside the production module. This violates:

1. **Separation of concerns** — tests are a separate "perspective" on the code, not part of the production artifact
2. **<300 line rule** — production + tests combined exceed limit by 5.6×
3. **Build pollution** — every compilation of `vb_runtime` recompiles 1274 lines of tests

**Expected Refactor:**
```
src/primitives/retry.rs          (~260 lines, production only)
src/primitives/tests/retry_tests.rs  (~1274 lines, test module)
```

Or use `#[path = "tests/retry_generated.rs"] mod tests;` to separate.

---

## 3. ACTUAL PRODUCTION CODE SIZE (Without Tests)

| Component | Lines |
|-----------|-------|
| `DelayStrategy` + impl | ~25 |
| `RetryPolicy` + impl | ~95 |
| `RetryPolicyError` | ~20 |
| `RetryState` + impl (excluding slot I/O) | ~100 |
| `RetryDecision` | ~20 |
| `is_failure_retriable` | ~18 |
| `evaluate_retry` | ~40 |
| `compute_delay` | ~35 |
| `exhaustion_error` | ~6 |
| `retry_start` / `retry_on_failure` | ~31 |
| **Total** | **~390** |

**Verdict:** Production code alone (excluding slot I/O pollution and tests) is ~390 lines. Still over 300-line limit but much closer. The slot I/O (~25 lines) and orchestration (~31 lines) are the real bloat — 56 lines of wrong-layer code.

With proper extraction:
- Domain primitives: ~260 lines (DelayStrategy + RetryPolicy + RetryPolicyError + RetryState + RetryDecision + core functions)
- Remaining: ~130 lines of mixed-in concerns

---

## 4. WHAT IS ACTUALLY GOOD

The **core domain model** is well-designed:

- `RetryPolicy` — clean value object with validation, good defaults
- `RetryState` — properly modeled as immutable state with `encode`/`decode` for persistence
- `RetryDecision` — exhaustive decision type (`Retry`, `Exhausted`, `NotRetriable`)
- `evaluate_retry` — pure state machine transition, well-specified
- `compute_delay` — handles `None`/`Fixed`/`ExponentialBackoff` cleanly with overflow saturation
- `is_failure_retriable` — proper combination of `ActionFailure.retry_policy` AND `RetrySafety`

The tests are **excellent BDD coverage** — 1274 lines of thorough testing including:
- Roundtrip encode/decode
- Boundary conditions (u16::MAX, u32::MAX)
- Adversarial BDD scenarios
- Full exhaustion cycles
- Slot corruption handling

**The problem is not code quality — the problem is file organization.**

---

## 5. RECOMMENDED REFACTORING PLAN

### Phase 1: Extract Slot I/O from RetryState (56 lines)

```
crates/vb_runtime/src/primitives/retry/
├── mod.rs                  (~260 lines: exports)
├── retry_policy.rs         (~140 lines: RetryPolicy + RetryPolicyError + DelayStrategy)
├── retry_state.rs          (~130 lines: RetryState — NO slot I/O)
├── retry_decision.rs       (~30 lines: RetryDecision + evaluate_retry + compute_delay)
├── retry_slot_codec.rs     (~40 lines: RetryState slot serialization — EXTRACTED)
├── retry_orchestration.rs  (~35 lines: retry_start + retry_on_failure — EXTRACTED, named explicitly as orchestration)
└── tests/
    └── retry_tests.rs      (~1274 lines: all tests)
```

### Phase 2: Separate Test Module

Move `#[cfg(test)] mod tests { ... }` to `src/primitives/tests/retry_tests.rs` or `src/primitives/retry/tests.rs`.

### Phase 3: Rename Orchestration Functions

`retry_start` → `init_retry_state_for_slot`
`retry_on_failure` → `advance_retry_state_for_slot`

This makes the orchestration intent explicit and separates "retry primitives" from "retry workflow."

---

## 6. SUMMARY SCORECARD

| Rule | Status | Details |
|------|--------|---------|
| <300 Lines | **FAIL** | 1686 lines (5.6× over) |
| Primitive Obsession | **PARTIAL FAIL** | Core primitives clean, but `RetryState` pollutes with Frame I/O |
| Single Responsibility | **FAIL** | One file = policy + state + decision + errors + slot I/O + orchestration + tests |
| DDD Cohesion | **PARTIAL PASS** | Domain concepts well-modeled, but infrastructure leaked into domain |
| Test Isolation | **FAIL** | 1274 lines of tests embedded in production module |

---

## 7. IMMEDIATE ACTION ITEMS

- [ ] **BEAD:** Extract `RetryState::write_to_slot` / `read_from_slot` to `retry_slot_codec.rs`
- [ ] **BEAD:** Extract `retry_start` / `retry_on_failure` to `retry_orchestration.rs` (or workflow layer)
- [ ] **BEAD:** Move test module to `src/primitives/tests/retry_tests.rs`
- [ ] **BEAD:** Verify all downstream `use` imports updated after refactor
- [ ] **BEAD:** Verify `moon ci` passes after refactor

---

*Report generated by: architectural-drift agent*
*Date: 2026-05-29*
*Workspace: arch-drift-hammer*
