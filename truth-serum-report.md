# Truth Serum Report: vb-y4pa for_each/repeat/reduce/collect body re-entry fix

**BEAD**: vb-y4pa
**STATUS**: PASS (with one verification gap flagged)
**DATE**: 2026-05-19

---

## Execution Evidence

### G1: Clippy (Zero Runtime Panic Surface)
```
$ cargo clippy --workspace --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use
cargo clippy: No issues found
exit: 0
```

### G2: Test Suite (reentry_tests)
```
$ cargo test --package vb_runtime --lib primitives::reentry_tests
cargo test: 22 passed, 1453 filtered out (1 suite, 0.00s)
exit: 0
```

### G3: Test Suite (helpers jump_to_body)
```
$ cargo test --package vb_runtime --lib primitives::helpers::tests
cargo test: 28 passed, 1447 filtered out (1 suite, 0.00s)
exit: 0
```

### G4: Test Suite (for_each tests)
```
$ cargo test --package vb_runtime --lib primitives::for_each::tests
cargo test: 45 passed, 1430 filtered out (1 suite, 0.00s)
exit: 0
```

### G5: Full vb_runtime lib tests
```
$ cargo test --package vb_runtime --lib
cargo test: 1475 passed (1 suite, 0.16s)
exit: 0
```

---

## Empathetic User Review

The bug description documents a clear, specific failure mode: loop body steps in `Succeeded` state cannot be re-entered because the `Succeeded→Pending` transition was missing. The fix is a surgical 3-line change in `jump_to_body` (`helpers.rs:60-69`). No user-facing API surface changed. The tests are thorough: 22 dedicated reentry tests covering all 5 loop primitives plus 6 GWT-style BDD scenarios. No confusing terminology or friction introduced.

---

## Skeptical QA Review

### PASS: Production Panic Surface — ZERO
- `forbid(unsafe_code)` present on `for_each.rs`, `reduce.rs`, `collect.rs`, `repeat.rs`, `helpers.rs`
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable` in production primitives
- All state transitions go through `is_valid_step_state_transition` which returns `Err` (not panic) on invalid transition

### PASS: jump_to_body Fix Correctness
`helpers.rs:60-69`:
```rust
pub(crate) fn jump_to_body(...) -> Result<...> {
    let current = run.step_state(body)?;
    if current == StepState::Succeeded {
        run.mark_pending(body)?;
    }
    jump_to(run, body)
}
```
This is the **single point of fix** for all 5 loop primitives:
- `for_each_next` (`for_each.rs:84`) → `jump_to_body`
- `reduce_next` (`reduce.rs:82`) → `jump_to_body`
- `collect_next` (`collect.rs:521`) → `jump_to_body`
- `collect_page` (`collect.rs:397`) → `jump_to_body`
- `repeat_attempt` (`repeat.rs:88`) → `jump_to_body`
- `repeat_check` (`repeat.rs:115`) → `jump_to_body`

### PASS: VALID_TRANSITIONS includes Succeeded→Pending
`vb_proof_kernels/src/step_state.rs:48`:
```rust
(StepState::Succeeded, StepState::Pending),  // loop body re-entry
```
This is the only terminal→non-terminal transition in the state machine. Comment correctly identifies it as "loop body re-entry".

### PASS: No hallucinated file paths
All referenced files exist and contain the claimed code.

### PASS: No deleted tests
`reentry_tests.rs` has 22 tests. No evidence of pre-existing tests removed.

### FLAG: Verification gap in `kani_step_state_transition.rs`

`vb_core/src/kani_step_state_transition.rs:49-68` defines `transition_contract`:
```rust
fn transition_contract(current: StepState, next: StepState) -> bool {
    match (current, next) {
        (state, target) if state == target => true,
        (StepState::Pending, StepState::Running) => true,
        (StepState::Pending, StepState::Succeeded | StepState::Failed | StepState::Cancelled | StepState::Skipped) => true,
        (StepState::Running, StepState::Succeeded | StepState::Failed | StepState::Waiting | StepState::Asking | StepState::Cancelled | StepState::Skipped) => true,
        (StepState::Waiting | StepState::Asking, StepState::Running) => true,
        _ => false,  // ← MISSING: (Succeeded, Pending)
    }
}
```

The proof at line 43:
```rust
kani::assert(is_valid_step_state_transition(current, next) == transition_contract(current, next), ...)
```
**This proof would FAIL** because the runtime `is_valid_step_state_transition` allows `Succeeded→Pending` (for loop re-entry) but `transition_contract` does not model it.

However: this harness is gated behind `#[cfg(kani)]` and `kani` is not installed in this environment, so the proof failure is **UNVERIFIED** (cannot execute `cargo kani`). The mismatch is confirmed by code inspection.

**Severity**: LOW for production (only affects formal verification harness, not runtime). But if Kani proofs are run, this would produce a false failure report.

---

## Mandated Improvements

1. **[VERIFICATION GAP]**: Update `transition_contract` in `vb_core/src/kani_step_state_transition.rs` to include `(StepState::Succeeded, StepState::Pending) => true`. This aligns the formal contract with the runtime state machine and fixes the Kani proof `kani_step_state_transition_matches_contract`.

---

## Summary

| Check | Result |
|-------|--------|
| Zero runtime panic surface | PASS |
| Production `unwrap`/`panic`/`todo` | NONE FOUND |
| `unsafe_code` in primitives | FORBIDDEN + ENFORCED |
| `jump_to_body` fix correctness | CORRECT |
| All 5 loop primitives use `jump_to_body` | VERIFIED |
| `Succeeded→Pending` in VALID_TRANSITIONS | VERIFIED |
| Reentry test coverage | 22 tests PASS |
| Hallucinated paths | NONE |
| Deleted tests | NONE |
| `transition_contract` Kani gap | FLAGGED (UNVERIFIED) |
