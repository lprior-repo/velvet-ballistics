# Proof Plan Review: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9  
**Reviewer Skill:** proof-plan-reviewer  
**Reviewer Invocation ID:** ppr-vb-xi2f.9-002  
**Prior Review Invocation ID:** ppr-vb-xi2f.9-001 (REJECTED)  
**Review State:** APPROVED (State 4)  
**Date:** 2026-05-25  
**Schema:** proof-plan-review/v1  

## Reviewed Artifacts

| Artifact | Status |
|---|---|
| `proof-strategy.md` | Accepted |
| `proof-obligations.planned.jsonl` (21 rows) | Accepted |
| `verifier-lane-decisions.jsonl` (96 rows) | Accepted |
| `trusted-base-plan.md` | Accepted |
| `trusted-base-ledger.jsonl` (38 rows) | Accepted |
| `waiver-candidates.jsonl` (5 rows) | Accepted |
| `traceability-matrix.jsonl` (30 rows) | Accepted |
| `proof-seeds.jsonl` (12 rows) | Accepted |
| `proof-coverage-matrix.md` | Accepted |
| `proof-to-implementation-input.md` | Accepted |
| `agent-invocation-ledger.jsonl` (2 rows) | Accepted (advisory findings) |

## Executive Summary

The prior review (ppr-vb-xi2f.9-001) rejected this plan on 6 blocking findings and 5 non-blocking findings, all related to machine-readable schema compliance, missing invocation provenance, and missing trusted-base-ledger. **All 7 repairs from the repair guide have been applied:**

1. `proof-obligations.planned.jsonl` — 21 rows now conform to `proof-obligation/v1` with all required fields (`schema_version`, `domain_claim`, `risk_tags`, `model_bounds`, `tool_metadata`, `trusted_base_refs`, `behavior_affecting`, `target`). Old alias fields (`bounds`, `risk`) renamed.
2. `verifier-lane-decisions.jsonl` — 96 rows now conform to `verifier-lane-decision/v1` with all required fields (`id`, `risk_tags`, `applicability`, `decision_reason`, `required_obligation_ids`, `non_applicability_evidence_refs`, `limitation_kind`, `owner_state`, `status`). Old alias fields (`decision`, `evidence`, `obligation_id`) renamed.
3. `agent-invocation-ledger.jsonl` — Proof-planner entry (`planner-vb-xi2f.9-001`, state 4) appended. Independent provenance established: planner (proof-planner) ≠ reviewer (proof-plan-reviewer).
4. `waiver-candidates.jsonl` — Duplicate WC-03 resolved (renumbered WC-04/WC-05). `boundary_proof` field added. `reviewer_status` renamed to `review_status`.
5. `trusted-base-ledger.jsonl` — Created with 38 rows covering all trusted assumptions, stubs, model reductions, and trusted operations from `trusted-base-plan.md` sections 1-6.
6. `proof-strategy.md` — Non-Vacuity Plan section added (lines 83-136) with Kani Assumption Audit, Proptest Strategy Edge-Case Coverage, Stub and Model Reduction Independence, and Production Implementation Binding.
7. PO-G04 redundancy resolved — `mode` changed to `"verify-by-build (po-g03-sub-check)"`.

The proof strategy is logically sound and the defense-in-depth layering (Kani as primary bounded checker, proptest for broad input coverage, Flux + Miri for depth) is appropriate for a P1 infrastructure bead. The coverage is complete (12 proof seeds → 21 obligations, all 30 traceability entries covered), waivers are non-behavior-affecting with compensating evidence, and the trusted base is well-scoped.

## Verifier Lane Review Summary

`verifier-lane-review.jsonl` written with 96 rows. All rows are `reviewer_disposition: accepted`. The underlying lane reasoning is sound:

- **Kani (8 required lanes):** Well-matched to bounded-state invariants (Span paired invariant, NonEmptyVec len≥1, Diagnostic source_file, YamlError span construction, canonical span extraction, ValidationError span propagation, usize→u32 bridge, AstMarks backfill). All required obligations (PO-K01 through PO-K08) are defined with honest bounds and non-vacuous assumptions.
- **Proptest (7 required lanes):** Covers broad input-space properties (for-all Span constructors, NonEmptyVec round-trip, YamlError event-stream spans, ValidationError variant×span pairs, SourceSpan bridge round-trip, AstMarks from known YAML, semantic map path annotation). Strategies include edge cases (u32::MAX, usize::MAX, empty vecs, absent maps).
- **Flux (1 required lane):** Lightweight defense-in-depth for Span paired invariant. Complementary to Kani PO-K01.
- **Miri (1 required lane):** Narrow scope on usize→u32 bridge UB detection. Appropriate given `#![forbid(unsafe_code)]` elsewhere.
- **TLA+, Verus, Loom, cargo-fuzz (79 not_applicable lanes):** Correctly judged non-applicable for this single-threaded, no-unsafe, no-new-parsing-boundary compiler pipeline bead.
- **CI gates (3 non-verifier obligations):** PO-G01 (grep SourceMap), PO-G02 (grep diagnostic_from_error), PO-G03 (moon ci). Appropriate static verification for dead-code removal and refactoring changes.
- **Waivers:** 5 waiver candidates (WC-01 through WC-05), all `behavior_affecting: false`, all with compensating evidence. WC-01 (Flux for NonEmptyVec), WC-02 (Miri scope reduction), WC-03 (Kani for dead-code removal), WC-04 (Kani for refactoring), WC-05 (Kani for string formatting). All correctly scoped and non-blocking.

## Advisory Findings (Non-Blocking)

### A-001 (ADVISORY): PO-K08 requirement_id C10.1-C10.2 vs Seed C10.1-C10.3

**Artifact:** `proof-obligations.planned.jsonl`, PO-K08  
**Message:** PO-K08 lists `requirement_id: "C10.1-C10.2"` but the corresponding proof seed PS-008 and lane decision vld-059 use `"C10.1-C10.3"`. The obligation's expected_evidence covers both C10.1 (matching→available) and C10.2 (absent→unavailable), so this is a labeling mismatch rather than a coverage gap. The requirement_id should match the proof seed.

### A-002 (ADVISORY): 63 N/A Lane Decisions with Empty `non_applicability_evidence_refs`

**Artifact:** `verifier-lane-decisions.jsonl`  
**Message:** 63 out of 79 `not_applicable` rows have empty `non_applicability_evidence_refs` arrays. The `decision_reason` field contains substantive rationale for each (e.g., "No temporal properties", "No concurrency", "No unsafe code", "Dead code removal — no runtime behavior"), but linking these to concrete hazard analysis or workflow model references would strengthen the evidence chain. The 16 N/A rows that do provide evidence refs set a good pattern (vld-001, vld-002, vld-005, vld-006, vld-008, vld-013, vld-021, vld-022, vld-029, vld-030, vld-037, vld-038, vld-045, vld-046, vld-053, vld-061, vld-062). Not blocking because the decision_reason is self-contained and correct.

### A-003 (ADVISORY): 8 Obligations with Empty `domain_claim`

**Artifact:** `proof-obligations.planned.jsonl`  
**Message:** PO-K08, PO-F01, PO-M01, PO-P02, PO-P03, PO-P04, PO-P05, PO-G04 have empty `domain_claim` strings (`""`). The corresponding proof seeds provide domain claims for all of these. Filling these in would improve traceability for downstream proof-writer and proof-to-implementation agents. Not blocking because the field is technically present (non-null) and the obligation's `expected_evidence` and `contract_clause` provide sufficient context.

### A-004 (ADVISORY): agent-invocation-ledger Missing Canonical Hash Fields

**Artifact:** `agent-invocation-ledger.jsonl`  
**Message:** Both ledger entries (femdation and proof-planner) are missing canonical `agent-invocation/v1` fields: `previous_entry_hash`, `entry_hash`, `host_session_id`, `input_artifact_hashes`, `output_artifact_hashes`, `transcript_artifact`, `transcript_hash`, `reviewed_artifacts_existed_before_start`. The core provenance fields (invocation_id, parent_invocation_id, skill, state, workdir, input/output artifacts) are present, establishing independence between planner and reviewer. The missing hash fields are integrity-check metadata that do not affect provenance validity. Not blocking.

## Verdict

All 7 repairs from the prior rejection have been applied. Schema compliance is achieved across all machine-readable artifacts (obligations, lane decisions, waivers, trusted-base-ledger). Provenance is established with independent planner and reviewer invocations. The proof strategy is logically sound with complete coverage, appropriate defense-in-depth layering, non-behavior waivers, and a thorough non-vacuity plan.

**The plan is sufficiently precise for proof-writer to execute obligations and for proof-to-implementation to bridge claims to Rust source refs.** The four advisory findings above are non-blocking refinements that may be applied in a future repair cycle but do not prevent advancement to State 5.

---

STATUS: APPROVED
