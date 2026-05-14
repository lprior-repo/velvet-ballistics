# Assurance Bundle — vb-qi37.5.3

**Bead**: vb-qi37.5.3 — runtime: Carry idempotency evidence into admission
**Type**: TEST COVERAGE BEAD — no production changes
**Generated**: 2026-05-14
**Status**: APPROVED (black-hat-reviewer re-review)

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|-------------|-----------------|---------------------|----------------|--------|
| PRE-01: ArtifactEnvelope validation | cargo-test | 1074 tests pass | test-plan-review.md: APPROVED | PASS |
| PRE-02: StorageArtifactStore::load_accepted_artifact | cargo-test | 1074 tests pass | test-plan-review.md: APPROVED | PASS |
| PRE-03: Non-null idempotency fields | verus+proptest | DEFERRED_GLOBAL | formal-verification-report.md: APPROVED | DEFERRED_GLOBAL (pre-existing vb_runtime build failure) |
| POST-01: RunAdmission idempotency fields copied | verus+proptest | DEFERRED_GLOBAL | formal-verification-report.md: APPROVED | DEFERRED_GLOBAL (pre-existing) |
| POST-02: Box<[ActionId]> type match | verus | DEFERRED_GLOBAL | formal-verification-report.md: APPROVED | DEFERRED_GLOBAL (pre-existing) |
| POST-03: Existing RunAdmission fields unchanged | cargo-test | 1074 tests pass | test-plan-review.md: APPROVED | PASS |
| POST-04: Caller sites provide idempotency evidence | cargo-test | DEFERRED_GLOBAL | formal-verification-report.md: APPROVED | DEFERRED_GLOBAL (pre-existing) |
| POST-05: IdempotencyTracker tracking | cargo-test+miri | DEFERRED_GLOBAL | formal-verification-report.md: APPROVED | DEFERRED_GLOBAL (pre-existing) |
| POST-06: No new panics in admission path | miri | DEFERRED_GLOBAL | formal-verification-report.md: APPROVED | DEFERRED_GLOBAL (pre-existing) |
| INV-01: Field-length equality (keyed) | verus | TYPE-CHECK-PASS (DEFERRED_GLOBAL) | proof-evidence.md: FIXED | TYPE-CHECK-PASS |
| INV-02: Field-length equality (attested) | verus | TYPE-CHECK-PASS (DEFERRED_GLOBAL) | proof-evidence.md: FIXED | TYPE-CHECK-PASS |
| INV-03: IdempotencyTracker capacity bound | verus+proptest | TYPE-CHECK-PASS (DEFERRED_GLOBAL) | proof-evidence.md: FIXED | TYPE-CHECK-PASS |
| INV-04: IdempotencyTracker Send+Sync | miri+loom | DEFERRED_GLOBAL | formal-verification-report.md: APPROVED | DEFERRED_GLOBAL (pre-existing) |
| INV-05: Flag-gated deterministic replay | kani | KANI-PASS (vb_storage only) | proof-evidence.md: FIXED | PARTIAL (KANI-INV-05 PASS for vb_storage; vb_runtime blocked) |
| ERR-01: Error taxonomy coverage | cargo-test | 1074 tests pass | test-plan-review.md: APPROVED | PASS |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|-----------|------|---------|----------|--------|--------|
| VERUS-POST-01 | verus | verus crates/vb_runtime/src/admission.rs | verification/verus/vb_runtime_admission_proofs.rs | TYPE-CHECK-PASS | DEFERRED_GLOBAL (pre-existing chunk_001.rs missing) |
| VERUS-POST-02 | verus | verus crates/vb_runtime/src/admission.rs | verification/verus/vb_runtime_admission_proofs.rs | TYPE-CHECK-PASS | DEFERRED_GLOBAL (pre-existing) |
| VERUS-INV-01 | verus | verus crates/vb_runtime/src/admission.rs | verification/verus/vb_runtime_admission_proofs.rs | TYPE-CHECK-PASS | DEFERRED_GLOBAL (pre-existing) |
| VERUS-INV-02 | verus | verus crates/vb_runtime/src/admission.rs | verification/verus/vb_runtime_admission_proofs.rs | TYPE-CHECK-PASS | DEFERRED_GLOBAL (pre-existing) |
| VERUS-INV-03 | verus | verus crates/vb_runtime/src/idempotency.rs | verification/verus/vb_runtime_idempotency_proofs.rs | TYPE-CHECK-PASS | DEFERRED_GLOBAL (pre-existing) |
| KANI-INV-05 | kani | cargo kani --harness verification_proof_flags_harness --workspace crates/vb_storage | verification/kani/kani_verification_proof_flags.rs | KANI-PASS | None (runs against vb_storage which compiles) |
| KANI-POST-05 | kani | cargo kani --harness load_accepted_artifact_harness --workspace crates/vb_runtime | verification/kani/load_accepted_artifact_harness.rs | DEFERRED_GLOBAL | DEFERRED_GLOBAL (pre-existing vb_runtime build failure) |
| MIRI-INV-04 | miri | MIRIFLAGS cargo miri test -p vb_runtime idempotency | N/A | DEFERRED_GLOBAL | DEFERRED_GLOBAL (pre-existing) |
| MIRI-POST-06 | miri | MIRIFLAGS cargo miri test -p vb_runtime run_admission | N/A | DEFERRED_GLOBAL | DEFERRED_GLOBAL (pre-existing) |
| LOOM-INV-04 | loom | cargo loom test -p vb_runtime idempotency --persist | N/A | DEFERRED_GLOBAL | DEFERRED_GLOBAL (pre-existing) |
| PROPTEST-POST-01 | proptest | cargo test -p vb_runtime run_admission_idempotency_proptest | N/A | DEFERRED_GLOBAL | DEFERRED_GLOBAL (pre-existing) |
| PROPTEST-INV-03 | proptest | cargo test -p vb_runtime idempotency_tracker_capacity_proptest | N/A | DEFERRED_GLOBAL | DEFERRED_GLOBAL (pre-existing) |

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|-----------|---------|----------|--------|
| TEST-POST-03 | cargo test -p vb_storage admit_run | tests/proptests.rs | PASS (1074 tests total) |
| TEST-POST-04 | cargo test -p vb_storage admission | tests/proptests.rs | PASS (1074 tests total) |
| TEST-ERR-01 | cargo test -p vb_storage artifact_envelope_error | tests/proptests.rs | PASS (1074 tests total) |
| TEST-INV-05 | cargo test -p vb_storage verification_proof_flags | tests/proptests.rs | PASS (1074 tests total) |
| TEST-POST-05 | cargo test -p vb_storage idempotency | tests/proptests.rs | PASS (1074 tests total) |
| LINT-01 | cargo clippy -p vb_storage --all-features -- -D warnings | N/A | PASS (0 warnings) |
| FMT-01 | cargo fmt --check | N/A | PASS (no diffs) |
| BUILD-01 | cargo build -p vb_storage | N/A | PASS (builds cleanly) |

---

## Review Evidence

| Review | Artifact | Status | Findings |
|--------|----------|--------|----------|
| contract-verification-review | contract-verification-review.md | APPROVED | Contract clauses properly mapped |
| proof-plan-review | proof-plan-review-input.md | APPROVED | 18 obligations planned |
| proof-review | proof-review.md | APPROVED | Verus/kani scope properly categorized |
| test-plan-review | test-plan-review.md | APPROVED | 84 tests / 4 pub fns = 21x trophy allocation |
| formal-verification-report | formal-verification-report.md | APPROVED | All vb_storage gates pass; DEFERRED_GLOBAL documented |
| black-hat-review (State 12 re-review) | black-hat-review.md | APPROVED | All 3 LETHAL defects fixed |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|------|--------|-------|------------------|----------------------|
| DEFERRED-GLOBAL-01: vb_runtime missing chunk_001.rs | Pre-existing at commit ffbe7f5cd; chunk_001.rs file missing from workspace | external | When chunk_001.rs is restored or include directive removed | vb_storage gates all pass (1074 tests, 0 clippy, fmt compliant) |
| VERUS-POST-01/02, INV-01/02 | vb_runtime cannot compile; verus runs on standalone proof files only (TYPE-CHECK-PASS) | external | When vb_runtime build restored | TYPE-CHECK-PASS on standalone files; vb_storage verified |
| VERUS-INV-03 | vb_runtime cannot compile; TYPE-CHECK-PASS on standalone files only | external | When vb_runtime build restored | TYPE-CHECK-PASS on standalone files; vb_storage verified |
| KANI-POST-05 | vb_runtime cannot compile | external | When vb_runtime build restored | KANI-INV-05 PASS on vb_storage |
| MIRI-INV-04, MIRI-POST-06 | vb_runtime cannot compile | external | When vb_runtime build restored | vb_storage verified via other gates |
| LOOM-INV-04 | vb_runtime cannot compile | external | When vb_runtime build restored | vb_storage verified via other gates |
| PROPTEST-POST-01, PROPTEST-INV-03 | vb_runtime cannot compile | external | When vb_runtime build restored | vb_storage verified via other gates |

---

## Truth Serum Audit

- report: `.beads/vb-qi37.5.3/truth-serum-report.md`
- status: APPROVED (see truth-serum-report.md for full evidence)

---

## Verdict

**STATUS: APPROVED**

All vb_storage gates pass (1074 tests, 0 clippy warnings, fmt compliant, builds cleanly). This is a test coverage improvement bead with no production changes.

All vb_runtime formal verification obligations are correctly documented as DEFERRED_GLOBAL due to pre-existing missing chunk_001.rs (workspace debt at commit ffbe7f5cd). The TYPE-CHECK-PASS on standalone verus proof files is accurately documented and does not falsely claim verification of actual vb_runtime code.

Black-hat-reviewer re-review (State 12) APPROVED with no blocking defects.
