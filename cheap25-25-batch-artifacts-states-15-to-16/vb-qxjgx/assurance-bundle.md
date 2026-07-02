# Assurance Bundle

bead_id: vb-qxjgx
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx
commit_or_change: ttulypyv 376c7ccc (state 11 implementation); 1b72c500 (state 5 proof-writer)

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| RecordKind::StepSucceeded = 33 + closed-set bijection | POST-001, PRE-001, INV-002 | back-compat test #1 `step_succeeded_event_maps_to_step_succeeded_kind` (codec/tests.rs:1630) PASS; PO-QXJGX-001 kani H1/H2/H3 (BLOCKED_TOOLING, TBR-001, compensated) | proof-review.md STATUS: APPROVED; proof-to-rust-review.md STATUS: APPROVED; black-hat-review.md STATUS: APPROVED | covered |
| JournalEvent::record_kind one-to-one projection (OR-collapse removed) | POST-002, PRE-002, INV-001 | back-compat test #1 + test #3 `step_succeeded_and_slot_written_record_kinds_are_distinct` (codec/tests.rs:1672) PASS; PO-QXJGX-002 kani H1/H2/H3 (BLOCKED_TOOLING, compensated); PO-QXJGX-006-H3 proptest PASS | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| is_known_record_kind(33) = true | POST-003, PRE-003, INV-003 | PO-QXJGX-003 kani H1, H5, H6 (BLOCKED_TOOLING, compensated); proptest PO-QXJGX-007-H4 PASS | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| validate_kind_family admit/reject grid | POST-004, INV-003 | PO-QXJGX-003 kani H2/H3/H4/H5/H6 (BLOCKED_TOOLING, compensated); proptest PO-QXJGX-007-H4 `kind_33_journal_family_admit_reject_grid` PASS | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| Parity {12, 33} for StepSucceeded; {12} for SlotWrittenEvent; reject 33 for SlotWrittenEvent | POST-005, POST-007, INV-004, ERR-006 | back-compat test #4 `legacy_envelope_id_12_with_step_succeeded_payload_is_accepted` (codec/tests.rs:1702) PASS; test #6 `slot_written_with_envelope_id_33_is_rejected` (codec/tests.rs:1765) PASS; PO-QXJGX-004 kani H1..H7 (BLOCKED_TOOLING, compensated) | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| decode_journal_event round-trip canonical id-33 + legacy id-12 | POST-006, POST-013, INV-005 | back-compat test #5 `canonical_id_33_round_trip_step_succeeded` (codec/tests.rs:1734) PASS; test #4 (legacy id-12) PASS; PO-QXJGX-005 kani H1/H2/H3 (BLOCKED_TOOLING, compensated) | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| decode_journal_event envelope/payload sequence identity | POST-013 | mod.rs:143-148 typed ReplayEnvelopeSequenceMismatch check; back-compat test #5 round-trip covers the happy path; PO-QXJGX-005-H3 kani (BLOCKED_TOOLING, compensated) | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| Durability matrix step-closing rows use StepSucceeded | POST-008, PRE-007 | proptest PO-QXJGX-007-H1 `durability_matrix_step_closing_rows_use_step_succeeded` PASS at 10000 cases; 10 row substitutions verified at durability_matrix.rs:75,89,100,110,120,132-133,146-147,158,171,186-187 | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| Recovery summary counters variant-keyed | POST-009, INV-008 | proptest PO-QXJGX-006 (4 properties) PASS at 10000 cases; H4 `id_keyed_counter_would_diverge_from_variant_keyed` is the E_KANI_ASSUMPTION_VACUITY closure | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| flux_validation literal-sync id 33 | POST-011 | proptest PO-QXJGX-007-H3 `flux_validation_literal_includes_33` PASS — parses codec/flux_validation.rs:14,33 and asserts id 33 in known set | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| CURRENT_SCHEMA_VERSION = 1 (UNCHANGED) | PRE-005, INV-006 | constants.rs:58 reads `pub const CURRENT_SCHEMA_VERSION: u16 = 1;`; proptest PO-QXJGX-007-H2 `schema_version_is_pinned_at_one` PASS; in-crate tests at tests.rs:3925, 4223 | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |
| Back-compat legacy envelope-12 tolerance (NOT a schema bump) | POST-005, PRE-005 | back-compat test #4 `legacy_envelope_id_12_with_step_succeeded_payload_is_accepted` PASS; LegacyEnvelopeBinding::Legacy { accepted_ids: &[12, 33] } for StepSucceeded (kind_parity.rs:45-66) | proof-review.md APPROVED; black-hat-review.md APPROVED | covered |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-QXJGX-001 | kani | `cargo kani -j 1 --output-format=regular --harness check_step_succeeded_kind_id --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` | `crates/vb_storage/src/kani_record_kind_id_step_succeeded.rs` | BLOCKED_TOOLING (TBR-001; pre-existing `vb_core` kani_helpers.rs) | n/a (compensated by back-compat test #1) |
| PO-QXJGX-002 | kani | `cargo kani -j 1 --output-format=regular --harness check_step_succeeded_record_kind_projection --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` | `crates/vb_storage/src/kani_record_kind_projection_split.rs` | BLOCKED_TOOLING (TBR-001) | n/a (compensated by back-compat tests #1, #3) |
| PO-QXJGX-003 | kani | `cargo kani -j 1 --output-format=regular --harness check_kind_33_journal_family_full --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` | `crates/vb_storage/src/kani_record_kind_journal_family_33.rs` | BLOCKED_TOOLING (TBR-001) | n/a (compensated by proptest PO-QXJGX-007-H4) |
| PO-QXJGX-004 | kani | `cargo kani -j 1 --output-format=regular --harness check_parity_gate_step_succeeded_legacy --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` | `crates/vb_storage/src/kani_record_kind_parity_legacy_envelope.rs` | BLOCKED_TOOLING (TBR-001) | n/a (compensated by back-compat tests #4, #6) |
| PO-QXJGX-005 | kani | `cargo kani -j 1 --output-format=regular --harness check_decode_round_trip_step_succeeded --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` | `crates/vb_storage/src/kani_record_kind_decode_round_trip.rs` | BLOCKED_TOOLING (TBR-001) | n/a (compensated by back-compat test #5) |
| PO-QXJGX-006 | proptest | `PROPTEST_CASES=10000 cargo test --test proptest_replay_summary_step_succeeded_split --release -p vb_storage` | `crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs` | **PASS** (4/4) | n/a |
| PO-QXJGX-007 | proptest | `PROPTEST_CASES=10000 cargo test --test proptest_durability_matrix_step_succeeded --release -p vb_runtime` | `crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs` | **PASS** (5/5) | n/a |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| `cargo test -p vb_storage --tests` | `cargo test -p vb_storage --tests` | `.beads/vb-qxjgx/evidence/fv-cargo-test-vb_storage.txt` | PASS (1678 passed, 17 suites, 13.13s) |
| `cargo test -p vb_runtime --tests` | `cargo test -p vb_runtime --tests` | `.beads/vb-qxjgx/evidence/fv-cargo-test-vb_runtime.txt` | PASS (2348 passed, 1 ignored, 35 suites, 3.34s) |
| Back-compat 6 tests (codec/tests.rs:1630-1791) | `cargo test -p vb_storage --tests -- step_succeeded_event_maps_to_step_succeeded_kind slot_written_event_maps_to_slot_written_kind_unchanged step_succeeded_and_slot_written_record_kinds_are_distinct legacy_envelope_id_12_with_step_succeeded_payload_is_accepted canonical_id_33_round_trip_step_succeeded slot_written_with_envelope_id_33_is_rejected` | `.beads/vb-qxjgx/evidence/fv-backcompat-6-tests.txt` | PASS (6 passed, 1672 filtered out) |
| Proptest PO-QXJGX-006 | `PROPTEST_CASES=10000 cargo test --test proptest_replay_summary_step_succeeded_split --release -p vb_storage` | `.beads/vb-qxjgx/evidence/fv-proptest-replay-split.txt` | PASS (4 passed, 1 suite, 0.04s) |
| Proptest PO-QXJGX-007 | `PROPTEST_CASES=10000 cargo test --test proptest_durability_matrix_step_succeeded --release -p vb_runtime` | `.beads/vb-qxjgx/evidence/fv-proptest-durability.txt` | PASS (5 passed, 1 suite, 0.02s) |
| `cargo check -p vb_storage --all-targets` | `cargo check -p vb_storage --all-targets` | (terminal output) | PASS (Finished `dev` profile, 4.98s) |
| `cargo check -p vb_runtime --all-targets` | `cargo check -p vb_runtime --all-targets` | (terminal output) | PASS (Finished `dev` profile, 2.84s) |
| `cargo clippy -p vb_storage --lib` | `cargo clippy -p vb_storage --lib` | (terminal output) | PASS (No issues found) |
| `cargo clippy -p vb_runtime --lib` | `cargo clippy -p vb_runtime --lib` | (terminal output) | PASS (No issues found) |
| `cargo fmt --check -p vb_storage` | `cargo fmt --check -p vb_storage` | (terminal output) | PASS (no output) |
| `cargo fmt --check -p vb_runtime` | `cargo fmt --check -p vb_runtime` | `.beads/vb-qxjgx/evidence/mg-cargo-fmt.txt` | DEFERRED_GLOBAL (pre-existing frame_pool/tests.rs:85,114,139; not modified by this bead) |
| `cargo kani` workspace-wide | `KANI_FEATURES=kani-vb-qxjgx-record-kind-split bash scripts/kani-list.sh vb_storage` | `.beads/vb-qxjgx/evidence/fv-kani-list-vb_storage.txt` | BLOCKED_TOOLING (TBR-001; pre-existing `vb_core` kani_helpers.rs unclosed-delimiter) |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| proof-plan-review (state 4) | `.beads/vb-qxjgx/proof-plan-review.md` | APPROVED | 5 findings (all observation/info; 0 blocker; 0 medium+) |
| proof-review (state 6) | `.beads/vb-qxjgx/proof-review.md` | APPROVED | 5 findings (all observation/info; 0 blocker; 0 medium+); 22 kani harnesses + 9 proptest properties + 6 back-compat tests reviewed |
| proof-to-rust-review (state 8) | `.beads/vb-qxjgx/proof-to-rust-review.md` | APPROVED | 7 RRO rows reviewed; all bound to production symbols + behavior test refs + refinement harness refs |
| test-plan-review (state 12 — delegated) | `.beads/vb-qxjgx/test-plan-review.md` | APPROVED | 5 findings (all observation/info; 0 blocker) |
| formal-verification (state 12) | `.beads/vb-qxjgx/formal-verification-report.md` | APPROVED | 2 PASS + 5 BLOCKED_TOOLING (TBR-001); 7/7 obligations dispositioned |
| black-hat-review (state 13) | `.beads/vb-qxjgx/black-hat-review.md` | APPROVED | 6 findings (3 pre-existing owner_approved_debt, 1 fixed_with_evidence, 1 expected, 1 BLOCKED_TOOLING compensated) |
| machine-gate (state 12) | `.beads/vb-qxjgx/machine-gate-report.md` | PASS (bead-local); DEFERRED_GLOBAL | All bead-local gates PASS; 3 pre-existing global debt items |
| regression-diff (state 12) | `.beads/vb-qxjgx/regression-diff.md` | NO BEAD-LOCAL REGRESSIONS | 3 pre-existing global debt items |

## Findings Disposition

| Finding | Severity | Source Review | Disposition | Evidence Or Owner Approval |
|---|---|---|---|---|
| TBR-001: pre-existing kani_helpers.rs unclosed-delimiter | HIGH (pre-existing) | proof-review.md (TBR-001); formal-verification-report.md (BLOCKED_TOOLING); black-hat-review.md (FIND-001) | owner_approved_debt | Verified pre-existing in parent commit ywnswumt 1b72c500; routes to kani-helpers owner; compensation: 1678 + 2348 cargo test PASS + 6 back-compat + 9 proptest |
| TBR-002: 4 forward-looking E0599 errors (RecordKind::StepSucceeded undefined) | MEDIUM (expected) | proof-writer-report.md (PENDING_FORMAL_EXECUTION); formal-verification-report.md (resolved) | fixed_with_evidence | State 11 implementation landed; cargo test PASS post-state-11 |
| TBR-003: kani::assume pre-conditions mirror JournalEvent::is_valid() | LOW (assumed) | proof-review.md (TBR-003) | owner_approved_no_action | Pre-conditions mirror production validation; not property short-circuits; documented in harness headers |
| TBR-004: CURRENT_SCHEMA_VERSION = 1 | LOW (const) | proof-review.md (TBR-004) | fixed_with_evidence | constants.rs:58 verified UNCHANGED; proptest PO-QXJGX-007-H2 PASS |
| TBR-005: validate_schema_version via public surface | LOW (deviation) | proof-review.md (TBR-005) | owner_approved_no_action | In-crate tests cover direct call; proptest exercises public surface |
| TBR-006: proptest path deviation | LOW (deviation) | proof-review.md (TBR-006) | owner_approved_no_action | planned.jsonl paths are authoritative; proptest files at tests/ directories |
| TBR-007: closed-set golden array | MEDIUM (extern_spec) | proof-review.md (TBR-007) | owner_approved_no_action | Array paired with production function calls; drift caught by kani::assert |
| TBR-008: synthesized envelope-12 in kani H2 | MEDIUM (model) | proof-review.md (TBR-008) | owner_approved_no_action | Pattern matches pre-existing kani_record_kind.rs:107-134 |
| TBR-009: anti-invariant token | LOW (non_vacuity) | proof-review.md (TBR-009) | fixed_with_evidence | `invalid_input` literal present in both proptest files (grep-confirmed) |
| TBR-010: pre-fix check_unknown_kind_rejected | MEDIUM (pre-existing) | proof-writer-report.md (NOT_BLOCKING); formal-verification-report.md (resolved) | fixed_with_evidence | DELETED in state 11; replacement at kani_record_kind_journal_family_33.rs:H2 |
| Pre-existing aggregate_resource_budget_properties_red proptest failure | HIGH (pre-existing) | transcript-state11-holzman-rust.txt (residual risks) | owner_approved_debt | Pre-existing global failure; not in scope for this bead |
| Pre-existing vb_runtime/src/frame_pool/tests.rs cargo fmt drift | LOW (pre-existing) | transcript-state11-holzman-rust.txt (residual risks); machine-gate-report.md (DEFERRED_GLOBAL) | owner_approved_debt | Not modified by this bead (verified via `jj diff`); routes to frame_pool owner |

## Waivers And Deferred Work

Waivers and deferred work are not finding dispositions. Findings must use only canonical `finding/v1.disposition` values: `fixed_with_evidence`, `owner_approved_debt`, `owner_approved_no_action`, or `blocker`.

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| TBR-001: kani_helpers.rs unclosed-delimiter | pre-existing global blocker; cargo kani workspace-wide unavailable | kani-helpers owner | when vb_core kani_helpers delimiter fixed | 1678 + 2348 cargo test PASS; 6 back-compat unit tests; 9 proptest properties at PROPTEST_CASES=10000 |
| TBR-002 (resolved): forward-looking E0599 | expected at state 5; resolved at state 11 | holzman-rust (state 11) | resolved | cargo test PASS post-state-11 |
| TBR-010 (resolved): pre-fix check_unknown_kind_rejected | pre-fix harness asserts inverse of post-fix contract | holzman-rust (state 11) | resolved | DELETED; replacement at kani_record_kind_journal_family_33.rs:H2 |
| aggregate_resource_budget_properties_red proptest failure | pre-existing global failure; literal-string check | aggregate_resource_budget owner | when fixed | not in scope for this bead |
| vb_runtime/src/frame_pool/tests.rs cargo fmt drift | pre-existing global drift | frame_pool owner | when fixed | not in scope for this bead |

## Truth Serum Audit

- report: `.beads/vb-qxjgx/truth-serum-report.md`
- status: APPROVED

## Verdict

**STATUS: APPROVED.** All 7 proof obligations dispositioned (2 PASS + 5 BLOCKED_TOOLING compensated). 1678 + 2348 cargo test PASS. 6 back-compat unit tests PASS. 9 proptest properties PASS at PROPTEST_CASES=10000. CURRENT_SCHEMA_VERSION preserved at 1. Back-compat legacy envelope-12 tolerance VERIFIED. No bead-local regressions; 3 pre-existing global debt items (TBR-001, aggregate_resource_budget, frame_pool/tests.rs fmt) with owner_approved_debt disposition. All findings at every severity have canonical dispositions. Black-hat review STATUS: APPROVED. Truth-serum audit STATUS: APPROVED. Ready for landing.
