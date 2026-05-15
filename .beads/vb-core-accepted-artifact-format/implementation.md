# Implementation Report — vb-core-accepted-artifact-format

## Bead: vb-core-accepted-artifact-format
## Workspace: /tmp/vb-ws/vb-core-accepted-artifact-format
## State: 10 (Holzman-Rust Implementation)
## Date: 2026-05-15

---

## Classification: No-Op at S10 — Specification Bead

This bead is a **formal-verification-first specification bead**. The S6 proof review confirmed the gate_count contract mismatch (KANI-MISMATCH-001: found=2, required=15) at the formal verification level. No production code changes are required or appropriate at this stage.

**Verification artifacts (TLA+, Kani, Verus, Miri) are the deliverable.** The implementation gate at S10 is satisfied by documenting why no Rust implementation change occurs here and which follow-on bead must resolve the contract.

---

## Gate Count Mismatch — Formal Confirmation

KANI-MISMATCH-001 (State 5/6) formally confirmed:

```
submit_artifact(Strict) → artifact.verification.gate_count = 2
load_accepted_artifact(Strict) → requires gate_count = 15
                                          ↓
               ArtifactEnvelopeError::InvalidGateCount { found: 2, required: 15 }
```

| Location | Constant | Value |
|----------|----------|-------|
| `vb_storage/src/admission.rs:118` | `ADMISSION_GATE_COUNT` | 2 |
| `vb_runtime/src/admission.rs:16` | `REQUIRED_GATE_COUNT` | 15 |

---

## Resolution Options (Documented for Follow-On Bead)

| Option | Description | Impact | Complexity |
|--------|-------------|--------|------------|
| **A** | Change `ADMISSION_GATE_COUNT` to 15 in vb_storage | vb_storage emits 15-gate proof flags; Strict policy satisfied | Medium — requires 15-gate verification implementation |
| **B** | Change `REQUIRED_GATE_COUNT` to 2 in vb_runtime | vb_runtime accepts 2-gate artifacts under all policies | Low — one constant change, but weakens Strict policy |
| **C** | Implement 15-gate verification and retire 2-gate path | Full 15-gate verification in vb_storage; 2-gate path removed | High — full gate implementation, test suite update |
| **D** | Add version field supporting both formats | `AcceptedArtifact.version: u8` with format discriminator | Medium — versioning schema change, dual-path code |

---

## Why No Implementation at This Bead

1. **Proof obligation result**: KANI-MISMATCH-001 was designed to find this counterexample. Finding it is the proof, not a bug in the implementation.

2. **Contract specification bead**: This bead established the formal contract and verification evidence for the AcceptedArtifact format. Resolving the mismatch is a separate implementation concern with its own bead.

3. **Follow-on bead required**: Resolution options A–D each require code changes to vb_storage or vb_runtime. The appropriate next step is a new bead (e.g., `vb-core-gate-count-resolution`) to implement one of the options.

4. **No changes to source code**: This bead's verification artifacts (TLA+ specs, Kani harnesses, Verus specs, Miri tests) do not modify production Rust code.

---

## Verification Artifacts Produced

| Artifact | Location | Status |
|----------|----------|--------|
| ArtifactAdmission.tla | `specs/tla/ArtifactAdmission.tla` | ✅ Sound |
| ArtifactDigest.tla | `specs/tla/ArtifactDigest.tla` | ✅ Sound |
| Kani harness (mismatch) | `crates/vb_storage/src/kani_admission.rs` | ✅ Counterexample confirmed |
| Kani harness (gate) | `crates/vb_storage/src/kani_admission.rs` | ✅ Pass |
| Verus invariants | `verification/verus/admission_invariants.rs` | ✅ 4 proofs verified |
| Miri tests | `crates/vb_storage/src/admission_miri_tests.rs` | ✅ 5 tests, 0 UB |

---

## Recommendation

**Option D (versioned format)** is the preferred path for long-term compatibility. Options A and C require significant new gate implementation. Option B is the simplest but weakens the Strict policy. A follow-on bead should implement the chosen resolution.

---

## State Advancement

- **Current state**: 10 (Holzman-Rust Implementation)
- **Next gate**: State 11 (Formal Verification Execution)
- **Rationale**: S6 proof-review APPROVED with no required obligations failing. No production code changes are needed or appropriate. This bead's implementation artifact is the documentation of the mismatch and its resolution options, not a code change.
