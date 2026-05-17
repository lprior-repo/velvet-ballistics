# vb-ebjo - Resolve step-state Verus transition mismatch

STATUS: IMPLEMENTED

## Canonical contract

`verification/verus/step_state_machine.rs` now mirrors `crates/vb_proof_kernels/src/step_state.rs` and the primary L4 registry table:

- `Pending -> Running | Succeeded | Failed | Cancelled | Skipped`.
- `Running -> Succeeded | Failed | Waiting | Asking | Cancelled | Skipped`.
- `Waiting -> Running`.
- `Asking -> Running`.
- Terminal states (`Succeeded`, `Failed`, `Cancelled`, `Skipped`) transition only to themselves.
- Non-terminal self transitions are rejected, including `Running -> Running`.

## Evidence

- `verus verification/verus/step_state_machine.rs` -> `verification results:: 9 verified, 0 errors`.
- `bash scripts/verify-verus.sh` executes the canonical registry target and passes.

## Boundary

This bead resolves the registry/Verus proof mismatch. It deliberately does not edit production runtime transition behavior because that would be a behavior change outside the proof-artifact scope and there are unrelated dirty local edits in adjacent crates.
