# Proof Evidence - vb-0253.5

STATUS: PASS

## Tool Versions

- Kani: `cargo-kani 0.67.0`.
- Verus: `0.2026.05.05.d03e906`, platform `linux_x86_64`, toolchain `1.95.0-x86_64-unknown-linux-gnu`.
- TLC: `TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)`.

## Kani Evidence

Command: `cargo kani -p vb_core --harness kani_step_state_transition_matches_contract`

Result:

```text
Check 5: pending idempotent transition covered - SATISFIED
Check 6: terminal outward rejection covered - SATISFIED
Check 7: suspended resume transition covered - SATISFIED
Check 8: runtime StepState transition predicate matches formal contract - SUCCESS
SUMMARY:
 ** 0 of 98 failed (1 unreachable)
 ** 3 of 3 cover properties satisfied
VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

Kani surface scan for scoped file `crates/vb_core/src/kani_step_state_transition.rs` found only:

- `impl kani::Arbitrary for StepState`
- `kani::any::<u8>()`
- `#[kani::proof]`
- two symbolic `kani::any()` inputs
- three `kani::cover(...)` calls
- one `kani::assert(...)` parity assertion

No `kani::assume`, stubs, contracts, or disabled-check flags were used in the accepted harness command.

## Verus Evidence

Command: `verus verification/verus/step_state_machine.rs`

Result:

```text
verification results:: 6 verified, 0 errors
```

Trusted-boundary scan for `verification/verus/step_state_machine.rs` found no `assume(`, `#[verifier::external_body]`, `#[verifier::external]`, or `axiom` matches.

## TLA+ Evidence

Command: `tlc -config specs/tla/StepState.cfg specs/tla/StepState.tla`

Result:

```text
Model checking completed. No error has been found.
5377 states generated, 512 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 7.
```

Model bounds:

- `StepId = {1, 2, 3}`.
- Invariants checked: `TypeInvariant`, `TerminalStateBlocksOutwardTransitions`.

## Rust Test Evidence

- `cargo test -p vb_proof_kernels step_state -- --nocapture`: `10 passed, 24 filtered out`.
- `cargo test -p vb_core step_state -- --nocapture`: `12 passed, 1888 filtered out`.

## Non-Blocking Negative Evidence

- `cargo kani list --format json`: no output in this repo/tool version.
- `cargo kani list`: `error: No supported targets were found.` The named harness command still discovered and verified the scoped harness successfully.
- `verus crates/vb_proof_kernels/src/step_state.rs`: fails because production Rust lacks Verus `vstd` import. This validates that direct production-file Verus was not falsely claimed.
