# RP-013: jump_to_body Sets PC To Terminal Body Steps Without Re-Admitting Them

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/primitives/helpers/jump.rs:31`
- **Confidence**: confirmed

## Description
`jump_to_body` only re-admits a body step when its state is `Succeeded`, but still sets the program counter to the body for all other states. If the body is `Failed`, `Cancelled`, or `Skipped`, the helper routes execution to a terminal step without a valid lifecycle transition.

## Evidence
The helper documents terminal states as absorbing, but only handles `Succeeded` specially:

```rust
31:     let current = run.step_state(body)?;
32:     if current == vb_core::frame::StepState::Succeeded {
33:         run.mark_pending(body)?;
34:         run.mark_running(body)?;
35:     }
36:     jump_to(run, body)
```

Every primitive using this helper inherits that behavior on loop or retry re-entry.

## Adversarial Check
This is not just a missing optimization. If the engine executes based on the program counter, it can re-run a terminal step without a state transition. If the engine refuses to execute terminal steps, the run can be left pointing at an unexecutable step. Both outcomes violate the helper's own lifecycle comment that terminal states are absorbing.

## Suggested Fix
Match all step states explicitly. Re-admit only states that are valid for the primitive's semantics, and return an `EngineError` for `Failed`, `Cancelled`, or `Skipped` unless a specific retry primitive has performed an explicit, audited reset transition.
