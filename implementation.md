# Implementation Report: vb-y4pa p10-holzman

## BEAD: vb-y4pa
## STATE: 10 holzman-rust (attempt 2)
## DATE: 2026-05-19

## Changed Files

1. `crates/vb_runtime/src/primitives/helpers.rs` (lines 60-68)

## Fix Applied

**Bug**: `jump_to_body` unconditionally called `mark_pending(body)`, which fails for `Waiting` and `Asking` states that are valid re-entry states per contract.

**Fix**: Only reset to `Pending` when current state is `Succeeded`:

```rust
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let current = run.step_state(body)?;
    if current == vb_core::frame::StepState::Succeeded {
        run.mark_pending(body)?;
    }
    jump_to(run, body)
}
```

## Tests Updated

- `tc004_jump_to_body_waiting_is_invalid` → `tc004_jump_to_body_waiting_reentry_valid`
  - Now verifies `Waiting` state is preserved (valid re-entry, no mark_pending call)
- `tc005_jump_to_body_asking_is_invalid` → `tc005_jump_to_body_asking_reentry_valid`
  - Now verifies `Asking` state is preserved (valid re-entry, no mark_pending call)

## Verification

- **cargo build -p vb_runtime**: ✅ Compiled successfully
- **cargo nextest run -p vb_runtime**: ✅ 1651 tests passed

## STATUS: READY