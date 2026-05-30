# Architectural Drift Report: `frame.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_core/src/frame.rs`
**Total Lines**: 2081
**Limit**: 300 lines
**Violation**: 693% of budget — **CATASTROPHIC**

**Date**: 2026-05-29
**Enforcer**: architectural-drift
**Status**: `VIOLATION_FOUND`

---

## Executive Summary

`frame.rs` is a **god file** that violates every structural principle in the architectural contract. It is 2081 lines carrying three completely different concerns: domain logic, behavioral tests, and formal verification harnesses — all smashed into a single file. This is the canonical example of what the 300-line rule exists to prevent.

---

## Structural Breakdown

| Lines | Concern | Classification |
|-------|---------|----------------|
| 1–64 | `StepState` enum + `is_valid_step_state_transition` | PRODUCTION — Domain types |
| 66–461 | `RunFrame` struct + full impl | PRODUCTION — Core frame machinery |
| 463–473 | `initialized_slot_entry` helper | PRODUCTION — Slot utilities |
| 475–1314 | Unit + BDD tests (840 lines) | TESTS — Should be in `tests/frame_tests.rs` |
| 1316–2081 | Kani harnesses (765 lines) | VERIFICATION — Should be in `verification/frame_kani.rs` |

**Actual production code**: ~407 lines
**Test code**: ~840 lines (2x the production code)
**Verification code**: ~765 lines (nearly 2x the production code)

**Total non-production**: 1605 lines (80% of the file is NOT production code)

---

## Violations

### V1: Line Count — CATASTROPHIC

**Required**: ≤300 lines per file
**Actual**: 2081 lines
**Excess**: 1781 lines over budget

This is not a marginal violation. This is 693% of the allowed budget.

---

### V2: Primitive Obsession — Domain Types Encoded as Raw Primitives

`RunFrame` encodes domain concepts as untyped Rust primitives:

```rust
pub struct RunFrame {
    run_id: RunId,           // ✅ Newtype — correct
    pc: StepIdx,             // ✅ Newtype — correct
    executed: u64,           // ❌ Raw u64 — should be ExecutedCount(u64)
    step_count: u16,         // ❌ Raw u16 — should be StepCount(u16)
    slot_count: u16,         // ❌ Raw u16 — should be SlotCount(u16)
    max_parallel_in_flight: u16,  // ❌ Raw u16 — should be ParallelBudget(u16)
    parallel_in_flight: u16,       // ❌ Raw u16 — should be ParallelInFlight(u16)
    states: Box<[StepState]>,
    slots: Box<[Option<SlotValue>]>,
    taint: Box<[Taint]>,
}
```

**Harm**: You can pass `frame.step_count()` (u16) to `frame.slot_count()` (also u16) with no type-level resistance. These are **conceptually distinct** quantities but **structurally identical**. Any future arithmetic between them is a semantic footgun.

**Required refactor**: Newtypes under `vb_core/src/types/`:
- `ExecutedCount(u64)` — wraps step transition counter
- `StepCount(u16)` — number of steps in workflow
- `SlotCount(u16)` — number of slots in workflow
- `ParallelBudget(u16)` — max parallel in-flight branches
- `ParallelInFlight(u16)` — current parallel in-flight count

---

### V3: God File — Three Concerns Conflated

The file contains **three structurally different concerns** that must be separated:

1. **Production domain logic** (lines 1–473): The actual `RunFrame` state machine
2. **Behavioral tests** (lines 475–1314): BDD/unit tests that verify behavior
3. **Formal verification** (lines 1316–2081): Kani harnesses for bounded model checking

These should live in **separate files** with **separate compilation gates**:
- `src/frame/step_state.rs` — `StepState` enum + transition predicate
- `src/frame/frame.rs` — `RunFrame` struct + impl
- `src/frame/slot_registry.rs` — (extracted slot management concern)
- `tests/frame_tests.rs` — all behavioral tests
- `verification/frame_kani.rs` — all Kani harnesses (behind `#[cfg(kani)]`)

---

### V4: Suspiciously High Test-to-Code Ratio

Production code: ~407 lines
Test code: ~840 lines
**Ratio: 2.06:1**

For a bounded state machine like `RunFrame`, this ratio is a **code smell signal** — either:
- Tests are duplicated/overlapping (many test the same boundary condition)
- The state machine is under-designed and needs architectural attention
- Tests include commented-out or redundant coverage

The file needs a **test audit**: deduplicate coverage, eliminate redundant edge-case tests, and reduce to a 1:1 test-to-logic ratio.

---

### V5: `find_handle_taint` — O(n) Linear Scan

```rust
fn find_handle_taint(&self, value: &SlotValue) -> CoreResult<Taint> {
    match value {
        SlotValue::Object(id) => {
            let mut idx = 0usize;
            while idx < usize::from(self.slot_count) {  // O(n) scan
                if let Some(Some(SlotValue::Object(vid))) = self.slots.get(idx)
                    && *vid == *id
                {
                    return self.taint.get(idx).copied().ok_or(...);
                }
                idx = idx.saturating_add(1);
            }
            Ok(Taint::Clean)
        }
        // ... same pattern for List ...
    }
}
```

This is an **O(n) linear scan** over all slots for every handle taint lookup. For `SlotValue::Object` and `SlotValue::List`, this is called via `find_handle_taint`. This is an architectural hotspot — if slot_count grows, this becomes a performance cliff.

**Scott Wlaschin principle violated**: "Make illegal states unrepresentable" — if you need to find taint by handle ID often, you should maintain an auxiliary index (e.g., `HashMap<ObjectId, SlotIdx>`) rather than scanning.

---

### V6: `mark_running` etc. — One Method Per State Variant

```rust
pub fn mark_running(&mut self, step: StepIdx) -> CoreResult<()> { ... }
pub fn mark_pending(&mut self, step: StepIdx) -> CoreResult<()> { ... }
pub fn mark_succeeded(&mut self, step: StepIdx) -> CoreResult<()> { ... }
pub fn mark_failed(&mut self, step: StepIdx) -> CoreResult<()> { ... }
pub fn mark_skipped(&mut self, step: StepIdx) -> CoreResult<()> { ... }
pub fn mark_waiting(&mut self, step: StepIdx) -> CoreResult<()> { ... }
pub fn mark_asking(&mut self, step: StepIdx) -> CoreResult<()> { ... }
pub fn mark_cancelled(&mut self, step: StepIdx) -> CoreResult<()> { ... }
```

Eight near-identical methods that **diverge only by the `StepState` variant**. This is **primitive obsession at the method level** — the method names encode the state, but the signature is identical. This pattern makes it impossible to accidentally pass a `StepState` to the wrong call site, but it also means every new `StepState` variant requires a new method.

**Scott Wlaschin principle**: When you see a set of methods that differ only by an enum argument, replace them with a single method that takes the enum. Use `write_step_state(step, StepState::Running)` directly.

---

### V7: Kani Harnesses at Module Level Pollute Compilation

Lines 1316–2081 contain **7 Kani proofs** and **2 harness modules** (`frame_kani_harnesses` + `parallel_in_flight_kani`). These are behind `#[cfg(kani)]` gates, but they:

1. **Live at module level** inside `frame.rs` — not in a separate verification crate
2. **Duplicate logic** (`validate_transition_inline` re-implements the transition predicate)
3. **Use manual 64-pair expansion** instead of a proper exhaustiveness proof

The `frame_kani_harnesses::validate_transition_exhaustive_64` function manually writes out all 64 pairs with `kani::assert` calls — this is **not how Kani should be used**. It should use `kani::any()` with symbolic exploration, not hand-unrolled pairs.

---

## Required Refactoring Plan

### Phase 1: Split File (Mandatory, Non-Negotiable)

```
crates/vb_core/src/frame/
├── mod.rs              # Re-exports
├── step_state.rs       # StepState enum + is_valid_step_state_transition (~65 lines)
├── frame.rs           # RunFrame struct + impl (~400 lines)
└── slot_registry.rs   # Slot registry concern (extract from find_handle_taint) (~50 lines)

crates/vb_core/tests/
└── frame_tests.rs     # All behavioral tests (~840 lines, deduplicated target: ~300)

crates/vb_core/verification/
└── frame_kani.rs      # Kani harnesses (~765 lines, target: ~300)
```

**Target after split**: Each file ≤300 lines.

### Phase 2: Primitive Obsession Cleanup

Create domain newtypes in `crates/vb_core/src/types/`:
- `ExecutedCount(u64)` — step transition counter
- `StepCount(u16)` — workflow step count
- `SlotCount(u16)` — workflow slot count
- `ParallelBudget(u16)` — max parallel in-flight
- `ParallelInFlight(u16)` — current parallel in-flight

Replace raw primitives in `RunFrame` with these newtypes.

### Phase 3: Test Deduplication

Audit the 840 lines of tests. Expected deduplication target: **~300 lines**.
- Many tests cover the same boundary conditions with slight variations
- `ut_terminal_state_blocks_transitions` (line 1228) is 86 lines — could be table-driven

### Phase 4: Kani Harness Rewrite

Replace the hand-unrolled 64-pair proof with a **proper symbolic harness**:
```rust
#[kani::proof]
fn validate_transition_exhaustive_all_pairs() {
    let current = kani::any::<StepState>();
    let target = kani::any::<StepState>();
    let _ = validate_transition_inline(current, target); // No panic for any pair
}
```

This is the correct Kani usage — symbolic exploration over all 8×8 = 64 pairs automatically.

### Phase 5: `find_handle_taint` Index

Replace O(n) scan with a `HashMap<ObjectId, SlotIdx>` auxiliary index maintained on `write_slot`/`write_slot_with_taint`. This is a meaningful architectural improvement for workflows with many object slots.

---

## Severity Assessment

| Violation | Severity | Effort to Fix |
|-----------|----------|---------------|
| Line count (2081 vs 300) | **CRITICAL** | Medium — mechanical split |
| Primitive obsession | **HIGH** | Low — add newtypes |
| God file (3 concerns) | **CRITICAL** | High — separate modules + tests + verification |
| Test-to-code ratio (2:1) | **MEDIUM** | Medium — deduplicate |
| `find_handle_taint` O(n) | **HIGH** | Medium — add HashMap index |
| mark_* method proliferation | **LOW** | Low — consolidate to `write_step_state` |
| Kani harness quality | **MEDIUM** | Low — rewrite as symbolic |

---

## Recommendations

1. **Immediately split** `frame.rs` into `frame/step_state.rs`, `frame/frame.rs`, and move tests to `tests/frame_tests.rs` and verification to `verification/frame_kani.rs`
2. **Create domain newtypes** for all raw numeric fields in `RunFrame`
3. **Audit and deduplicate tests** targeting 1:1 test-to-logic ratio
4. **Rewrite Kani harnesses** to use symbolic exploration instead of hand-unrolled pairs
5. **Index `find_handle_taint`** with a `HashMap` auxiliary structure

---

## Conclusion

`frame.rs` is a **structural disaster** by any reasonable metric. It is 7x over the line count budget, mixes three fundamentally different concerns (production code, tests, formal verification), encodes domain concepts as raw primitives, and contains suspicious test bloat. The code itself is **well-written and carefully reasoned** — the `RunFrame` implementation is solid. But the **file organization** is an anti-pattern that will compound as the codebase grows.

**This file must be split before any further feature work proceeds.**

---

*Report generated by architectural-drift agent*
*Next action: Await bead creation for refactoring work*
