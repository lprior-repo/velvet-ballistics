# Assurance Bundle

**bead_id:** vb-core-proof-gate-inputs
**source_checkout:** /home/lewis/src/velvet-ballistics
**isolated_workspace:** /tmp/vb-ws/vb-core-proof-gate-inputs
**commit_or_change:** HEAD

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| VB-CORE-STATE-001 | Step-state machine invariants | V-PF-001 (4 proofs) + V-G1-001 (4 proofs) | proof-review.md: CONDITIONAL PASS | COVERED |
| VB-CORE-STATE-002 | Budget boundedness | V-G1-002 (7 proofs) | proof-review.md: CONDITIONAL PASS | COVERED |
| VB-CORE-STATE-003 | Taint lattice | V-POL-001 (7 proofs) | proof-review.md: CONDITIONAL PASS | COVERED |
| VB-CORE-GATE-001 | Checksum validation | V-G2-001 (5 proofs) + TEST-POL-001/002/003 | proof-review.md: CONDITIONAL PASS | COVERED |
| VB-CORE-GATE-002 | Warning validity | V-PF-002 (12 proofs) + TEST-WARN-001 | proof-review.md: PASS | COVERED |
| VB-CORE-POL-001 | Policy dispatch | V-POL-001 (7 proofs) + TEST-BDD-001 | proof-review.md: CONDITIONAL PASS | COVERED |
| VB-STORAGE-ADMIT-001 | Artifact admission | 39 Verus proofs + 2445 tests | black-hat-review.md: APPROVED | COVERED |
| VB-STORAGE-DUR-001 | Durability gates | TEST-POL-001/002/003 | test-suite-review.md: APPROVED | COVERED |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| V-PF-001 | verus | `cargo verus -- verify verification/verus/vb_core_verification_proof_new.rs` | 4 proofs verified | PASS | No |
| V-PF-002 | verus | `cargo verus -- verify verification/verus/vb_core_verification_warning_is_valid.rs` | 12 proofs verified | PASS | No |
| V-G1-001 | verus | `cargo verus -- verify verification/verus/vb_core_try_from_parts.rs` | 4 proofs verified | PASS | No |
| V-G1-002 | verus | `cargo verus -- verify verification/verus/vb_core_validate_budget.rs` | 7 proofs verified | PASS | No |
| V-G2-001 | verus | `cargo verus -- verify verification/verus/vb_core_checksum_validation.rs` | 5 proofs verified | PASS | No |
| V-POL-001 | verus | `cargo verus -- verify verification/verus/vb_core_policy_dispatch.rs` | 7 proofs verified | PASS | No |
| K-G2-001 | kani | `cargo kani --workspace` | Workspace compilation error | BLOCKED | DEFERRED_GLOBAL (pre-existing blake3 issue) |
| K-G1-001 | kani | `cargo kani` | Times out | DEFERRED_GLOBAL | Optional (required:false) |
| MIRI-001 | miri | `cargo miri` | Times out | DEFERRED_GLOBAL | Optional (required:false) |
| PROP-G1-001 | proptest | `cargo test -p vb_core` | 25 cases passed | PASS | No |

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| TEST-POL-001 | `cargo test -p vb_storage submit_artifact_relaxed` | 3 passed | PASS |
| TEST-POL-002 | `cargo test -p vb_storage submit_artifact_journaled` | 3 passed | PASS |
| TEST-POL-003 | `cargo test -p vb_storage submit_artifact_strict` | 3 passed | PASS |
| TEST-WARN-001 | `cargo test -p vb_storage warning` | 11 passed | PASS |
| TEST-BDD-001 | `cargo test -p vb_storage bdd_` | 3 passed | PASS |
| Full test suite | `cargo test -p vb_core -p vb_storage` | 2445 passed | PASS |
| Clippy | `cargo clippy -p vb_core -p vb_storage --lib --bins --all-features` | No issues | PASS |

---

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Proof Review | proof-review.md | CONDITIONAL PASS (8 PASS, 5 CONDITIONAL, 3 FAIL in optional lanes) | Kani stubs repaired; proptest helpers resolved |
| Contract Verification | contract-verification-review.md | APPROVED | Contract adequately specifies proof obligations |
| Test Plan Review | test-suite-review.md | APPROVED | 16/16 obligations have coverage; 2445 tests pass |
| Test Suite Review | test-suite-review.md | APPROVED | BDD-style policy tests; Kani harnesses substantive |
| Black Hat Review | black-hat-review.md | APPROVED | 39 Verus proofs provide rigorous coverage; K-G2-001 is pre-existing workspace debt |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| K-G2-001 | Pre-existing workspace configuration issue (blake3 dependency in velvet_ballastics CLI crate) | velot_ballastics workspace | Must be resolved by workspace maintainers | 39 Verus proofs cover vb_core/vb_storage scope; K-G2-001 Kani harness blocked at workspace level, not bead level |
| K-G1-001 | Optional (required:false); cargo kani times out | N/A | N/A | Behavioral coverage via unit tests |
| MIRI-001 | Optional (required:false); cargo miri times out; admission.rs has #![forbid(unsafe_code)] | N/A | N/A | No unsafe code in scope |
| WAIVER-FLAG-DERIV | Flag derivation waiver for bounded/taint_safe/retry_safe/replayable/idempotency_keyed/idempotency_attested | vb-core-proof-gate-inputs | Valid | Compensating evidence: gate_count/durable are primary signals |

---

## Truth Serum Audit

- report: `.beads/vb-core-proof-gate-inputs/truth-serum-report.md`
- status: APPROVED (see final-evidence-decision.md)

---

*Assurance bundle generated for vb-core-proof-gate-inputs State 13 evidence packaging*
