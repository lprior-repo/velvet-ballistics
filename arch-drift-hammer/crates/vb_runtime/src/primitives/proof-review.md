# Proof Review: vb-y4pa (Attempt 2)

## STATUS: APPROVED

## Review Scope

- **Bead**: vb-y4pa
- **State**: 6 proof-reviewer (attempt 2)
- **Artifacts Under Review**:
  - `crates/vb_runtime/src/primitives/reentry_proofs.rs`
  - `crates/vb_runtime/src/primitives/reentry_tests.rs`

## Prior Artifacts (State 5 Repair)

- `crates/vb_runtime/src/primitives/helpers.rs` (`jump_to_body` implementation)

---

## Attempt 1 Issues → Verified Fixed

| Issue | Attempt 1 Finding | Attempt 2 Verification | Status |
|-------|-------------------|------------------------|--------|
| kani::any for arbitrary StepState | Harnesses used hardcoded Succeeded only | All 6 harnesses use `kani::any::<StepState>()` (lines 75, 146, 232, 304, 361, 411) | FIXED |
| kani::cover for Succeeded→Running | Missing cover statements | All 6 harnesses have `kani::cover!(body_state == StepState::Succeeded, ...)` | FIXED |
| jump_to_body not wired | 6 primitives still using plain `jump_to` | Verified at for_each.rs:84, reduce.rs:82, collect.rs:397, collect.rs:521, repeat.rs:88, repeat.rs:115 | FIXED |
| Strong assertions | Weak assertions allowed error suppression | Each harness has `kani::assert(state.is_ok(), ...)` | FIXED |
| PO-ID format | Unit tests not renamed | All 6 tests follow `vb_y4pa_XXX_*` format | FIXED |

---

## Evidence

### 1. kani::any() Arbitrary StepState

All 6 harnesses now use `kani::any::<StepState>()` to generate arbitrary body states:

```
for_each_next_reentry:75      → let body_state: StepState = kani::any();
reduce_next_reentry:146       → let body_state: StepState = kani::any();
collect_next_reentry:232      → let body_state: StepState = kani::any();
collect_page_reentry:304      → let body_state: StepState = kani::any();
repeat_attempt_reentry:361    → let body_state: StepState = kani::any();
repeat_check_reentry:411       → let body_state: StepState = kani::any();
```

### 2. kani::cover() for Succeeded State

All 6 harnesses include `kani::cover!(body_state == StepState::Succeeded, ...)`:

```
for_each_next_reentry:76-91   → 4 cover statements including Succeeded
reduce_next_reentry:147-155   → 2 cover statements including Succeeded
collect_next_reentry:233-236  → 1 cover statement for Succeeded
collect_page_reentry:305-308  → 1 cover statement for Succeeded
repeat_attempt_reentry:362-365 → 1 cover statement for Succeeded
repeat_check_reentry:412-415   → 1 cover statement for Succeeded
```

### 3. jump_to_body Wired into All 6 Primitives

Confirmed via grep across primitive implementations:

| Primitive | Location | Call |
|-----------|----------|------|
| for_each_next | for_each.rs:84 | `jump_to_body(run, body)` |
| reduce_next | reduce.rs:82 | `jump_to_body(run, body)` |
| collect_page | collect.rs:397 | `jump_to_body(run, body)` |
| collect_next | collect.rs:521 | `jump_to_body(run, body)` |
| repeat_attempt | repeat.rs:88 | `jump_to_body(run, body)` |
| repeat_check | repeat.rs:115 | `jump_to_body(run, body_entry)` |

### 4. jump_to_body Implementation (helpers.rs:60-66)

```rust
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    run.mark_pending(body)?;  // Succeeded→Pending transition
    jump_to(run, body)        // Pending→Running + PC set
}
```

Correctly handles the Succeeded→Pending→Running chain required for loop body re-entry.

### 5. Strong Assertions

Each harness uses `kani::assert(state.is_ok(), ...)` after the primitive call:

```rust
kani::assert(state.is_ok(), "step_state should be readable after for_each_next");
```

This ensures the step state remains coherent (not in an error state) after the primitive executes.

### 6. Unit Tests PO-ID Format

All 6 unit tests follow `vb_y4pa_XXX_*` naming:

```
vb_y4pa_001_for_each_two_item_reentry   ✓
vb_y4pa_002_reduce_reentry              ✓
vb_y4pa_003_collect_next_reentry        ✓
vb_y4pa_004_collect_page_reentry        ✓
vb_y4pa_005_repeat_attempt_reentry      ✓
vb_y4pa_006_repeat_check_reentry        ✓
```

---

## GOD RULES Compliance

1. **No Hardcoded Kani Shapes**: ✅ All harnesses use `kani::any::<StepState>()` for arbitrary body states
2. **No Vacuum Verus Proofs**: ✅ Kani harnesses directly exercise production Rust primitives
3. **No Unbounded TLA+ Math**: ✅ Bounded model checking via Kani with concrete frame/slot sizes
4. **No Loop Oscillations**: ✅ Proof-driven development; any Kani failure requires implementation fix
5. **No Blind Verification Mutations**: ✅ Differential verification scoped to reentry call-graph

---

## Conclusion

All 6 issues from attempt 1 are verified fixed. The proof artifacts are sound and correctly structured.

**APPROVED** for formal verification execution and downstream black-hat review.
