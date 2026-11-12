# State 7 Test Plan - vb-0253.5

STATUS: APPROVED

## Required Behaviors

- Pending can transition to Running, Succeeded, Failed, Cancelled, and Skipped.
- Running can transition to Succeeded, Failed, Waiting, Asking, Cancelled, and Skipped.
- Waiting and Asking can resume only to Running, except idempotent self-transition.
- Terminal states Succeeded, Failed, Cancelled, and Skipped reject outward transitions and permit only idempotent re-mark.
- Runtime `RunFrame` state writes reject invalid transitions without changing stored state.
- Runtime transition predicate delegates to the proof-kernel transition function.

## Test Lanes

- Unit: `cargo test -p vb_proof_kernels step_state -- --nocapture`.
- Unit/integration scoped to runtime frame: `cargo test -p vb_core step_state -- --nocapture`.
- Proof-backed tests: Kani harness `kani_step_state_transition_matches_contract`.
- Model-backed tests: Verus and TLA gates from `proof-evidence.md`.

## Acceptance

- All scoped Rust tests pass.
- Kani, Verus, and TLA proof gates pass.
- Repository-wide formatting drift is recorded but not repaired if outside this bead scope.
