STATUS: PASS

# State 11 Machine Gate Report

Commands executed from `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5`:
- `rtk cargo fmt -p vb_compile && TMPDIR=target/tmp rtk cargo test -p vb_compile --test idempotency_parity`: PASS, 9 tests.
- `TMPDIR=target/tmp cargo kani -p vb_compile --harness idempotency_gate_parity --output-format=regular`: PASS, `VERIFICATION:- SUCCESSFUL`, raw `/home/lewis/.local/share/opencode/tool-output/tool_e35595389001V8cydoKJUYkkZC`.
- `TMPDIR=target/tmp rtk cargo clippy -p vb_compile -p vb_validate -p vb_core -- -D warnings -D clippy::unwrap_used -D clippy::panic -D clippy::expect_used`: PASS.
- `TMPDIR=target/tmp rtk cargo test -p vb_validate -p vb_core -p vb_compile`: PASS, 3070 tests across 20 suites.
- `TMPDIR=target/tmp tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla`: PASS.
- `TMPDIR=target/tmp verus verification/verus/idempotency_decision.rs`: PASS.
- `TMPDIR=target/tmp verus verification/verus/idempotency_certificate_summary.rs`: PASS.
- `TMPDIR=target/tmp verus verification/verus/idempotency_replay_tracker.rs`: PASS.
- `TMPDIR=target/tmp cargo kani -p vb_validate --output-format=regular`: PASS, raw `/home/lewis/.local/share/opencode/tool-output/tool_e355af7c6001ZoNsPBMDGbb52x`.
- `TMPDIR=target/tmp cargo kani -p vb_core --harness verify_idempotency_all_clean --harness verify_idempotency_missing_key --harness verify_idempotency_secret_in_key --harness verify_idempotency_random_in_key --harness verify_idempotency_time_in_key --harness verify_idempotency_single_error --output-format=regular`: PASS, raw `/home/lewis/.local/share/opencode/tool-output/tool_e355e8930001J3f32yV3GYKELp`.

Waived tooling blocker:
- `FUZZ-ARTIFACT-011`: see `formal-waivers.jsonl`; no fuzz pass claimed.
