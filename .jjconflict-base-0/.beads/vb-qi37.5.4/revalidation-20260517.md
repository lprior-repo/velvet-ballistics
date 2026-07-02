# vb-qi37.5.4 Current Evidence Revalidation

STATUS: APPROVED

The previous State 13 evidence was stale because it relied on a 37-combination Kani scope reduction. Current isolated source in `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5` no longer relies on that exclusion.

Fresh validation:
- `crates/vb_compile/tests/idempotency_parity.rs` now contains `parity_exhaustive_all_45_cases`.
- `crates/vb_compile/src/kani_idempotency_parity.rs` states all 45 combinations and contains no disagreement exclusion.
- `TMPDIR=target/tmp cargo kani -p vb_compile --harness idempotency_gate_parity --output-format=regular`: PASS, `VERIFICATION:- SUCCESSFUL`, raw `/home/lewis/.local/share/opencode/tool-output/tool_e35595389001V8cydoKJUYkkZC`.
- `TMPDIR=target/tmp rtk cargo test -p vb_validate -p vb_core -p vb_compile`: PASS, 3070 tests.
- `TMPDIR=target/tmp rtk cargo clippy -p vb_compile -p vb_validate -p vb_core -- -D warnings -D clippy::unwrap_used -D clippy::panic -D clippy::expect_used`: PASS.

Decision: valid for State 13 bookmark-ready under bookmark `go-skill-p0-vb-qi37-5-4`. Stop before main merge.
