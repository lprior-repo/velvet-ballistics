# State 11 Machine Gate Report - vb-0253.5

STATUS: APPROVED

## PASS Gates

- `cargo test -p vb_proof_kernels step_state -- --nocapture`: `cargo test: 10 passed, 24 filtered out (1 suite, 0.01s)`.
- `cargo test -p vb_core step_state -- --nocapture`: `cargo test: 12 passed, 1888 filtered out (10 suites, 0.01s)`.
- `verus verification/verus/step_state_machine.rs`: `verification results:: 6 verified, 0 errors`.
- `tlc -config specs/tla/StepState.cfg specs/tla/StepState.tla`: `No error has been found`, `5377 states generated`, `512 distinct states found`.
- `cargo kani -p vb_core --harness kani_step_state_transition_matches_contract`: `VERIFICATION:- SUCCESSFUL`.

## DEFERRED_GLOBAL Gates

- `cargo fmt --check` failed on unrelated pre-existing formatting drift outside the StepState scope. Raw output was stored by OpenCode at `/home/lewis/.local/share/opencode/tool-output/tool_e352bc6f4001U2lz1oBigb0Is6`.

## Non-Blocking Tool Notes

- `cargo kani list`: `error: No supported targets were found`; exact named harness execution succeeded.
- `tlc -version`: TLC binary exists but does not support `-version`; version was printed by actual model run.
