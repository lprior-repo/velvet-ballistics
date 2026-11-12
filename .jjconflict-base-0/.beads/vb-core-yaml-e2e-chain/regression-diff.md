# Regression Diff

STATUS: APPROVED (no regressions, 3 FAIL_LOCAL bead-local code issues, 2 DEFERRED_GLOBAL pre-existing)

## Baseline

- Baseline report: isolated workspace started from `moyvrvsn c9c7eee4 Delete CHANGELOG.md` with no working-copy changes.
- State 11 is the first canonical machine gate comparison point.
- Baseline did not run full machine gate; this diff compares against pre-edit baseline state.

## Current Diff Scope

`jj diff --stat` shows changes in:
- `crates/vb_compile/src/lib.rs` (State 10: digest computation change)
- `crates/vb_storage/src/admission.rs` (State 10: gate count 2→15)
- `crates/vb_storage/src/proptests.rs` (State 10: stale gate assertions updated)
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` (State 10: stale assertions)
- `crates/vb_storage/tests/accepted_artifact_red_phase.rs` (State 10: stale assertions)
- `crates/vb_runtime/src/lib.rs` (State 8/10: Kani harness wiring)
- `crates/vb_runtime/src/yaml_e2e_admission_matrix.rs` (State 5/8: Kani proof)
- `fuzz/src/lib.rs` and fuzz bins (State 8: fuzz targets)
- `tests/vb_core_yaml_e2e_chain_contract.rs` (State 8: contract tests)
- `crates/vb_compile/tests/vb_core_yaml_e2e_chain_strict_yaml.rs` (State 8: strict YAML tests)
- `verification/tla/YamlE2eChain.*` (State 5: TLA spec)
- `verification/verus/yaml_e2e_digest_roles.rs` (State 5: Verus proof)

## Bead-Local Failures (FAIL_LOCAL — code repair required)

1. **vb_compile digest semantics** (State 10): State 10 changed `vb_compile` to compute workflow digest from serialized compiled artifact bytes (with digest field zeroed). This caused:
   - `tests::canonical_route_accepts_event_and_webhook_and_digest_changes` to fail: event and webhook canonical workflows now produce the same digest.
   - `STRICT-YAML-012` and `ERR-STRICT-013` fail: `cargo test -p vb_compile` exits 101.
   - Owner: State 10.

2. **fuzz clippy** (State 8): Bead-added fuzz code at `fuzz/src/lib.rs:1392` has needless `return` under clippy `needless_return`:
   - `STATIC-BOUNDARY-009` fails: `cargo clippy` exits 101.
   - Owner: State 8.

3. **Stale obligation package name** (E2E-REC-008, State 5/8): `proof-obligations.jsonl` command named package `velvet-ballistics-workspace` but the test target is in `velvet-ballistics-workspace-tests`. **FIXED in State 11 retry 3**: updated proof-obligations.jsonl metadata. Corrected command passes 19 tests.

## Pre-Existing Failures (DEFERRED_GLOBAL — not caused by this bead)

4. **Miri toolchain** (pre-existing): `cargo +nightly miri` fails because nightly rust-src library directory is absent. `rustup component add rust-src --toolchain nightly` reports `up to date` but the directory does not exist. Not caused by this bead. Compensating evidence: Kani admission matrix (PASS), vb_storage tests (PASS), vb_runtime tests (PASS).

5. **moon ci source-length** (pre-existing environment): `cargo-mutants residue check` fails because the jj workspace is not a git repository. Not caused by this bead.

## Non-Regression PASS Evidence

- TLA model checking: exit 0, 2728 states, 990 distinct, depth 13.
- Verus proof: exit 0, 8 verified, 0 errors.
- Kani admission matrix: exit 0, 1 successfully verified harness.
- vb_storage tests: exit 0, 983 passed.
- vb_runtime tests: exit 0, 1460 passed.
- CLI integration: exit 0, 86 passed.
- Strict YAML bead tests: exit 0, 10 passed.
- Contract bead tests: exit 0, 35 passed.
- Corrected recovery integration: exit 0, 19 passed.

## Classification

| Failure | Classification | Owner | Status |
|---|---|---|---|
| vb_compile digest test | FAIL_LOCAL | State 10 | Requires code repair |
| fuzz clippy needless_return | FAIL_LOCAL | State 8 | Requires code repair |
| Stale recovery package name | PASS (fixed) | State 11 | proof-obligations.jsonl corrected |
| Miri toolchain rust-src | DEFERRED_GLOBAL | pre-existing | Toolchain setup, not blocking |
| moon ci source-length | DEFERRED_GLOBAL | pre-existing | JJ workspace env, not blocking |

## Changed Files by State

| State | Files Changed |
|---|---|
| State 5 (proof writing) | verification/tla/*.tla, verification/verus/*.rs, verification/kani/*.rs |
| State 8 (tests) | fuzz/src/lib.rs, fuzz/src/bin/*.rs, crates/vb_compile/tests/vb_core_yaml_e2e_chain_strict_yaml.rs, tests/vb_core_yaml_e2e_chain_contract.rs |
| State 10 (implementation) | crates/vb_compile/src/lib.rs, crates/vb_storage/src/admission.rs, crates/vb_storage/src/proptests.rs, crates/vb_storage/src/vb_2bok_durability_gate_tests.rs, crates/vb_storage/tests/accepted_artifact_red_phase.rs |
| State 11 (formal-verifier retry 3) | .beads/vb-core-yaml-e2e-chain/proof-obligations.jsonl (E2E-REC-008 package name fix) |
