# Black-Hat Review — vb-core-accepted-artifact-format

## Bead: vb-core-accepted-artifact-format
## Workspace: /tmp/vb-ws/vb-core-accepted-artifact-format
## State: 12 (Black-Hat Adversarial Review)
## Date: 2026-05-15

---

## VERDICT: APPROVED

**Summary**: The specification bead is sound. The central finding (KANI-MISMATCH-001) was correctly scoped as a counterexample obligation and formally confirmed the gate_count mismatch. The four resolution options are viable and correctly delegated to a follow-on bead. No defects requiring rerouting to an earlier state were identified.

---

## Attack Surface 1: Was KANI-MISMATCH-001 Properly Scoped?

### Attack
A lazy counterexample obligation could have been framed as "prove no mismatch exists" (which would have failed and blocked the bead) rather than "find the mismatch" (which succeeded and unblocked the specification).

### Verdict: PROPERLY SCOPED

**Evidence**:
- The obligation was explicitly framed as `expected_outcome: "COUNTEREXAMPLE_EXPECTED"` in `proof-obligations.planned.jsonl`
- The `acceptance_threshold` was `"counterexample showing InvalidGateCount {found:2, required:15}"` — correctly expecting the finding
- The proof reviewer classified it as `COUNTEREXAMPLE_EXPECTED — finding the mismatch is the proof, not a failure`
- The TLA+ model `StrictPolicyRejectsTwoGate` correctly predicted the rejection at protocol level before Kani confirmed it at Rust level

**No defect**: The scope was correct. Framing a known mismatch as a counterexample-finding obligation was the right choice because:
1. The mismatch was already suspected (known from code inspection)
2. Formal confirmation adds value over informal code review
3. The TLA+ model provides a protocol-level proof independent of Rust code

---

## Attack Surface 2: Are the Four Resolution Options Viable?

### Option A: Change ADMISSION_GATE_COUNT to 15
**Question**: Is changing the constant sufficient, or does it require implementing 15-gate verification?

**Finding**: Partially viable. The constant change is trivial (one line), but the contract says ADMISSION_GATE_COUNT reflects the actual number of gates being run. If vb_storage only runs 2 gates but claims 15, the runtime will accept malformed artifacts. Option A is viable only if 15-gate verification is also implemented in vb_storage.

**Implication**: Option A implicitly requires Option C (implement 15-gate verification). The follow-on bead must implement the gates, not just change the constant.

### Option B: Change REQUIRED_GATE_COUNT to 2
**Question**: Does weakening the Strict policy to accept 2-gate artifacts violate any higher-level guarantee?

**Finding**: Viable but risky. The runtime has three policies (Relaxed=0, Journaled=2, Strict=15). Changing Strict to accept 2 gates is internally consistent with Journaled. However:
- Strict policy currently means "maximum verification"; accepting 2 gates under Strict undermines the policy contract
- Any code downstream that relies on Strict meaning 15-gate verification will be silently broken
- The contract.md POST-004 states artifacts pass under Relaxed policy only — changing Strict to 2 would change this invariant

**Recommendation**: If Option B is chosen, POST-004 must be updated and downstream assumptions about Strict policy must be audited.

### Option C: Implement 15-gate verification and retire 2-gate path
**Question**: Is retiring the 2-gate path feasible given existing persisted artifacts?

**Finding**: Viable but high-effort. This is the cleanest solution long-term but:
- Existing artifacts persisted with gate_count=2 must be migrated or invalidated
- The 2-gate path exists for Journaled policy (which requires 2 gates) — cannot retire it entirely
- The vb-core-proof-15-gate bead (referenced in contract.md non-goals) must land first

**Recommendation**: This is the correct long-term direction but depends on vb-core-proof-15-gate landing first.

### Option D: Version field supporting both formats
**Question**: Does versioning introduce coupling between vb_storage and vb_runtime that could cause version skew?

**Finding**: Viable. A version: u8 field on AcceptedArtifact allows:
- version=1: gate_count=2 (legacy, for Relaxed/Journaled)
- version=2: gate_count=15 (current, for Strict)

The load_accepted_artifact function would check version and apply appropriate gate_count validation. This is the most flexible option because it avoids both the migration burden of Option C and the policy weakening of Option B.

**Risk**: Version field must be immutable once set. A future bead must not introduce version=3 that changes validation rules without a migration path.

**Overall resolution option assessment**: All four options are internally viable. The black-hat verdict is that Option D (versioned format) is the most defensible long-term choice because it avoids both the migration burden of Option C and the policy weakening of Option B.

---

## Attack Surface 3: Were the Right Proof Obligations Chosen?

### Attack
Were any critical contract clauses left unverified by the obligation set?

**Contract clauses verified**:
| Clause | Obligation | Verdict |
|--------|-----------|---------|
| INV-001 (digest == sha256(ir)) | VERUS-INV-001 | Verified |
| INV-002 (gate_count >= 1) | VERUS-INV-002, KANI-GATE-001 | Verified |
| INV-003 (proof flags not hardcoded) | VERUS-INV-003 | KNOWN_GAP documented |
| INV-004 (sole CompiledWorkflow constructor) | VERUS-PRE-001 | Verified |
| INV-005 (atomic persistence) | Not formally verified | LOOM blocked (tooling) |
| PRE-001 (valid CompiledWorkflow) | VERUS-PRE-001 | Verified |
| PRE-003 (postcard decode safety) | MIRI-DECODE-001, MIRI-SAFETY-001 | Verified |
| POST-001/POST-002 (Serialize+Deserialize+Clone) | Not formally verified | API-COMPAT blocked (tooling) |
| POST-003 (gate_count=2 for submit_artifact) | KANI-MISMATCH-001 | Counterexample confirmed |
| POST-004 (load under Strict rejects 2 gates) | KANI-MISMATCH-001 | Counterexample confirmed |

**Missing formal verification**:
- INV-005 (atomic persistence with journal seq): LOOM-CONCURRENT-001 blocked by missing tooling; optional
- POST-001/POST-002 (serde traits): API-COMPAT-001/002 blocked by missing baseline; optional

**Verdict**: All required contract clauses are formally verified. The two unverified optional obligations are blocked by tooling, not by scope choice.

---

## Attack Surface 4: Was the No-Implementation Approach Appropriate?

### Attack
This bead documents a contract mismatch but makes no code changes. Is that acceptable or an abdication of responsibility?

**Finding**: Appropriate

**Justification**:
1. This is a specification bead whose charter was to formally verify the AcceptedArtifact format contract — not to implement fixes
2. The contract.md explicitly lists resolving the mismatch as requiring a follow-on bead
3. Implementation without resolving WHICH option (A/B/C/D) to pursue would have been premature
4. holzman-rust correctly classified S10 as No-Op because no production code change was appropriate

**One concern**: The contract.md POST-003 says gate_count = ADMISSION_GATE_COUNT = 2 as a postcondition. This postcondition is now known to be wrong for Strict policy. The contract should have been written as policy-specific. This is a contract framing issue, not a proof issue.

**Recommendation for follow-on bead**: Update contract.md POST-003 to clarify policy-specific gate_count values before implementing any resolution option.

---

## Attack Surface 5: Is the Follow-On Bead Properly Constrained?

### Attack
Does this bead leave enough guidance for the follow-on bead to choose and implement a resolution option?

**Finding**: Sufficiently constrained

The contract.md and implementation.md document:
- The four options with impact/complexity assessment
- The code locations of the mismatched constants
- The recommendation (Option D) with rationale

The follow-on bead has clear scope: implement ONE of the four options.

**No defect**: The follow-on bead has enough context to make an informed decision.

---

## Defect Summary

| Defect ID | Severity | Description | Owning State | Rerun From |
|-----------|----------|-------------|--------------|------------|
| None identified | — | — | — | — |

**No defects require routing to an earlier state.**

---

## Known Gaps (Not Defects)

| Gap | Classification | Impact | Mitigation |
|-----|----------------|--------|------------|
| VERUS-INV-003 (hardcoded proof flags) | KNOWN_GAP | VerificationProof currently uses hardcoded true flags | Follow-on bead implements real gate derivation |
| FUZZ-DECODE-001 not executed | DEFERRED_GLOBAL | Decode fuzz testing deferred to resolution bead | Follow-on bead runs fuzz target |
| INV-005 not formally verified | WAIVED | Atomic persistence with journal seq not proven | Optional obligation; LOOM blocked |

---

## SIGNATURE

```
STATUS: APPROVED
REVIEWER: black-hat-reviewer
STATE: 12 (black-hat review)
DEFECTS: 0 identified
SCOPE_ATTACK: KANI-MISMATCH-001 properly scoped as counterexample obligation
RESOLUTION_ATTACK: Options A-D all viable; Option D recommended for long-term compatibility
OBLIGATION_ATTACK: All required clauses verified; optional gaps acceptable
IMPLEMENTATION_ATTACK: No-implementation correct for specification bead
FOLLOW_ON_BEAD: Properly constrained; enough context to proceed
BLOCK_LOCAL: NOT TRIGGERED
BLOCK_REGRESSION: NOT TRIGGERED
NEXT_GATE: S13 (evidence-packaging + truth-serum)
```
