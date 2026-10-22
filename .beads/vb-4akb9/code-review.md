# Code Review: vb-k8z9l and vb-my3b3 Typed Artifact Fixture Migration

**Reviewer:** LOCAL3090 independent reviewer (vb-4akb9)
**Workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-4akb9-local3090-artifact-review
**Commits Reviewed:** c33ae5e008f2 (parent be65d919), 5effe4a0bf1d (intermediate 0efe), f253538b97a4 (parent 5eff)

## Decision: APPROVE

### Scope
Migration of five artifact fixture suites plus three additional test files to typed evidence contract (schema 3, canonical 10-entry ledger) with real producer fixtures.

### Evidence
**Tests executed via immutable runner /tmp/vb-run-gate.py with RUSTFLAGS=-Dwarnings:**

| Suite | Tests | Result |
|-------|-------|--------|
| vb_storage/accepted_artifact_red_phase | 29 | 29 passed |
| vb_core_yaml_e2e_chain_contract | 35 | 35 passed |
| vb_lp2v_admission_integration | 9 | 9 passed |
| vb_lp2v_admission_proptests | 9 | 9 passed |
| vb_lp2v_proof_admission_bdd | 3 | 3 passed |
| vb_9agmk_artifact_evidence | 25 | 25 passed |
| vb_qi37_4_2_strict_runtime_admission | 22 | 22 passed |
| vb_ssei_verification_admission_acceptance | 4 | 4 passed |
| **Total** | **146** | **146 passed** |

Proptest regression seed replay verified all 22 tests in vb_qi37_4_2_strict_runtime_admission pass with retained seeds (root restored during merge 78a0ff).

### Verified Assertions

**Producer fixture quality:**
- All positive cases use real `prepare_compiled_artifact` (not fake placeholder artifacts)
- Artifact digests computed via actual blake3 postcard encoding, not renamed stubs
- Real source digest vs compiled artifact digest distinct (source_digest != digest)
- Schema 3 canonical 10-entry typed ledger present in all produced artifacts

**Typed evidence contract:**
- `ArtifactEvidence::Verified(Box<ArtifactVerificationEvidence>)` asserted (not Unverified/Packed)
- `VerificationLedgerEntry` obligations verified: Determinism, EffectSafety, CapabilityNonMinting, Boundedness, ReplayEffectSeparation, ArtifactValidity, IdempotencyClassification, RecoveryClassification, TaintHandling, PolicyBinding
- `ObligationClosure::RuntimeValidated` vs `ExternalAssumption` correctly distinguished
- EMPTY operator attestations verified (prepare_compiled_artifact does not infer OPERATOR ATTESTATIONS)

**No legacy flag authority:**
- `gate_count == 0` for all policies (display-only)
- `durable == false` for all policies (determined by submit, not producer)
- `bounded_claimed == true`, `idempotency_verified_claimed == true`
- `taint_safe_claimed == false`, `retry_safe_claimed == false`, `replayable_claimed == false`
- No test uses legacy flag values as verification authority

**Exact rejection payloads:**
- Unverified ledger rejects with `ArtifactInvalidProofFlag { flag: "evidence" }` (not `ArtifactInvalidGateCount`)
- Digest mismatch: `ArtifactDigestMismatch { requested, found }`
- Capability denied: `CapabilityDenied { action, required, granted }`
- Capability count mismatch: `CapabilityCountMismatch { required_count, granted_count }`
- No allocation on rejection verified (no frame, run, or drive state)

**Strong tamper matrices:**
- Schema version fails closed (any u16 != 3 rejected)
- Verification digest must equal artifact digest
- Arbitrary payload bit change rejected
- Changed valid IR and contract declarations cannot reuse old evidence
- Rehashed forged closure and duplicate ledger entries rejected
- Malformed IR and trailing payload bytes reject after envelope rehash
- Bound checked before decoding or hashing payload

**Positive producer accepts:**
- All policies (Relaxed, Journaled, Strict) accept valid prepared artifacts
- No shortcircuit/skip/ignore/catch error pretend passes

### Remaining Gaps / Notes

1. **Dead code warning:** `accepted_artifact_with_required_capability` in vb_core_yaml_e2e_chain_contract.rs is never used (compiles with RUSTFLAGS=-Dwarnings but produces a warning). Author should remove or use.

2. **Stale author metadata:** Author's TypedLedger fake enum descriptions may remain stale in documentation. Root should verify documentation matches actual schema 3 structure.

3. **Old qi37 gate count cases replaced:** The old `gate_count 0/2/14/16` fake artifact tests have been replaced with real prepared artifact type-based rejection tests. This is correct and stronger than the old approach.

### Commands Used
```bash
RUSTFLAGS=-Dwarnings /tmp/vb-run-gate.py 600 .beads/vb-4akb9/evidence/storage-red cargo test -p vb_storage --test accepted_artifact_red_phase
RUSTFLAGS=-Dwarnings /tmp/vb-run-gate.py 600 .beads/vb-4akb9/evidence/workspace-five cargo test -p velvet-ballistics-workspace-tests --test vb_core_yaml_e2e_chain_contract --test vb_lp2v_admission_integration --test vb_lp2v_admission_proptests --test vb_lp2v_proof_admission_bdd
RUSTFLAGS=-Dwarnings /tmp/vb-run-gate.py 600 .beads/vb-4akb9/evidence/vb-9agmk-artifact cargo test -p velvet-ballistics-workspace-tests --test vb_9agmk_artifact_evidence
RUSTFLAGS=-Dwarnings /tmp/vb-run-gate.py 600 .beads/vb-4akb9/evidence/vb-qi37-admission cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_4_2_strict_runtime_admission
RUSTFLAGS=-Dwarnings /tmp/vb-run-gate.py 600 .beads/vb-4akb9/evidence/vb-ssei cargo test -p velvet-ballistics-workspace-tests --test vb_ssei_verification_admission_acceptance
RUSTFLAGS=-Dwarnings /tmp/vb-run-gate.py 600 .beads/vb-4akb9/evidence/seed-replay cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_4_2_strict_runtime_admission -- --include-ignored
```