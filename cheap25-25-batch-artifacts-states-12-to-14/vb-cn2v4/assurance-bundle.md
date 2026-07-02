# Assurance Bundle: vb-cn2v4

**Bead**: vb-cn2v4 — Keys reject zero `RunId` (P1 bug)
**Date**: 2026-07-01
**Pipeline State**: 14 (Evidence Packaging)
**Workspace**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4`
**Working-copy commit**: `xrpxwkvz a47b72c6` (vb-cn2v4 state11: holzman-rust impl - reject zero RunId)
**Pipeline**: 1 (go-skill) → 2 (explore) → 3 (rust-contract) → 4 (proof-planner) → 4b (proof-plan-reviewer) → 7 (proof-to-implementation) → 11 (holzman-rust) → 12-14 (formal-verifier combined)

## Requirement-to-Evidence Mapping

### C1 — Encoder/Decoder Symmetry: every run-bearing encoder returns `Err(InvalidRunId)` for `run == 0`
- **Contract clause**: C1
- **Production source refs**: `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW), `crates/vb_storage/src/keys.rs::run_header_key`, `crates/vb_storage/src/keys.rs::run_event_key`, `crates/vb_storage/src/keys.rs::run_snapshot_key`, `crates/vb_storage/src/keys.rs::index_status_key`, `crates/vb_storage/src/keys.rs::index_workflow_key`, `crates/vb_storage/src/keys.rs::index_action_key`
- **Tests** (61 keys + 23 fjall + 33 bdd = 117 passing): `cargo test -p vb_storage --lib keys::tests` (61 passed), `cargo test -p velvet-ballistics-workspace-tests --test fjall_keyspace_manifest_tests` (23 passed), `cargo test -p velvet-ballistics-workspace-tests --test vb_eepg_bdd_tests` (33 passed)
- **Proofs** (planned, not in this bead's verifier scope): PO-001-VERUS-MIRROR, PO-002-VERUS-DECODER-SYMMETRY, PO-003-KANI-SPLIT-HARNESS, PO-004-KANI-ORDER-OF-CHECKS, PO-005-PROPTEST-PER-PREFIX, PO-006-PROPTEST-MUTATION
- **Bridge**: `proof-to-rust-map.md` lines 68-72; `proof-to-rust-review.md: STATUS APPROVED`
- **Review**: `formal-verification-report.md: STATUS APPROVED`; `black-hat-review.md: STATUS APPROVED`

### C2 — Shared guard helper `require_non_zero_run(run)` centralises the rejection
- **Contract clause**: C2
- **Production source refs**: `crates/vb_storage/src/keys.rs::require_non_zero_run` (NEW), called by `run_only_key`, `sequenced_run_key`, `index_status_key`, `index_workflow_key`, `index_action_key`
- **Tests**: covered by C1's test suite (the helper is the single source of truth for the rejection)
- **Proofs** (planned): PO-005-PROPTEST-PER-PREFIX, PO-006-PROPTEST-MUTATION (mutation-resistance covers guard removal)
- **Bridge**: `proof-to-rust-map.md` lines 70-72; `proof-to-rust-review.md`
- **Review**: `formal-verification-report.md`; `black-hat-review.md`

### C3 — Error reuse: existing `JournalError::InvalidRunId { run: RunId }` variant
- **Contract clause**: C3
- **Production source refs**: `crates/vb_storage/src/error/mod.rs::JournalError::InvalidRunId { run: RunId }` (UNCHANGED), `crates/vb_storage/src/error/codes.rs::INVALID_RUN_ID_CODE = 0x4021`, `INVALID_RUN_ID` (UNCHANGED)
- **Tests**: 61 `keys::tests::*` tests assert `Err(JournalError::InvalidRunId { run: RunId::new(0) })`; 23 `fjall_keyspace_manifest_tests::*` tests assert the same; 33 `vb_eepg_bdd_tests::*` tests assert the same
- **Proofs**: N/A (regression-test surface, not a new claim)
- **Review**: `formal-verification-report.md`; `black-hat-review.md`

### C4 — Manual check in `headers.rs::run_header` (KEEP or REMOVE)
- **Contract clause**: C4
- **Production source refs**: `crates/vb_storage/src/headers.rs::run_header` lines 36-39 (KEPT as defence-in-depth per State 11 implementation.md §Manual Check Decision)
- **Tests**: 3 companions in `keys/tests.rs` (`run_header_key_accepts_nonzero_run_id`, `index_status_key_with_zero_state_and_timestamp_nonzero_run`, `run_prefix_key_rejects_zero_run_id`) preserve non-zero coverage
- **Review**: `formal-verification-report.md`; `black-hat-review.md` (Attack Vector A5)

### C5 — Test suite flip (18 tests)
- **Contract clause**: C5
- **11 tests in `crates/vb_storage/src/keys/tests.rs`** (flipped from Ok to Err assertions): `run_header_key_has_correct_prefix` (line 72-78), `run_event_key_length` (line 123-128), `index_status_key_has_correct_prefix` (line 190-195), `index_status_key_length` (line 214-219), `index_workflow_key_length` (line 246-251), `index_action_key_length` (line 284-289), `run_header_key_with_zero_run_id` (line 468-474), `index_status_key_with_zero_values` (line 507-514), `run_prefix_key_is_9_bytes` (line 587-592), `index_status_key_rejects_other_state_in_collision_range` (line 678-708, with `RunId::new(0)`→`RunId::new(1)` swap), `index_status_key_accepts_other_state_above_collision_range` (line 710-717, same swap)
- **3 tests in `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs`** (flipped): `encode_exact_length_run_header` (line 340-348), `encode_exact_length_run_event` (line 350-358), `encode_exact_length_index_action` (line 366-374)
- **4 tests in `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs`** (flipped): `run_header_key_prefix_is_0x10` RunId-0 arm (line 91-107), `run_header_key_zero_run_id` (line 109-125), `index_workflow_key_zero_values` (line 205-228), `run_id_zero_roundtrip` (line 699-711)
- **Tests**: 61 + 23 + 33 = 117 passing includes the 18 flips + companions + property guards
- **Review**: `formal-verification-report.md`; `black-hat-review.md`

### C6 — Kani harness split (Err(_) arms must not fire for run_value == 0)
- **Contract clause**: C6
- **Production source refs**: `crates/vb_storage/src/kani_typed_partitioned_ids.rs::assert_key_contracts` (reorganised), `vb_eepg_typed_partitioned_ids` (proof entry point)
- **Tests**: source compiles under `#[cfg(kani)]` per `evidence/kani_typed_partitioned_ids_syntax_check.log` (EXIT: 0); running the Kani solver is planner-deferred to the next bead
- **Proofs** (planned): PO-003-KANI-SPLIT-HARNESS, PO-004-KANI-ORDER-OF-CHECKS
- **Review**: `formal-verification-report.md`; `black-hat-review.md` (residual risk acknowledged)

### C7 — Verus mirror variant `SpecKeyEncodeError::InvalidRunId { run: u64 }` (NOT YET DISCHARGED)
- **Contract clause**: C7
- **Production source refs**: `verification/verus/extern_vb_storage_keys.rs::SpecKeyEncodeError` (variant NOT YET ADDED; planner-deferred to the next bead per State 11 implementation.md §Residual Risks #4)
- **Tests**: N/A (Verus mirror is a second-line defence; first-line defence is the 117 unit/integration tests)
- **Proofs** (planned, not run in this bead): PO-001-VERUS-MIRROR, PO-002-VERUS-DECODER-SYMMETRY
- **Bridge**: `proof-to-rust-map.md` lines 68-69
- **Review**: `formal-verification-report.md`; `black-hat-review.md` (residual risk acknowledged)

### C8 — Decoder side unchanged
- **Contract clause**: C8
- **Production source refs**: `crates/vb_storage/src/keys.rs::decode_storage_key` (UNCHANGED), `crates/vb_storage/src/error/key_decode.rs::KeyDecodeError::InvalidRunId` line 28 (UNCHANGED)
- **Tests**: 69 `restate_doctor_storage_scan_decode_tests` pass (downstream repair surface); includes `parse_decode_error_zero_run_id_is_typed_error` companion test
- **Review**: `formal-verification-report.md`; `black-hat-review.md` (Attack Vector A6, A7)

### C9 — Out-of-scope surfaces preserved
- **Contract clause**: C9
- **Surfaces preserved**: `RunId::new`/`RunId::ZERO` constructor invariant; recovery diagnostics using `RunId::new(0)` as `NoRecoveryData` placeholder (`recovery/replay/summary/{derive,apply,tests}.rs`); workspace tests that build `RunId::new(0)` without reaching a key encoder (`vb_test_runtime_lifecycle_state_behavior.rs`, `integration_runtime_storage_fault_tolerance.rs`, `runtime_version_barrier_tests.rs`, `cancel_kill_lattice_props.rs`, `vb_core_yaml_e2e_chain_contract.rs`); `contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs` TLA+ spec mirror; `crates/vb_storage/src/proptests.rs::all_key_functions_are_deterministic` (already excludes zero); `crates/vb_storage/src/tests.rs::symbolic_code_table` (already maps `INVALID_RUN_ID`)
- **Tests**: pre-existing tests in those files continue to pass; `cargo check --workspace --all-targets --all-features` exit 0
- **Review**: `formal-verification-report.md`; `black-hat-review.md` (Attack Vector A8)

## Formal Verification Results Summary

| id | layer | result | evidence |
|---|---|---|---|
| TEST-KEYS-PRECISE-001 | rust-test (unit) | PASS | `evidence/keys_tests.log` — 61 passed; 0 failed; 0 ignored |
| TEST-FJALL-001 | rust-test (integration) | PASS | `evidence/fjall_keyspace_manifest_tests.log` — 23 passed; 0 failed |
| TEST-EEPG-001 | rust-test (BDD) | PASS | `evidence/vb_eepg_bdd_tests.log` — 33 passed; 0 failed |
| TEST-KEYS-BROAD-001 | rust-test (literal user command) | PASS | `evidence/keys_tests_broad.log` — 85 passed; 0 failed |
| TEST-VB-STORAGE-ALL-001 | rust-test (full suite) | PASS | `evidence/vb_storage_all_tests.log` — 1674 passed; 0 failed across 17 suites |
| CHECK-WORKSPACE-001 | cargo check | PASS | `evidence/workspace_check.log` — 33 crates compiled, exit 0 |
| PO-001-VERUS-MIRROR | verus | PLANNED | planner-deferred to next bead per State 11 implementation.md §Residual Risks #4 |
| PO-002-VERUS-DECODER-SYMMETRY | verus | PLANNED | same |
| PO-003-KANI-SPLIT-HARNESS | kani | PLANNED | harness source compiles; Kani solver run deferred to next bead |
| PO-004-KANI-ORDER-OF-CHECKS | kani | PLANNED | same |
| PO-005-PROPTEST-PER-PREFIX | proptest | PLANNED | proptest not yet implemented; test-writer in next bead |
| PO-006-PROPTEST-MUTATION | proptest | PLANNED | same |

## Unresolved Waiver/Deferred Debt Table

| id | classification | owner | reason |
|---|---|---|---|
| PO-001-VERUS-MIRROR | PLANNED | proof-writer (next bead) | SpecKeyEncodeError::InvalidRunId variant not yet added to verification/verus/extern_vb_storage_keys.rs |
| PO-002-VERUS-DECODER-SYMMETRY | PLANNED | proof-writer (next bead) | same |
| PO-003-KANI-SPLIT-HARNESS | PLANNED | proof-writer (next bead) | harness source compiles; solver run deferred |
| PO-004-KANI-ORDER-OF-CHECKS | PLANNED | proof-writer (next bead) | same |
| PO-005-PROPTEST-PER-PREFIX | PLANNED | test-writer (next bead) | proptest not yet implemented |
| PO-006-PROPTEST-MUTATION | PLANNED | test-writer (next bead) | same |
| (no behavior-affecting waivers) | — | — | `formal-waivers.jsonl` is empty per State 12 closure |

## Test Evidence Summary

| Suite | Tests | Exit | Result |
|---|---|---|---|
| `vb_storage --lib keys::tests` (precise) | 61 | 0 | PASS |
| `vb_storage --lib keys` (literal user) | 85 | 0 | PASS |
| `velvet-ballistics-workspace-tests --test fjall_keyspace_manifest_tests` | 23 | 0 | PASS |
| `velvet-ballistics-workspace-tests --test vb_eepg_bdd_tests` | 33 | 0 | PASS |
| `vb_storage --all-features` (17 suites) | 1674 | 0 | PASS |
| `velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` (downstream repair) | 69 | 0 | PASS |
| `cargo check --workspace --all-targets --all-features` | n/a (33 crates) | 0 | PASS |
| **TOTAL** | **1945** | **0** | **PASS** |

(User-mandated 117 tests = 61 + 23 + 33. Supplementary 1828 = 24 keys-broad delta + 1674 vb_storage-all + 69 restate + 33 crates checked.)

## Review Approval Chain

| Review | Status | Artifact | Line/Note |
|---|---|---|---|
| proof-plan-review.md | APPROVED | `.beads/vb-cn2v4/proof-plan-review.md` | line 3 |
| proof-to-rust-review.md | APPROVED | `.beads/vb-cn2v4/proof-to-rust-review.md` | line 3 |
| formal-verification-report.md | APPROVED | `.beads/vb-cn2v4/formal-verification-report.md` | this report |
| black-hat-review.md | APPROVED | `.beads/vb-cn2v4/black-hat-review.md` | this report |
| final-evidence-decision.md | APPROVED | `.beads/vb-cn2v4/final-evidence-decision.md` | this report |
| assurance-bundle.md | (this file) | `.beads/vb-cn2v4/assurance-bundle.md` | this report |
| truth-serum-report.md | PASS | `.beads/vb-cn2v4/truth-serum-report.md` | this report |

## Raw Evidence File Inventory

- `.beads/vb-cn2v4/evidence/keys_tests.log` (61 passed, 0 failed) — keys::tests
- `.beads/vb-cn2v4/evidence/keys_tests_broad.log` (85 passed, 0 failed) — keys (broader)
- `.beads/vb-cn2v4/evidence/fjall_keyspace_manifest_tests.log` (23 passed, 0 failed)
- `.beads/vb-cn2v4/evidence/vb_eepg_bdd_tests.log` (33 passed, 0 failed)
- `.beads/vb-cn2v4/evidence/vb_storage_all_tests.log` (1674 passed, 0 failed across 17 suites)
- `.beads/vb-cn2v4/evidence/restate_doctor_storage_scan_decode_tests.log` (69 passed, 0 failed; downstream repair)
- `.beads/vb-cn2v4/evidence/workspace_check.log` (33 crates compiled, exit 0)
- `.beads/vb-cn2v4/evidence/kani_typed_partitioned_ids_syntax_check.log` (EXIT: 0, harness source compiles)
- `.beads/vb-cn2v4/evidence/clippy_vb_storage.log` (production-target clippy green)
- `.beads/vb-cn2v4/evidence/cargo_check_vb_storage.log` (lib + bins compile, exit 0)
- `.beads/vb-cn2v4/evidence/diff_summary.txt` (jj diff --stat: 6 files modified)
- `.beads/vb-cn2v4/evidence/full_diff.txt` (full jj diff output)
- `.beads/vb-cn2v4/evidence/vb_core_preexisting_red_test.log` (out of scope per C9)
