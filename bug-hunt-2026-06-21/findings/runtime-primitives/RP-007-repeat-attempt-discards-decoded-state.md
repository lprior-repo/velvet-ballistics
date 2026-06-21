# RP-007: `repeat_attempt` validates state but discards the decoded values

- **Severity**: Low
- **Category**: simplification
- **Location**: `crates/vb_runtime/src/primitives/repeat.rs:78-89`
- **Confidence**: confirmed

## Description

`repeat_attempt` reads the packed attempt state from the slot, decodes it purely for validation, and discards both the max and current attempt values before jumping to the body. The decode is correct defensive validation, but the discarding makes the intent unclear; the call also does nothing the body couldn't do itself via the slot read.

## Evidence

`crates/vb_runtime/src/primitives/repeat.rs:78-89`:

```rust
pub fn repeat_attempt(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    body: StepIdx,
    _done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let packed = expect_i64(*run.read_slot(attempt_slot)?)?;
    // Validate that the slot contains a valid repeat state.
    let (_max, _current) = decode_repeat_state(packed)?;
    // Slot already holds the correct packed state; just jump to body.
    jump_to_body(run, body)
}
```

The function discards `_max` and `_current`. The `_done` parameter is also unused. If validation is the only intent, the body of the function is essentially `decode_repeat_state(expect_i64(*run.read_slot(attempt_slot)?)?)?; jump_to_body(run, body)`, which could be expressed more clearly as:

```rust
repeat_state_from_slot(run, attempt_slot)?; // pure validation
jump_to_body(run, body)
```

or folded into `repeat_check` (which already decodes the same state).

## Adversarial Check

The validation is not strictly dead — it *does* reject malformed slots with `invalid_repeat_state()`. So this is a clarity/maintainability issue, not a bug. Severity Low / Info.

The presence of `_done: StepIdx` (unused) is also slightly concerning: it suggests the compiler-emitted node has a `done` target that this handler silently ignores, which may indicate the handler is a stub for an unimplemented code path. Worth confirming against the workflow compiler's expectations.

## Suggested Fix

Either:

- Remove the unused `_done` parameter and rename the function to `repeat_validate_then_body` (or merge into `repeat_check`), or
- Document explicitly that `RepeatAttempt` is a no-op validation node inserted by the compiler for invariant checking on re-entry, so future maintainers do not assume the values are needed.
