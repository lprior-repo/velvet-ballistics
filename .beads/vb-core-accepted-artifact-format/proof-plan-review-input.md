# Proof Plan Review Input — vb-core-accepted-artifact-format

## Bead & Workspace
- **Bead ID**: `vb-core-accepted-artifact-format`
- **Workspace**: `/tmp/vb-ws/vb-core-accepted-artifact-format`
- **Reviewer**: contract-verification-reviewer (S5 gate)

---

## Executive Summary

The `AcceptedArtifact` format bead has **one central known defect**: `vb_storage::submit_artifact` produces artifacts with `gate_count=2` but `vb_runtime::load_accepted_artifact` requires `gate_count=15` under Strict/Journaled policy. This mismatch means all stored artifacts will be rejected by the runtime under the strict policy.

**14 proof obligations** are planned across 7 verifier lanes. The mismatch is itself the subject of KANI-MISMATCH-001 — a proof obligation whose expected result is a counterexample, not the absence of one.

---

## Contract Clauses and Coverage

| Clause | Description | Verifiers | Coverage |
|--------|-------------|-----------|----------|
| INV-001 | `digest == sha256(ir)` | TLA-ARTIFACT-002, VERUS-INV-001 | Full |
| INV-002 | `gate_count >= 1` | TLA-ARTIFACT-001, VERUS-INV-002 | Full |
| INV-003 | Proof flags derived (not hardcoded) | VERUS-INV-003 | Full — flag gap |
| INV-004 | `try_from_parts` is sole `CompiledWorkflow` constructor | VERUS-PRE-001 | Full |
| INV-005 | Persistence atomic with journal seq | TLA-ARTIFACT-001, LOOM-CONCURRENT-001 | Partial (Loom optional) |
| PRE-001 | Caller provides valid `CompiledWorkflow` | VERUS-PRE-001, KANI-GATE-001 | Full |
| PRE-003 | IR postcard-decode to valid `WorkflowParts` | MIRI-DECODE-001, MIRI-SAFETY-001, FUZZ-DECODE-001 | Full |
| PRE-004 | Digest matches SHA-256 of IR bytes | TLA-ARTIFACT-002, KANI-GATE-001 | Full |
| POST-001 | `AcceptedArtifact` serde traits | API-COMPAT-001 | Full |
| POST-002 | `VerificationProof` serde traits | API-COMPAT-002 | Full |
| POST-003 | `submit_artifact` returns `gate_count=2` | KANI-MISMATCH-001, TLA-ARTIFACT-001 | Full (mismatch confirmed) |
| POST-004 | Stored artifact passes Relaxed only | KANI-MISMATCH-001, TLA-ARTIFACT-001 | Full |
| ERR-001 | Error taxonomy exhaustive | VERUS-INV-003, KANI-MISMATCH-001 | Full |

---

## Critical Obligation: KANI-MISMATCH-001

**Harness**: `gate_count_mismatch_harness`

**Scenario**:
1. Construct `CompiledWorkflow` via `CompiledWorkflow::try_from_parts(workflow_parts, ...)`
2. Call `submit_artifact(journal, &workflow, Relaxed) → Ok(artifact)` — artifact has `gate_count=2`
3. Call `load_accepted_artifact(artifact_store, &artifact.digest, Strict)`
4. Expect: `Err(InvalidGateCount { found: 2, required: 15 })`

**Kani expectation**: Counterexample found — confirming the mismatch exists as documented.

**Why this is the primary blocking obligation**: If Kani finds NO counterexample, it means either (a) the mismatch is resolved or (b) the harness is incorrectly structured. The contract-verification-reviewer must validate the harness structure.

---

## Traceability Summary

- **16 contract clauses** covered by **14 proof obligations**
- **4 obligations** with `required: false` (LOOM-CONCURRENT-001, API-COMPAT-001, API-COMPAT-002, FUZZ-DECODE-001)
- **FUZZ-DECODE-001** deferred to owner_state=6
- **MIRI-DECODE-001** and **MIRI-SAFETY-001** owner_state=6 — verify-safe on decode

---

## Assumptions

1. TLA+ specs exist at `specs/ArtifactAdmission.tla` and `specs/ArtifactDigest.tla` with correct invariants
2. `CompiledWorkflow::try_from_parts` is accessible to Kani harness
3. `vb_storage` and `vb_runtime` crates are in the same workspace; cross-crate harness composition is valid
4. `StorageArtifactStore` implements `AcceptedArtifactStore` trait with `load_accepted_artifact(digest, policy)` signature
5. `ArtifactEnvelopeError::InvalidGateCount { found, required }` is reachable in the runtime

---

## Reviewer Action

The contract-verification-reviewer (S5) must:
1. Validate KANI-MISMATCH-001 harness structure is sound
2. Confirm TLA+ spec invariants match contract clause semantics
3. Confirm VERUS-INV-003 flagging mode is appropriate for known hardcoded-gap
4. Approve or reject the proof strategy before formal execution
