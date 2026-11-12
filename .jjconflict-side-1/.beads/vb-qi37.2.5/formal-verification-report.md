# Formal Verification Report — vb-qi37.2.5 State 11 (fresh execution)

STATUS: APPROVED

## Inputs
- proof-obligations.jsonl: `.beads/vb-qi37.2.5/proof-obligations.jsonl` (11 current obligations, valid JSONL; FUZZ-RESOURCE-001 repaired with stdin replay+proptest command).
- delivery-scope.jsonl: `.beads/vb-qi37.2.5/delivery-scope.jsonl` (valid JSONL).
- baseline-report.md: `.beads/vb-qi37.2.5/baseline-report.md`.
- tla-spec.md: `.beads/vb-qi37.2.5/tla-spec.md`.
- lean-contract.md: `.beads/vb-qi37.2.5/lean-contract.md`.
- contract-verification-review.md: `STATUS: APPROVED`.

## Startup Rule Citation
- Mandatory files read: `/home/lewis/.claude/skills/formal-verifier/SKILL.md` and `/home/lewis/.agents/skills/formal-verifier/SKILL.md`.
- Both copies report formal-verifier version 1.5.0 and the same relevant rules; `/home/lewis/.agents/skills/formal-verifier/SKILL.md` wins on conflict.
- Applied rules: exact approved commands, every obligation accounted, fail closed on required scoped failures, no hallucinated evidence.
- Contract verification review is `STATUS: APPROVED`; the repaired proof-obligations.jsonl was produced by the State 3/4/5 repair cycle and consumed here.

## Isolation
- Required workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5`.
- Guard: `test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5"` returned ISOLATION PASS.
- Source checkout `/home/lewis/src/velvet-ballistics` was not used for writes.

## Tool Availability
- verus: `/home/lewis/.local/bin/verus`, version `Verus 0.2026.05.05.d03e906`.
- tlc: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`, TLC2 Version 2.19.
- moon: `moon 2.2.4`.
- cargo-kani: `cargo-kani 0.67.0`.
- miri: `miri 0.1.0 (52b6e2c208 2026-04-27)`.
- cargo-fuzz: `cargo-fuzz 0.13.1`.
- lake: `/home/lewis/.elan/bin/lake`.
- apalache-mc: `/home/lewis/.local/share/mise/installs/http-apalache/0.57.0/bin/apalache-mc`.
- cargo-careful: not found by `command -v cargo-careful || true`.

## Obligation Results

### VERUS-STEP-001 — PASS
- id: VERUS-STEP-001
- risk: proof
- scope: bead-local
- layer: verus
- checker: verus
- command: `RUSTC_WRAPPER= TMPDIR=target/tmp verus verification/verus/step_budget.rs`
- required: true
- owner_state: 4
- rerun_from: 3
- result: PASS
- evidence: exit 0; `verification results:: 6 verified, 0 errors`

### VERUS-BUDGET-001 — PASS
- id: VERUS-BUDGET-001
- risk: proof
- scope: bead-local
- layer: verus
- checker: verus
- command: `RUSTC_WRAPPER= TMPDIR=target/tmp verus verification/verus/resource_budget.rs`
- required: true
- owner_state: 4
- rerun_from: 3
- result: PASS
- evidence: exit 0; `verification results:: 10 verified, 0 errors`

### TLA-SLICE-001 — PASS
- id: TLA-SLICE-001
- risk: proof
- scope: protocol
- layer: tla-plus
- checker: tlc
- command: `JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=$PWD/target/tmp RUSTC_WRAPPER= TMPDIR=target/tmp tlc -metadir /tmp/opencode/tlc-vb-qi37-2-5-slice specs/vb_qi37_2_5/BoundednessSlice.tla -config specs/vb_qi37_2_5/BoundednessSlice.cfg`
- required: true
- owner_state: 4
- rerun_from: 3
- result: PASS
- evidence: exit 0; `Model checking completed. No error has been found.`; `41 states generated, 21 distinct states found`

### TLA-ADMIT-001 — PASS
- id: TLA-ADMIT-001
- risk: proof
- scope: protocol
- layer: tla-plus
- checker: tlc
- command: `JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=$PWD/target/tmp RUSTC_WRAPPER= TMPDIR=target/tmp tlc -metadir /tmp/opencode/tlc-vb-qi37-2-5-nested specs/vb_qi37_2_5/NestedBoundednessAdmission.tla -config specs/vb_qi37_2_5/NestedBoundednessAdmission.cfg`
- required: true
- owner_state: 4
- rerun_from: 3
- result: PASS
- evidence: exit 0; `Model checking completed. No error has been found.`; `301 states generated, 237 distinct states found`

### KANI-LOOP-001 — WAIVED
- id: KANI-LOOP-001
- risk: proof
- scope: bead-local
- layer: waiver
- checker: waiver
- command: WAIVER: no cargo-kani command was run; Kani files are not Cargo-integrated harnesses.
- required: true
- owner_state: 3
- rerun_from: 3
- result: WAIVED
- evidence: `contract-verification-review.md` lines 27-31 approve the Kani non-execution limitation; no Kani PASS is claimed; compensating evidence provided by VERUS-STEP-001, TLA-SLICE-001, and proptest coverage.

### PROP-BUDGET-001 — PASS
- id: PROP-BUDGET-001
- risk: high
- scope: touched-crate
- layer: proptest
- checker: cargo test
- command: five exact `cargo test --package vb_core --lib -- budget::tests::*` commands (property_boundedness_policy, policy_rejects_budget_one_over_total_steps_limit, policy_boundary_fanout_one_over, policy_boundary_nesting_depth_one_over, policy_rejects_steps_executable_exceeded) each with `--nocapture`
- required: true
- owner_state: 6
- rerun_from: 3
- result: PASS
- evidence: all five commands exited 0; each reported `1 passed, 1520 filtered out`

### PROP-VALUE-001 — PASS
- id: PROP-VALUE-001
- risk: high
- scope: touched-crate
- layer: proptest
- checker: cargo test
- command: three exact `cargo test --package vb_core --lib -- value_store::tests::*` commands (property_value_store_cap, value_store_with_max_slots_allows_inserts_up_to_cap, value_store_with_max_slots_one_rejects_second_insert) each with `--nocapture`
- required: true
- owner_state: 6
- rerun_from: 3
- result: PASS
- evidence: all three commands exited 0; each reported `1 passed, 1520 filtered out` (or `1 passed, 4.62s` for the cap test)

### MIRI-VALUE-001 — PASS
- id: MIRI-VALUE-001
- risk: high
- scope: touched-crate
- layer: miri
- checker: moon run :miri
- command: `RUSTC_WRAPPER= TMPDIR=target/tmp moon run :miri`
- required: true
- owner_state: 8
- rerun_from: 3
- result: PASS
- evidence: exit 0; three scoped Miri tests passed; `Tasks: 1 completed`; Time: 1m 7s 228ms

### FUZZ-RESOURCE-001 — PASS (repaired command; old cargo-fuzz waived)
- id: FUZZ-RESOURCE-001
- risk: high
- scope: touched-crate
- layer: cargo-fuzz
- checker: stdin replay + cargo test
- command: exact repaired command from proof-obligations.jsonl: `mkdir -p target/tmp && RUSTC_WRAPPER= TMPDIR=target/tmp cargo build --manifest-path fuzz/Cargo.toml --features fuzz --bin resource_budget && python3 -c "..." (stdin replay 1000 cases) && RUSTC_WRAPPER= TMPDIR=target/tmp PROPTEST_CASES=10000 rtk cargo test --package vb_core --test vb_qi37_2_5_boundedness_adversarial proptest -- --nocapture`
- required: true
- owner_state: 8
- rerun_from: 3
- result: PASS
- evidence: exit 0; `resource_budget stdin replay PASS cases=1000`; `cargo test: 3 passed, 19 filtered out`
- waiver: old `cargo fuzz run resource_budget -- -runs=1000` command is waived per `waived_command` field in proof-obligations.jsonl and contract-verification-review.md; cargo-fuzz selects static musl target incompatible with ASAN in this environment.

### STATIC-NOPANIC-001 — PASS
- id: STATIC-NOPANIC-001
- risk: high
- scope: touched-crate
- layer: static-scan
- checker: moon run :lint-src
- command: `RUSTC_WRAPPER= TMPDIR=target/tmp moon run :lint-src`
- required: true
- owner_state: 8
- rerun_from: 3
- result: PASS
- evidence: exit 0; `Tasks: 1 completed`; Time: 808ms

### DEFERRED-GLOBAL-001 — DEFERRED_GLOBAL
- id: DEFERRED-GLOBAL-001
- risk: medium
- scope: workspace
- layer: waiver
- checker: waiver
- command: Record DEFERRED_GLOBAL if full workspace build hits `crates/vb_runtime/src/runtime.rs` missing `runtime/chunk_001.rs`
- required: true
- owner_state: 12
- rerun_from: 3
- result: DEFERRED_GLOBAL
- evidence: `delivery-scope.jsonl` marks `crates/vb_runtime/src/runtime.rs` / `chunk_001.rs` as deferred-global and outside bead-local boundedness scope. Focused State 11 gates did not encounter this failure.
- follow_up: Track/repair `crates/vb_runtime/src/runtime.rs` missing `runtime/chunk_001.rs` in a separate workspace/global bead; do not charge to vb-qi37.2.5 local boundedness evidence.

## Focused/Canonical Gates Rerun Evidence
- focused integration compile/test: PASS; `cargo test: 22 passed`
- extended proptest: PASS; `3 passed, 19 filtered out`
- repaired stdin replay: PASS; `resource_budget stdin replay PASS cases=1000`
- GNU target fuzz (repair evidence only): `cargo fuzz run --target x86_64-unknown-linux-gnu resource_budget -- -runs=1000` exited 0 (informational; not the approved command after repair)

## Waivers
- `KANI-LOOP-001`: accepted by `contract-verification-review.md`; no Kani PASS claimed; compensating evidence from VERUS-STEP-001, TLA-SLICE-001, and proptest coverage.
- `FUZZ-RESOURCE-001` old cargo-fuzz command: waived in proof-obligations.jsonl `waived_command` field; cargo-fuzz musl/ASAN incompatibility is an environment constraint, not a behavioral regression.

## Residual Risk
- No required/local obligations remain unpassed or unwaived.
- `DEFERRED-GLOBAL-001` is a pre-existing workspace issue unrelated to this bead's boundedness scope.
- Minor residual: Kani loop bounds are not formally verified, compensated by Verus step-budget lemmas and TLA+ state-space model checking.

## Decision
- STATUS: APPROVED.
- All 11 proof obligations: 9 PASS, 1 WAIVED, 1 DEFERRED_GLOBAL (pre-existing unrelated).
- No FAIL_LOCAL, FAIL_REGRESSION, or REQUIRED_OBLIGATION_FAIL entries.
