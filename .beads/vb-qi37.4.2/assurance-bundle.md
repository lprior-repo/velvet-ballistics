# Assurance Bundle

**bead_id**: vb-qi37.4.2
**bead_title**: runtime: Enforce admission gate before run creation
**source_checkout**: /home/lewis/src/velvet-ballistics
**isolated_workspace**: /tmp/vb-ws/vb-qi37.4.2
**phase**: 13 (Evidence Packaging)
**updated_at**: 2026-05-15T00:00:00Z
**attempt**: 1

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| INV-001: Never insert run unless build_admission returns Ok | Contract.md INV-001 | INT-INV-001 (strict), INT-INV-002 (journaled) — PASS | black-hat-review.md PHASE 1, contract-verification-review.md | APPROVED |
| INV-002: Sequencing — admission → frame → journal → insert | Contract.md INV-002 | WAIVER-TLA-001 (single atomic step, no temporal behavior) — WAIVED | black-hat-review.md PHASE 1 | WAIVED |
| POST-002: On rejection, no frame allocated, no journal, no insert, counter unchanged | Contract.md POST-002 | INT-POST-001 — PASS | black-hat-review.md PHASE 1 | APPROVED |
| ERR-Rejection (ArtifactNotFound): Strict/Journaled reject with ArtifactNotFound | Contract.md ERR taxonomy | INT-INV-001, INT-INV-002 — PASS | black-hat-review.md PHASE 1 | APPROVED |
| ERR-Rejection (CapabilityDenied): Capability mismatch returns CapabilityDenied | Contract.md ERR taxonomy | INT-ERR-001 — PASS | black-hat-review.md PHASE 1 | APPROVED |
| PRE-003: Duplicate run returns RunAlreadyExists | Contract.md PRE-003 | N/A — pre-existing test | contract-verification-review.md | PRE-EXISTING |
| PRE-004: At capacity returns ActiveRunCapacityExceeded | Contract.md PRE-004 | N/A — pre-existing test | contract-verification-review.md | PRE-EXISTING |
| Compile and Lint | N/A | COMPILE-001, LINT-001 — PASS | machine-gate-report.md | APPROVED |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| COMPILE-001 | cargo build | `cargo build -p vb_runtime` | admission.rs | PASS | None |
| LINT-001 | cargo clippy | `cargo clippy -p vb_runtime --lib --bins -- -D warnings` | admission.rs | PASS | None |
| INT-INV-001 | cargo test | `cargo test -p vb_runtime admission_strict_policy_rejects_missing_artifact_run_not_inserted` | chunk_003.rs:247+ | PASS | None |
| INT-INV-002 | cargo test | `cargo test -p vb_runtime admission_journaled_policy_rejects_missing_artifact_run_not_inserted` | chunk_003.rs:247+ | PASS | None |
| INT-ERR-001 | cargo test | `cargo test -p vb_runtime admission_capability_mismatch_error_exists` | chunk_003.rs:247+ | PASS | None |
| INT-POST-001 | cargo test | `cargo test -p vb_runtime admission_rejection_no_counter_increment_strict` | chunk_003.rs:247+ | PASS | None |
| UNIT-ADMIT-001 | cargo test (waived) | N/A | admission.rs unit test | WAIVED | Integration tests (INT-INV-001) provide equivalent shard-level coverage |
| UNIT-ADMIT-002 | cargo test (waived) | N/A | admission.rs unit test | WAIVED | Integration tests (INT-INV-002) provide equivalent shard-level coverage |
| WAIVER-TLA-001 | N/A | N/A | tla-spec.md | WAIVED | Single atomic step; no temporal behavior |
| WAIVER-VERUS-001 | N/A | N/A | lean-contract.md | WAIVED | Deterministic Rust ? propagation verified by integration test |
| MRI-001 | N/A | N/A | N/A | DEFERRED_GLOBAL | Miri tooling unavailable (missing rust-src); pre-existing tooling gap |

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| Build | `cargo build -p vb_runtime` | admission.rs | PASS |
| Clippy | `cargo clippy -p vb_runtime --lib --bins -- -D warnings` | admission.rs | PASS |
| INT-INV-001 | `cargo test -p vb_runtime admission_strict_policy_rejects_missing_artifact_run_not_inserted` | chunk_003.rs | PASS (1 passed) |
| INT-INV-002 | `cargo test -p vb_runtime admission_journaled_policy_rejects_missing_artifact_run_not_inserted` | chunk_003.rs | PASS (1 passed) |
| INT-ERR-001 | `cargo test -p vb_runtime admission_capability_mismatch_error_exists` | chunk_003.rs | PASS (1 passed) |
| INT-POST-001 | `cargo test -p vb_runtime admission_rejection_no_counter_increment_strict` | chunk_003.rs | PASS (1 passed) |
| Full suite | `cargo test -p vb_runtime` | all | 1270 passed; 85 pre-existing failures (DEFERRED_GLOBAL) |

---

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Proof Review | proof-review.md | REJECTED (attempt 1) → ACCEPTED (attempt 2 with repair) | proof-obligations.planned.jsonl updated |
| Contract Verification Review | contract-verification-review.md | APPROVED | Contract obligations adequate |
| Test Plan Review | test-plan-review.md | APPROVED | Test plan maps requirements to test cases |
| Test Suite Review | test-suite-review.md | APPROVED | Test suite covers all contract clauses |
| Formal Verification Report | formal-verification-report.md | APPROVED | All obligations PASS/WAIVED/DEFERRED_GLOBAL |
| Black-Hat Review | black-hat-review.md | APPROVED | NeverPresentArtifactStore correct; no real risks remain |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| UNIT-ADMIT-001, UNIT-ADMIT-002 | Integration tests provide equivalent shard-level coverage | vb-qi37.4.2 | N/A | INT-INV-001, INT-INV-002 pass |
| WAIVER-TLA-001 | INV-002 is single atomic step; no temporal behavior | vb-qi37.4.2 | N/A | Sequencing verified by integration test INT-INV-001 |
| WAIVER-VERUS-001 | INV-001 is deterministic Rust ? propagation | vb-qi37.4.2 | N/A | Verified by integration test INT-INV-001 |
| MRI-001 (DEFERRED_GLOBAL) | Miri tooling unavailable (missing rust-src component) | Tooling gap | Pre-existing environment issue | N/A — tooling gap, not code defect |
| 85 pre-existing test failures | Unrelated to this bead (do_action_completion_*, runtime_cancel_*, runtime_fail_action_*, etc.) | DEFERRED_GLOBAL | Pre-existing baseline failures | Not caused by NeverPresentArtifactStore |

---

## Truth Serum Audit

- report: `.beads/vb-qi37.4.2/truth-serum-report.md`
- status: PENDING (truth-serum runs in active execution context)
