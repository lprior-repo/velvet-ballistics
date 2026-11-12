STATUS: APPROVED

# State 6 Proof Review Rerun - 2026-05-17

The prior rejection is resolved for KANI-PARITY-006. The compile-side decision table now rejects side-effecting DeterministicPure and the Kani harness checks all 45 combinations without `kani::assume` exclusions.

Fresh evidence:
- `TMPDIR=target/tmp cargo kani -p vb_compile --harness idempotency_gate_parity --output-format=regular`: PASS, `VERIFICATION:- SUCCESSFUL`, `0 of 145 failed`, raw `/home/lewis/.local/share/opencode/tool-output/tool_e35595389001V8cydoKJUYkkZC`.
- `TMPDIR=target/tmp cargo kani -p vb_validate --output-format=regular`: PASS, `5 successfully verified harnesses, 0 failures`, raw `/home/lewis/.local/share/opencode/tool-output/tool_e355af7c6001ZoNsPBMDGbb52x`.
- `TMPDIR=target/tmp cargo kani -p vb_core --harness verify_idempotency_all_clean --harness verify_idempotency_missing_key --harness verify_idempotency_secret_in_key --harness verify_idempotency_random_in_key --harness verify_idempotency_time_in_key --harness verify_idempotency_single_error --output-format=regular`: PASS, `6 successfully verified harnesses, 0 failures`, raw `/home/lewis/.local/share/opencode/tool-output/tool_e355e8930001J3f32yV3GYKELp`.
- `TMPDIR=target/tmp tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla`: PASS, no error, 238912 states generated, 82192 distinct states, depth 7.
- `TMPDIR=target/tmp verus verification/verus/idempotency_decision.rs`: PASS, 8 verified, 0 errors.
- `TMPDIR=target/tmp verus verification/verus/idempotency_certificate_summary.rs`: PASS, 6 verified, 0 errors.
- `TMPDIR=target/tmp verus verification/verus/idempotency_replay_tracker.rs`: PASS, 5 verified, 0 errors.

FUZZ-ARTIFACT-011 remains waived as BLOCKED_TOOLING, not passed. The waiver is acceptable because the exact target exists (`cargo fuzz list`: `admission_fuzz`), both sanitizer and sanitizer-none executions fail before target execution due local musl/libfuzzer toolchain absence, and compensating Kani/TLA/Verus/test evidence covers the idempotency decision and replay/admission properties in this bead's blast radius.
