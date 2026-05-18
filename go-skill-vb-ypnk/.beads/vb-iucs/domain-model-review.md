# Domain Model Review: vb-iucs

## Concepts

- `WorkflowParts`: production validation input for Gate 8 accessors.
- `AccessorProgram`: root slot plus path segments requiring bounded root, field symbol, and index sentinel checks.
- `StepState`: finite runtime state enum with idempotent self-transitions, terminal rejection, and suspended resume semantics.
- `vb_proof_kernels::step_state`: shared proof kernel used by runtime predicate.
- `AggregateResourceUsage`: budget arithmetic domain modeled by TLA+ limb arithmetic.

## Illegal States

- Accessor root `>= slot_count` is rejected.
- Field symbol `>= symbols_count` is rejected.
- Index segment `u32::MAX` sentinel is rejected.
- Terminal StepState outward transition is rejected.
- Budget arithmetic overflow/underflow transitions to graceful error status in the TLA+ model.

## Binding Review

`crates/vb_core/src/frame.rs` line 32 delegates `is_valid_step_state_transition` to `vb_proof_kernels::step_state::is_valid_transition`, so the Kani parity harness checks production behavior through the runtime predicate, not a disconnected duplicate.
