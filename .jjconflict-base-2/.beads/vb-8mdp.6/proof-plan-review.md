# Proof Plan Review — vb-8mdp.6

## Bead: vb-8mdp.6
## Title: Add deterministic idempotency hydration tests for ActionTicket
## Status: APPROVED

---

## Reviewer Information

**reviewer_skill**: proof-plan-reviewer
**reviewer_invocation_id**: proof-plan-reviewer-vb-8mdp-6-20260525-r2
**planner_invocation_id**: proof-planner-vb-8mdp-6-20260525
**review_state**: approved (re-review of attempt 2 repairs)
**review_timestamp**: 2026-05-25
**review_type**: re-review of repairs from attempt 1 rejection

---

## Reviewed Artifacts

| Artifact | Hash (Evidence) |
|----------|-----------------|
| proof-strategy.md | Reviewed |
| verifier-lane-decisions.jsonl | 37 rows reviewed |
| proof-obligations.planned.jsonl | 33 rows reviewed |
| proof-coverage-matrix.md | Reviewed |
| trusted-base-plan.md | Reviewed |
| waiver-candidates.md | Reviewed |
| waiver-candidates.jsonl | 4 entries reviewed (W001 withdrawn) |
| verifier-lane-review.jsonl | 37 rows reviewed |

---

## Re-Review Scope

Per femdation protocol, this re-review focused exclusively on whether the 5 prior findings from attempt 1 are truly fixed. No new aspects were re-reviewed.

---

## Prior Findings Verification

| Finding ID | Code | Artifact | Status | Verification Evidence |
|------------|------|---------|--------|----------------------|
| F001 | E_LANE_DECISION_WEAK | VLD-008, VLD-019 | **FIXED** | Both lanes: `decision:"required"`, `status:"pending_flux_proof"`, `waiver:null`. W001 withdrawn from waiver-candidates.jsonl. Flux deferred to proof-writing phase. |
| F002 | E_COMMAND_EVIDENCE_MISSING | VLD-031 | **FIXED** | Command replaced with `cargo tree -p vb_core -e normal 2>&1 \| grep vb_storage \|\| echo 'PASS: no vb_storage deps'` |
| F003 | E_SCHEMA_VERSION_MISSING | VLD | **FIXED** | All 37 VLD rows have `schema_version:"verifier-lane-decision/v1"` |
| F004 | E_SCHEMA_VERSION_MISSING | PO | **FIXED** | All 33 PO rows have `schema_version:"proof-obligation/v1"` |
| F005 | E_SCHEMA_MISSING_FIELD | PO | **FIXED** | All 33 PO rows have `workdir`, `trusted_base_refs`, and `tool_metadata` fields |

---

## Lane Decision Summary (Actual Counts)

| Lane | Total Decisions | Accepted | Rejected | Notes |
|------|----------------|----------|----------|-------|
| kani | 18 | 18 | 0 | |
| tla-plus | 5 | 5 | 0 | |
| verus | 3 | 3 | 0 | |
| proptest | 2 | 2 | 0 | |
| cargo | 1 | 1 | 0 | |
| flux-rs | 2 | 2 | 0 | Status: pending_flux_proof |
| miri | 2 | 2 | 0 | not_applicable accepted |
| loom | 2 | 2 | 0 | not_applicable accepted |
| cargo-fuzz | 1 | 1 | 0 | not_applicable accepted |

**Total**: 37 lane decisions, 37 accepted, 0 rejected.

**NOTE**: Prior review summary incorrectly reported tla-plus as 7 (actual: 5) and verus as 2 (actual: 3). The VLD data is correct; only the summary table had a documentation error. This was identified and corrected in this re-review.

---

## Flux Lane Status (F001 Repair Verification)

VLD-008 and VLD-019 (flux-rs for VB-IDEM-HYDR-003 and VB-IDEM-HYDR-009):
- Both marked `decision:"required"` (not waived)
- Both have `status:"pending_flux_proof"`
- Both have `waiver:null`
- W001 (waiver candidate) properly withdrawn
- PO-VB-IDEM-003c and PO-VB-IDEM-009b created as proper Flux proof obligations
- Kani + TLA+ provide runtime coverage for taint rejection paths
- Contract clauses PS-VB-IDEM-003 and PS-VB-IDEM-009 explicitly scoped to runtime-only validation

This is the correct Option B repair: contract enforcement downgraded to runtime-only, Flux lane retained as required but deferred to proof-writing.

---

## Non-Vacuity Evidence

- **Kani**: Bounded exhaustion of u64+u64+u32 input space for key determinism
- **TLA+**: Finite state model checking with stated constants (MaxRuns=2, MaxActions=3, MaxSeq=4)
- **Verus**: Abstract state machine refinement proofs for is_resolved monotonicity
- **Proptest**: 1000 iterations with statistical coverage
- **Flux** (deferred): Type-level slot taint enforcement pending proof-writing

---

## Bridge Planning Assessment

The proof strategy identifies bridge requirements:
- TLA+ specs (`IdempotencySafety.tla`, `RecoveryHydration.tla`) → Rust implementation
- Verus specs → `ActionReplayTracker` implementation  
- Kani harnesses → `vb_storage/src/kani_recovery_hydrate.rs`
- Flux specs → type-level slot taint enforcement (deferred to proof-writing)

Bridge planning is complete for all accepted lanes.

---

## Reviewer Disposition

All 5 prior findings are verified FIXED. The proof plan is precise enough for proof-writer to begin. Flux lane is properly handled as deferred (not waived), which is an acceptable treatment per the repair guide Option B.

---

## verifier-lane-review.jsonl

All 37 lane decisions reviewed with `reviewer_disposition: accepted`. Independent reviewer_invocation_id: `proof-plan-reviewer-vb-8mdp-6-20260525-r2`.

---

## STATUS: APPROVED

All prior findings resolved. Plan ready for proof-writer (State 5).

**Next Action**: Dispatch proof-writer to begin artifact writing for accepted lanes.

(End of file - total 104 lines)
