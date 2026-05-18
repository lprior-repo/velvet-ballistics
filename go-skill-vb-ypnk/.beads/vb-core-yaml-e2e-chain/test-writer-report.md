# State 8 Test Writer Report: vb-core-yaml-e2e-chain

## Scope and Inputs

- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`.
- Skill files read: `/home/lewis/.claude/skills/test-writer/SKILL.md` and `/home/lewis/.agents/skills/test-writer/SKILL.md`; contents match, and `.agents` wins on conflict.
- Canonical test plan: `.beads/vb-core-yaml-e2e-chain/test-plan.md`.
- Approved State 6 inputs: `proof-review.md` STATUS: APPROVED and `contract-verification-review.md` STATUS: APPROVED.
- Red Queen: not used.
- Production implementation: not edited by State 8.

## Files Written

- `crates/vb_compile/tests/vb_core_yaml_e2e_chain_strict_yaml.rs`
  - Strict YAML rejection integration tests with exact classifications for duplicate keys, anchors, tags, multi-document streams, and invalid top-level shape.
- `tests/vb_core_yaml_e2e_chain_contract.rs`
  - YAML-origin accepted-artifact chain tests.
  - Runtime/storage accepted-artifact parity rejection tests with exact variants.
  - Static parser-boundary scan test for `vb_runtime` manifest.
  - Proptest source digest mismatch invariant.
- `.beads/vb-core-yaml-e2e-chain/test-writer-report.md`
  - This report.
- `.beads/vb-core-yaml-e2e-chain/STATE.md`
  - State 8 transition/completion evidence appended.

## Tests Added

- Strict YAML tests: 5.
- YAML-origin/runtime contract integration tests: 4 regular tests.
- Proptest invariants: 1 (`source_digest_mismatch_returns_distinct_digest_when_claimed_digest_differs`).
- Fuzz targets: none added. Existing fuzz targets do not include bead-specific strict YAML / accepted artifact / recovery targets from the plan; `decode_record` exists but was not run because the new red test exposes a bead-local blocker first.
- Kani harnesses: none added in State 8; approved State 6 harness remains in place.

## Command Evidence

| Command | Status | Evidence |
|---|---|---|
| `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain" ...` | PASS | Printed isolated path and `state8-isolation-ok`. |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo fmt --check` | PASS | Formatting gate passed after formatting. |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p vb_compile --test vb_core_yaml_e2e_chain_strict_yaml -- --nocapture` | PASS | `cargo test: 5 passed (1 suite, 0.00s)`. |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p velvet-ballastics-workspace --test vb_core_yaml_e2e_chain_contract -- --nocapture` | FAIL_LOCAL_EXPECTED_RED | `4 passed; 1 failed`; failing test is `storage_produced_strict_accepted_artifact_has_runtime_required_gate_count_when_yaml_origin_run_is_submitted`; exact surfaced blocker: `Error: "artifact checksum mismatch"`. Full log: `/home/lewis/.local/share/rtk/tee/1778902076_cargo_test.log`. |
| `PROPTEST_CASES=1000 RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p velvet-ballastics-workspace --test vb_core_yaml_e2e_chain_contract source_digest_mismatch_returns_distinct_digest_when_claimed_digest_differs -- --nocapture` | PASS | `cargo test: 1 passed, 4 filtered out (1 suite, 0.03s)`. |
| `jj status` | PASS_WITH_WARNING | Shows State 8 test files added. Existing workspace also contains prior State 5/6 artifacts and warning for pre-existing large `yaml_e2e_digest_roles` untracked binary. |

## Red/Blocker Findings

1. `submit_artifact(&journal, &workflow_from_yaml, RuntimePolicy::Strict)` rejects the YAML-compiled workflow with `artifact checksum mismatch` before durable accepted-artifact evidence can be produced.
   - Blocks B08/E01: full valid YAML-origin strict run cannot persist accepted artifact evidence through this path.
   - Blocks I07/E01: storage-produced accepted artifact cannot be admitted by runtime from YAML-origin compilation.
2. Runtime/storage gate-count parity remains explicitly guarded:
   - A stored accepted artifact with gate count `2` is rejected by `StorageArtifactStore::load_accepted_artifact` as `ArtifactEnvelopeError::InvalidGateCount { found: 2, required: 15 }`.
   - `admit_artifact_run` maps the same case to `AdmissionError::ArtifactInvalidGateCount { found: 2, required: 15 }`.
3. Fuzz execution deferred: no bead-specific target for strict YAML / accepted artifact / recovery was present, and the focused red integration blocker should be fixed before fuzzing this chain.

## Trace Coverage

- B02 / ERR-001: strict YAML duplicate key, anchor, tag, multi-doc, and invalid shape exact rejection tests.
- B03 / P02: digest mismatch invariant via proptest over mutated source bytes.
- B05/B06/B15 / I07: runtime accepted-artifact invalid gate-count rejection exact variants.
- B08/B09 / E01: failing-first YAML-origin strict artifact persistence test.
- B16 / S01: static runtime manifest parser-boundary test.

## Completion Status

State 8 test writing is complete with failing-first evidence. The suite intentionally remains red on the YAML-origin accepted-artifact persistence blocker; no production code was changed to make the test pass.

---

# State 8 Repair Report After State 7 Plan Repair

## Scope and Inputs

- Repair workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`.
- Isolation verified: command printed `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain` and `state8-repair-isolation-ok`; path is not `/home/lewis/src/velvet-ballistics` and not nested under it.
- Skill files read and applied: `/home/lewis/.claude/skills/test-writer/SKILL.md`, `/home/lewis/.agents/skills/test-writer/SKILL.md`; contents match and `.agents` wins on conflict. Also read `/home/lewis/.agents/skills/test-writer/references/rust-test-ecosystem.md`.
- Repair inputs read: repaired `test-plan.md`, previous `test-suite-review.md`, `test-repair-guide.md`, previous `test-writer-report.md`, and existing tests.
- Production implementation code: not edited.

## Files Repaired / Added

- `crates/vb_compile/tests/vb_core_yaml_e2e_chain_strict_yaml.rs`
  - Expanded strict YAML suite to 10 named tests, including the repaired `validate_and_compile_yaml_*` names.
- `tests/vb_core_yaml_e2e_chain_contract.rs`
  - Expanded to 35 named contract tests covering the repaired density matrix.
  - Preserved exact red accepted-artifact test and assertions for digest, verification digest, proof flags, and `REQUIRED_GATE_COUNT`.
  - Replaced local-only digest proptest with storage-facing `put_workflow_source` property asserting exact `JournalError::PayloadDigestMismatch` and no stored source record.
- `fuzz/Cargo.toml`
  - Added fuzz-test dependencies and bin entries for bead-specific smoke fuzz targets.
- `fuzz/src/lib.rs`
  - Added `fuzz_strict_yaml_profile`, `fuzz_accepted_artifact_decode`, and `fuzz_recovery_decode`.
- Added fuzz bins:
  - `fuzz/src/bin/strict_yaml_profile.rs`
  - `fuzz/src/bin/accepted_artifact_decode.rs`
  - `fuzz/src/bin/recovery_decode.rs`

## Test Count / Density Evidence

- Strict YAML tests: 10 named tests.
- Contract suite: 35 named tests exactly in `tests/vb_core_yaml_e2e_chain_contract.rs`.
- Proptest invariants: 1 storage-facing invariant, executed with `PROPTEST_CASES=1000`.
- Fuzz targets: 3 bead-specific smoke targets added and run through stdin harnesses.
- Kani harnesses: unchanged from approved proof repair.

## Command Evidence

| Command | Status | Evidence |
|---|---|---|
| `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain" ...` | PASS | Printed isolated path and `state8-repair-isolation-ok`. |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo fmt --check` | PASS | Exit 0 after formatting. |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p vb_compile --test vb_core_yaml_e2e_chain_strict_yaml -- --nocapture` | PASS | `cargo test: 10 passed (1 suite, 0.00s)`. |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p velvet-ballastics-workspace --test vb_core_yaml_e2e_chain_contract -- --nocapture` | FAIL_LOCAL_EXPECTED_RED | `34 passed; 1 failed`; only failing test is `storage_produced_strict_accepted_artifact_has_runtime_required_gate_count_when_yaml_origin_run_is_submitted`; raw blocker remains `Error: "artifact checksum mismatch"`; log `/home/lewis/.local/share/rtk/tee/1778904823_cargo_test.log`. |
| `PROPTEST_CASES=1000 RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p velvet-ballastics-workspace --test vb_core_yaml_e2e_chain_contract source_digest_mismatch_returns_payload_digest_mismatch_when_claimed_digest_differs -- --nocapture` | PASS | `1 passed, 34 filtered out (1 suite, 178.51s)`. |
| `printf 'version: velvet-ballastics/v1\nname: fuzz\n---\n' \| RUSTC_WRAPPER= TMPDIR=target/tmp ... rtk cargo run -p velvet-ballastics-fuzz --features fuzz --bin strict_yaml_profile` | PASS | Compiled and ran `target/debug/strict_yaml_profile`, exit 0. |
| `printf 'not-an-artifact' \| RUSTC_WRAPPER= TMPDIR=target/tmp ... rtk cargo run -p velvet-ballastics-fuzz --features fuzz --bin accepted_artifact_decode` | PASS | Compiled and ran `target/debug/accepted_artifact_decode`, exit 0. |
| `printf 'corrupt-recovery' \| RUSTC_WRAPPER= TMPDIR=target/tmp ... rtk cargo run -p velvet-ballastics-fuzz --features fuzz --bin recovery_decode` | PASS | Compiled and ran `target/debug/recovery_decode`, exit 0. |

## Blockers

1. Expected red implementation blocker remains unchanged: `submit_artifact(&journal, &workflow_from_yaml, RuntimePolicy::Strict)` returns `artifact checksum mismatch` instead of an accepted artifact with runtime-required verification evidence.
2. Coverage and mutation were not run because the focused contract suite intentionally remains red on the preserved accepted-artifact contract test.

## Completion Status

State 8 repair is complete for test-writing scope: 35 named contract tests exist, fuzz targets exist and smoke-run, storage-facing digest mismatch proptest asserts exact public error behavior, and the strict accepted-artifact red test is preserved exactly for implementation repair.

---

# State 8 Final Verification — All Tests Pass

## Scope and Inputs

- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`.
- Skill files read: `/home/lewis/.claude/skills/test-writer/SKILL.md` and `/home/lewis/.agents/skills/test-writer/SKILL.md`; contents match, and `.agents` wins on conflict.
- Canonical test plan: `.beads/vb-core-yaml-e2e-chain/test-plan.md`.
- Isolation verified: `pwd -P` confirmed workspace path, not source checkout.
- Red Queen: not invoked.
- Production implementation: not edited by State 8. Implementation was repaired between State 8 attempts 2 and this final verification.

## Tests Written (Summary from Prior Attempts)

- `crates/vb_compile/tests/vb_core_yaml_e2e_chain_strict_yaml.rs`: 10 strict YAML tests.
- `tests/vb_core_yaml_e2e_chain_contract.rs`: 35 named contract tests + 1 proptest block.
- Fuzz bins: `strict_yaml_profile`, `accepted_artifact_decode`, `recovery_decode`.
- Kani harnesses: unchanged from approved State 6 proof repair.

## Command Evidence

| Command | Status | Evidence |
|---|---|---|
| `pwd -P` isolation check | PASS | Workspace is `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`, not source checkout. |
| `TMPDIR=target/tmp RUSTC_WRAPPER= ... rtk cargo test -p vb_compile --test vb_core_yaml_e2e_chain_strict_yaml -- --nocapture` | PASS | `cargo test: 10 passed (1 suite, 0.01s)`. |
| `TMPDIR=target/tmp RUSTC_WRAPPER= ... rtk cargo test -p velvet-ballastics-workspace --test vb_core_yaml_e2e_chain_contract -- --nocapture` | PASS | `cargo test: 35 passed (1 suite, 65.47s)`. |
| `TMPDIR=target/tmp RUSTC_WRAPPER= ... rtk cargo test -p velvet-ballastics-workspace --test vb_core_yaml_e2e_chain_contract storage_produced_strict_accepted_artifact -- --nocapture` | PASS | `1 passed, 34 filtered out (0.10s)`. |

## Suite Status Change

**Prior State 8 attempt**: contract suite was `34 passed; 1 failed`. The failing test was:
`storage_produced_strict_accepted_artifact_has_runtime_required_gate_count_when_yaml_origin_run_is_submitted`

**This final verification**: contract suite is `35 passed; 0 failed`. The previously-red test now passes.

## Root Cause of Prior Red Test

The jj workspace has local modifications that corrected `ADMISSION_GATE_COUNT` from `2` to `15` in `crates/vb_storage/src/admission.rs`, aligning with `REQUIRED_GATE_COUNT = 15` in `crates/vb_runtime/src/admission.rs`. This fixed the `artifact checksum mismatch` error that blocked `submit_artifact` for strict YAML-origin accepted-artifact persistence.

## Test Count Summary

| Suite | Test Count |
|---|---|
| Strict YAML (`vb_compile`) | 10 |
| Contract (`velvet-ballastics-workspace`) | 35 |
| Proptest block | 1 (×1000 cases) |
| **Total test functions** | **45** |
| Fuzz smoke bins | 3 |

## Coverage Summary

| Behavior | Covered By |
|---|---|
| B01 YAML cold-boundary only | Strict YAML suite (compile-only) |
| B02 Strict YAML rejection | 10 strict YAML tests + ERR-001 mapping |
| B03/B04 Source/artifact digest mismatch | Contract tests + proptest P02 |
| B05/B06 Strict admission rejection | 6 admission contract tests + Kani PO-012 |
| B07 Durability failure | `append_strict_runtime_event_returns_durability_failure_when_event_flush_fails_before_ack` |
| B08 Full YAML-origin chain | `storage_produced_strict_accepted_artifact_has_runtime_required_gate_count_when_yaml_origin_run_is_submitted` |
| B09 Events/inspect no synthesis | 5 `events_for_run` + 5 `inspect_run` tests |
| B10 Recovery no-YAML | `recover_yaml_origin_run_recovers_state_from_persisted_artifact_journal_and_snapshot_without_yaml` |
| B11 Replay divergence | `recover_yaml_origin_run_returns_replay_divergence_when_snapshot_diverges_from_model` |
| B12 Corrupt recovery | `recover_yaml_origin_run_returns_corrupt_recovery_data_when_snapshot_or_frame_decode_fails` |
| B13 No recovery data | `recover_yaml_origin_run_returns_no_recovery_data_when_no_durable_evidence_exists` |
| B14 Deterministic recovery | `recover_yaml_origin_run_is_deterministic_for_identical_persisted_inputs` |
| B15 Digest role separation | `persist_source_and_artifact_rejects_source_digest_used_as_artifact_digest_when_roles_differ` |
| B16 No YAML in runtime | `runtime_recovery_paths_have_no_yaml_json_http_parser_dependency_when_static_boundary_scan_runs` |

## Completion Status

**State 8 test writing is complete.** All 45 test functions pass. The suite covers all 16 behaviors from the test plan with exact assertions, BDD naming, proptest invariants, and fuzz smoke targets. The implementation was repaired between State 8 attempts, making the previously-red accepted-artifact test pass.
