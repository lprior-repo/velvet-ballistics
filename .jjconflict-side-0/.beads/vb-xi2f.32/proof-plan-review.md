# Proof Plan Review: Wait Digest Coverage

**Reviewer skill:** `proof-plan-reviewer`
**Reviewer invocation ID:** `ppr-2026-05-25T14-00-00-vb-xi2f.32-002`
**Review state:** proof-plan-review (State 5)
**Bead:** vb-xi2f.32
**Date:** 2026-05-25

---

## STATUS: APPROVED

This is a re-review of the repaired artifacts following the prior rejection (reviewer `ppr-2026-05-24T03-35-00-vb-xi2f.32`). All eight schema-compliance findings (F-001 through F-008) and two waiver findings (F-009, F-010) from the prior review have been fully addressed. All repaired artifacts conform to their respective canonical schemas.

---

## Reviewed Artifacts

| Artifact | Path | Hash (SHA-256) |
|----------|------|-----------------|
| Proof seeds | `proof-seeds.jsonl` | `1bb08c770616e48572394543849c2d0973d9ef7d8b1cfbb9ecb306a8a5b015a1` |
| Proof strategy | `proof-strategy.md` | `6117edaaf500903fa6d3e857863f51fd1d1dd3fe10ad8328e0fe25f6dffac577` |
| Lane decisions | `verifier-lane-decisions.jsonl` | `1940f4917b743db47fe243bb65a8b60756e229658711a16046ed6a29c98953e7` |
| Planned obligations | `proof-obligations.planned.jsonl` | `52adb0d87c33113753d34bbb231d7e8f2d60953e1b95fc88860418b9f0aa7652` |
| Trusted-base plan | `trusted-base-plan.md` | `f328f4d1698b35cd62dd614811185948b3889d896459b4db4301c3f78172d434` |
| Waiver candidates | `waiver-candidates.jsonl` | `8b4fadda60fb40d44154fc822f7891e303591661e39b3c045f13729d051de17a` |
| Traceability | `traceability-matrix.jsonl` | `6347b6a0c852286025e9c716e0a82fd1387ca858fce799f5430748571e32064c` |
| Invocation ledger | `agent-invocation-ledger.jsonl` | `80b291b5947840a936dd51632e9a44f294795fe9a84d348e088fa140b1313290` |

---

## Schema Compliance Verification

### proof-obligations.planned.jsonl (16 rows)
- `schema_version: "proof-obligation/v1"` present on all rows. ✓
- Canonical `model_bounds` (not legacy `bounds`). ✓
- Required fields present: `domain_claim`, `risk_tags`, `target`, `tool_metadata`, `trusted_base_refs`, `behavior_affecting`. ✓
- No legacy alias fields detected. ✓

### verifier-lane-decisions.jsonl (72 rows)
- `schema_version: "verifier-lane-decision/v1"` present on all rows. ✓
- Canonical `applicability` (not legacy `decision`). ✓
- Canonical `decision_reason` (not legacy `rationale`). ✓
- Canonical `required_obligation_ids` as arrays (not legacy singular `obligation_id`). ✓
- Required fields present: `risk_tags`, `non_applicability_evidence_refs`, `limitation_kind`, `owner_state`, `status`. ✓

### waiver-candidates.jsonl (5 rows)
- `schema_version: "waiver-candidate/v1"` present on all rows. ✓
- Canonical `review_status` (not legacy `status`). ✓
- Required fields present: `requirement_id`, `contract_clause`, `reason`, `behavior_affecting`, `boundary_proof`, `compensating_evidence`, `owner`, `expiry`. ✓
- Minor: legacy `clause` field persists alongside canonical fields. Non-blocking. ✓
- All five waivers are non-behavior-affecting (`behavior_affecting: false`). ✓

### agent-invocation-ledger.jsonl (2 rows)
- Second row is `agent-invocation/v1` with invocation_id `proof-planner-vb-xi2f.32-001`. ✓
- Independent review provenance: planner ID differs from reviewer ID. ✓

---

## Substantive Review

### 1. Core Verifier Coverage: PASS

9 proof seeds × 8 verifiers = 72 lane decisions. Every seed has complete coverage across the full core verifier set (tla-plus, verus, kani, flux-rs, loom, miri, proptest, cargo-fuzz). No silent omissions.

### 2. Non-Applicability Decisions: PASS

| Verifier | Disposition | Evidence Strength |
|----------|-------------|-------------------|
| TLA+ | not_applicable | Pure function, no temporal behavior. Cites boundary-map.md§§5,7 and hazard-analysis.md CH-1,CH-2 (rated NONE). Evidence docs confirmed present. |
| Verus | not_applicable | P1 scope, pure function with simple match-arm + hasher.update(). Cites hazard-analysis.md UPH-1, boundary-map.md§3. Reasonable proportionality judgment. |
| Flux | not_applicable | No refinement-type predicates needed. Validation handled by pattern matching + validate_wait_shape. Cites type-contracts.md§7, hazard-analysis.md RH-1 (NONE). |
| Loom | not_applicable | Zero concurrency. Cites hazard-analysis.md CH-1,CH-2 (NONE), workflow-model.md§5. |
| Miri | not_applicable | Zero unsafe code. `#![forbid(unsafe_code)]` enforced. Cites hazard-analysis.md UPH-1 (NONE), boundary-map.md§4. |

All `not_applicable` rows cite concrete evidence document references. Evidence files (`boundary-map.md`, `hazard-analysis.md`, `type-contracts.md`, `workflow-model.md`) confirmed present in bead directory.

### 3. Required Lanes: PASS

| Lane | Obligation Count | Responsibilities |
|------|-----------------|------------------|
| Kani | 5 (PO-001, PO-005, PO-010, PO-013, PO-015) | Panic-freedom, collision/preimage, cross-path equivalence, bounded exhaustiveness |
| proptest | 8 (PO-002, PO-004, PO-006, PO-008, PO-009, PO-011, PO-014, PO-016) | Digest sensitivity, WaitUntil/WaitEvent discrimination, sentinel unambiguity, determinism, cross-path equivalence, regression |
| cargo-fuzz | 3 (PO-003, PO-007, PO-012) | Adversarial collision hunting, sentinel boundary fuzzing, exhaustive collision |

All behavior-affecting seeds (ps-wait-001 through ps-wait-006, ps-wait-009) are covered by at least two verifier lanes. Defense-in-depth is satisfied.

### 4. Trusted-Base Plan: PASS

Five trusted components (TB-001 through TB-005) with documented properties, boundaries, and impact analysis:
- **TB-001**: blake3 (collision resistance, determinism, panic-freedom of `Hasher::update()`)
- **TB-002**: YAML validation gate (rejects illegal `(None, None)` Wait before digest)
- **TB-003**: Rust stdlib `String`/`&str`/`Option` (fundamental correctness)
- **TB-004**: `WorkflowDigest` type (newtype wrapper, derive macros)
- **TB-005**: YAML parser vb_yaml (correct AST field extraction)

Assumed bounds are honest: 16-char slot text for Kani (real slot text is 1-7 chars), 4-char+small-alphabet for exhaustive Kani collision proof. Trusted harness expectations (TH-001 through TH-003) described for proof-writer.

### 5. Obligation Quality: PASS

All 16 obligations have:
- Exact command with harness name and crate flag (`cargo kani --harness ... -p vb_compile`)
- Explicit workdir (`/home/lewis/src/vb-workspaces/vb-xi2f.32`)
- Concrete expected evidence (e.g., "Kani reports SUCCESS: all proof harnesses satisfied")
- Declared assumptions with bounded parameters
- `model_bounds` objects with max_string_len and alphabet
- Trusted-base references (`trusted_base_refs`)
- Behavior-affecting classification matching proof seeds

### 6. Non-Vacuity: PASS

The plan explicitly tests for sensitivity (different inputs → different outputs), not just determinism (same input → same output):
- proptest PO-002, PO-004, PO-006, PO-011 test digest sensitivity to field changes
- Kani PO-005, PO-013 prove bounded collision-freedom for distinct Wait shapes
- cargo-fuzz PO-003, PO-007, PO-012 hunt for adversarial collisions
- Trusted-base plan honestly acknowledges blake3 collision resistance is trusted (TB-001)

### 7. Bridge/Failure Planning: PASS

Section 7 of proof-strategy.md maps each proof failure mode to the correct response:
- Kani panic → fix implementation, not harness (GOD RULE 4)
- proptest collision → fix implementation (likely discriminator/ordering bug)
- cargo-fuzz collision → fix implementation, not fuzz target
- Cross-path divergence → apply fix to both copies identically

### 8. Traceability: PASS

15 traceability rows link requirements C1-C8 to domain invariants (DI-1 through DI-5), hazards (RCIH-1 through RCIH-4, RAH-2, HIH-1, RH-1, RH-2, EC-4), proof seeds (ps-wait-001 through ps-wait-009), test requirements, and type contracts.

### 9. Waivers: PASS

Five non-behavior-affecting waiver candidates:
- WC-001: Verus (P1 scope; Kani+proptest+fuzz cover real risks)
- WC-002: TLA+ (pure function, no temporal behavior)
- WC-003: Loom (zero concurrency risk)
- WC-004: Miri (zero unsafe code)
- WC-005: Flux (no refinement-type predicates)

Zero behavior-affecting waivers proposed. All behavior requirements (C1-C6) are covered by proof obligations.

---

## Lane Review Disposition

All 72 `verifier-lane-review/v1` rows are written with `reviewer_disposition: accepted`. See `verifier-lane-review.jsonl` for the complete ledger.

Planner invocation: `proof-planner-vb-xi2f.32-001`
Reviewer invocation: `ppr-2026-05-25T14-00-00-vb-xi2f.32-002`
(Independent: reviewer ≠ planner)

---

## Minor Observations (Non-Blocking)

1. **waiver-candidates.jsonl** retains a legacy `clause` field alongside the canonical `requirement_id`/`contract_clause` fields. This is extra data that does not violate the `waiver-candidate/v1` schema. Recommended cleanup in a future maintenance pass.

2. **waiver-candidates.jsonl** `owner` field is set to `"proof-plan-reviewer"`. The owner should typically be the planner or bead requestor who will carry the waiver through its lifecycle. Non-blocking; the reviewer at State 5 can re-assign.

3. **Signed hashes in invocation ledger**: The planner invocation row has `output_artifact_hashes: {}` (empty). Post-review artifact hashes are in this review's header. Recommended for the planner to populate these before proof-writer begins.

---

## Overall Assessment

The repaired proof plan is schema-compliant and substantively strong. The P1 proportional posture (Kani + proptest + cargo-fuzz as the spine; TLA+/Verus/Flux/Loom/Miri as not_applicable with evidence) is well-justified for this pure-function fix. Honest bounds, explicit assumption declarations, clear trusted-base boundaries, and failure-mode mapping to implementation fixes meet the bar for proof-writer and proof-to-implementation handoff.

**Approved. Proceed to proof-writer (State 6).**
