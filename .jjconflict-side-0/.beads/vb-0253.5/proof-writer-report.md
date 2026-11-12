# State 5 Proof Writer Report - vb-0253.5

STATUS: COMPLETE

## Scope

- Runtime predicate: `crates/vb_core/src/frame.rs::is_valid_step_state_transition`.
- Proof kernel: `crates/vb_proof_kernels/src/step_state.rs`.
- Kani parity harness: `crates/vb_core/src/kani_step_state_transition.rs`.
- Verus model: `verification/verus/step_state_machine.rs`.
- TLA+ model: `specs/tla/StepState.tla` and `specs/tla/StepState.cfg`.

## Artifacts Written Or Reused

- Reused existing proof kernel transition matrix with eight `StepState` variants.
- Reused runtime delegation from `vb_core::frame` into `vb_proof_kernels::step_state::is_valid_transition`.
- Reused Kani symbolic all-pairs parity harness with `kani::Arbitrary` for runtime `StepState`.
- Reused Verus model proving all reviewed transition pairs, terminal outward blocking, suspended resume restriction, and idempotent re-mark.
- Reused TLA+ finite model proving type preservation and terminal outward blocking for three bounded steps.

## Command Evidence

- `verus verification/verus/step_state_machine.rs`: `verification results:: 6 verified, 0 errors`.
- `tlc -config specs/tla/StepState.cfg specs/tla/StepState.tla`: `No error has been found`, `5377 states generated`, `512 distinct states found`, depth `7`.
- `cargo kani -p vb_core --harness kani_step_state_transition_matches_contract`: `VERIFICATION:- SUCCESSFUL`, 1 harness verified, 0 failures, 3/3 cover properties satisfied.
- `cargo test -p vb_proof_kernels step_state -- --nocapture`: `10 passed, 24 filtered out`.
- `cargo test -p vb_core step_state -- --nocapture`: `12 passed, 1888 filtered out`.

## Notes

- Planned State 4 command text named stale paths/harnesses (`step_state_transition`, `vb_proof_kernels/src/step_state.rs`). State 5 resolves the executable proof lane to the actual checked artifacts above.
- `verus crates/vb_proof_kernels/src/step_state.rs` fails because production Rust is not a Verus module importing `vstd`; this is not used as acceptance evidence. Acceptance is through the Verus model plus Kani runtime-to-contract parity.
