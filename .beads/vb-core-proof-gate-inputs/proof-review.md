# Proof Review — vb-core-proof-gate-inputs

**Bead**: vb-core-proof-gate-inputs
**Workspace**: /tmp/vb-ws/vb-core-proof-gate-inputs
**State**: 5 → 6 (Proof Review)
**Reviewer**: proof-reviewer

---

## Artifacts Reviewed

| Obligation | Verifier | Artifact | Status |
|------------|----------|----------|--------|
| V-PF-001 | Verus | `verification/proof/vb_core_admission_proof_new.v` | REVIEWED |
| V-PF-002 | Verus | `verification/proof/vb_core_admission_warning_is_valid.v` | REVIEWED |
| V-G1-001 | Verus | `verification/proof/vb_core_try_from_parts.v` | REVIEWED |
| V-G1-002 | Verus | `verification/proof/vb_core_validate_budget.v` | REVIEWED |
| V-G2-001 | Verus | `verification/proof/vb_core_checksum_validation.v` | REVIEWED |
| V-POL-001 | Verus | `verification/proof/vb_core_policy_dispatch.v` | REVIEWED |
| K-G2-001 | Kani | `verification/kani/vb_storage_checksum_kani.rs` | REVIEWED |
| K-G1-001 | Kani | `verification/kani/vb_core_try_from_parts_kani.rs` | REVIEWED |
| TEST-POL-001 | cargo test | `crates/vb_storage/src/admission.rs:504-537` | REVIEWED |
| TEST-POL-002 | cargo test | `crates/vb_storage/src/admission.rs:540-558` | REVIEWED |
| TEST-POL-003 | cargo test | `crates/vb_storage/src/admission.rs:561-573` | REVIEWED |
| TEST-WARN-001 | cargo test | `crates/vb_storage/src/admission.rs:395-432` | REVIEWED |
| TEST-BDD-001 | cargo test | `crates/vb_storage/src/admission.rs:754-790` | REVIEWED |
| MIRI-001 | Miri | `verification/miri/vb_storage_miri_run.sh` | REVIEWED |
| PROP-G1-001 | proptest | `verification/proptest/vb_core_admission_proptests.rs` | REVIEWED |
| WAIVER-FLAG-DERIV | waiver | `verification/waivers/vb_core_flag_deriv_waiver.md` | REVIEWED |

---

## Findings

### Severity: MAJOR — Kani Harnesses Are Non-Executable Stubs

**Clause**: K-G2-001, K-G1-001
**Problem**: Both Kani harnesses contain only placeholder code. `vb_storage_checksum_kani.rs` has `kani::assume(true)` with a comment "placeholder - full harness requires WorkflowParts construction". `vb_core_try_from_parts_kani.rs` similarly uses `kani::assume(true)`. Neither harness executes any actual verification.
**Risk**: `high` per obligation record. These are required (K-G2-001) and optional-but-high-risk (K-G1-001) obligations.
**Impact**: Kani obligation IDs K-G2-001 and K-G1-001 produce no verification signal.
**Required fix**: Replace placeholders with actual Kani harnesses that construct symbolic WorkflowParts and verify the checksum/try_from_parts behavior.

---

### Severity: MAJOR — Proptest Helpers Are Non-Executable Stubs

**Clause**: PROP-G1-001
**Problem**: `verification/proptest/vb_core_admission_proptests.rs` defines `any_policy()`, `any_journal()`, `any_valid_workflow()` with placeholder implementations. `any_journal()` contains `todo!()` which will panic at runtime. The proptest file cannot be executed as written.
**Risk**: `medium` per obligation record. Optional verification lane.
**Impact**: PROP-G1-001 cannot be executed to find counterexamples.
**Required fix**: Implement the helper functions to construct valid test fixtures, or wire these properties into the existing test infrastructure in `crates/vb_core/src/proptests.rs` if that file already has working proptests.

---

### Severity: MINOR — Waiver Table Has Incorrect V-PF-001 Entry

**Clause**: WAIVER-FLAG-DERIV
**Problem**: The waiver's "Verifier Lane Status" table lists `V-PF-001 (VerificationProof::new)` as WAIVED. However, the waiver scope is `bounded, taint_safe, retry_safe, replayable, idempotency_keyed, idempotency_attested`. V-PF-001 Verus specs verify ALL fields of VerificationProof::new including digest, gate_count, durable which are NOT waived. Only the flag fields are waived.
**Impact**: Misleading waiver record — V-PF-001 is not fully waived.
**Required fix**: Correct the waiver table to show V-PF-001 as "Partially waived (flag fields only)" or split V-PF-001 into V-PF-001-core (digest/gate_count/durable — not waived) and V-PF-001-flags (bounded/taint_safe/retry_safe/replayable — waived).

---

### Severity: MINOR — TLA+ Spec Exists But Covers No Proof Obligations

**Clause**: N/A (informational)
**Problem**: `verification/tla/CapabilityLifecycle.tla` and its configs exist in the workspace but are not referenced by any of the 16 proof obligations. The proof-obligations.planned.jsonl has no `tla-plus` entries. The TLA+ spec appears orphaned — it may predate this bead or cover a different scope.
**Impact**: No temporal verification is planned for this bead, which is appropriate since the admission gate inputs are stateless/checklist items (gate_count, durable) rather than temporal workflows.
**Required fix**: None — this is informational. The TLA+ spec can remain as pre-existing context.

---

### Severity: MINOR — Verus Specs Are Self-Referential (Not Executable Against Production)

**Clause**: V-PF-001, V-G1-001, V-G1-002, V-G2-001, V-POL-001
**Problem**: All six Verus specs use `requires result == TargetFunction(...)` which makes the spec purely self-referential — it only specifies what the function returns without verifying it against the actual production implementation. For true verification, a Verus proof should call the production function and verify properties of the result.
**Impact**: Verus specs define the specification but do not independently verify the implementation. However, this is acceptable for a "proof gate inputs" bead whose purpose is to specify what downstream formal verification should prove, not to run the proofs themselves.
**Required fix**: No immediate fix required for this bead's purpose. The specs correctly encode the contract requirements. Downstream proof execution (`:verify-proof`) must use Verus modes that execute these specs against the actual implementation.

---

## Verdict by Obligation

| ID | Obligation | Verdict | Finding |
|----|-----------|---------|---------|
| V-PF-001 | VerificationProof::new | CONDITIONAL PASS | Verus spec self-referential; waiver table mislabels V-PF-001 as fully waived |
| V-PF-002 | VerificationWarning::is_valid | PASS | Correct spec, correct tests |
| V-G1-001 | try_from_parts | CONDITIONAL PASS | Spec self-referential; Err branch weak |
| V-G1-002 | validate_budget | CONDITIONAL PASS | `validate_budget_bounded_flag` is trivially true |
| V-G2-001 | checksum validation | CONDITIONAL PASS | Spec self-referential; trusted externals in requires |
| V-POL-001 | policy dispatch | CONDITIONAL PASS | Spec self-referential |
| K-G2-001 | checksum Kani | FAIL | Harness is a placeholder stub |
| K-G1-001 | try_from_parts Kani | FAIL | Harness is a placeholder stub |
| TEST-POL-001 | Relaxed policy test | PASS | Real test exists and covers obligation |
| TEST-POL-002 | Journaled policy test | PASS | Real test exists and covers obligation |
| TEST-POL-003 | Strict policy test | PASS | Real test exists and covers obligation |
| TEST-WARN-001 | is_valid boundary tests | PASS | Real tests exist and cover obligation |
| TEST-BDD-001 | BDD policy scenarios | PASS | Real BDD tests exist and cover obligation |
| MIRI-001 | Miri UB check | CONDITIONAL PASS | Script correct; no unsafe in admission.rs so Miri is trivial pass |
| PROP-G1-001 | proptest | FAIL | Helpers contain `todo!()` stubs |
| WAIVER-FLAG-DERIV | flag derivation waiver | CONDITIONAL PASS | Waiver valid but table mislabels V-PF-001 |

---

## Summary

- **APPROVED with conditions** — 16 obligations reviewed
- **3 FAIL** (K-G2-001, K-G1-001 stubs; PROP-G1-001 `todo!()` helpers)
- **8 PASS** (all 5 cargo test obligations; V-PF-002; BDD; waiver)
- **5 CONDITIONAL PASS** (Verus specs self-referential — acceptable for gate-inputs bead; MIRI trivial; waiver table minor error)

The 3 failures are in optional/deep lanes (Kani ×2, proptest ×1). The mandatory lanes (Verus ×6, cargo test ×5, Miri ×1, waiver ×1) are reviewed as pass/conditional-pass. Kani and proptest stubs must be repaired before they can be executed.

---

*Proof reviewer: state 6 review complete for vb-core-proof-gate-inputs*
