# Proof Plan Review — vb-rqmw

## Reviewer Identity
- **Skill**: proof-plan-reviewer
- **Reviewer Invocation ID**: ppr-vb-rqmw-2026-05-22
- **Review State**: State 4
- **Date**: 2026-05-22

## Reviewed Artifacts

| Artifact | Hash (approx) | Status |
|----------|--------------|--------|
| `proof-strategy.md` | 7.6K | reviewed |
| `verifier-lane-decisions.jsonl` | 27.4K | reviewed |
| `proof-obligations.planned.jsonl` | 8.2K | reviewed |
| `proof-coverage-matrix.md` | 5.5K | reviewed |
| `trusted-base-plan.md` | 5.6K | reviewed |
| `waiver-candidates.jsonl` | 3.5K | reviewed |
| `traceability-matrix.jsonl` | 2.7K | reviewed |

## Lanes Reviewed

- **Verus**: 8 seeds (001-008) — 1 required lane per seed
- **TLA+**: 8 seeds — all `not_applicable`
- **Kani**: 8 seeds — all `not_applicable`
- **Flux**: 8 seeds — all `not_applicable`
- **Loom**: 8 seeds — all `not_applicable`
- **Miri**: 8 seeds — all `not_applicable`
- **proptest**: 8 seeds — all `blocked_tooling`
- **cargo-fuzz**: 8 seeds — all `blocked_tooling`
- **Total rows**: 64 (8 seeds × 8 verifiers)

## Findings

### F-G4-BLOCKER (CRITICAL — BLOCKS STATE 5)

**Code**: `E_ORPHAN_DECISION_MISSING`

**Artifact**: `verifier-lane-decisions.jsonl` row vld-008, `proof-obligations.planned.jsonl` PO-008, `trusted-base-plan.md` G4

**Message**: G4 (Orphaned Specs) is a hard blocker. The proof-planner explicitly states "Bead owner must decide BIND or REMOVE for each spec before proof-writer can proceed." No such decision is recorded. PO-008 lumps all 5 orphaned specs into a single combined obligation with a single `&&`-chained command, providing no per-spec granularity. The proof-writer cannot proceed: binding orphaned specs to Rust requires either (a) finding existing Rust admission paths or (b) creating new Rust bindings and proof obligations. The waiver candidates (WC-001 through WC-005) document that no binding currently exists but do not substitute for a BIND/REMOVE decision.

**Affected Seeds**: vb-rqmw-seed-008 (5 orphaned admission specs)

**Required Fix**: Bead owner Lewis must provide a BIND or REMOVE decision for each of the 5 orphaned specs:
1. `accepted_artifact_admission_decision.rs` — BIND or REMOVE
2. `accepted_envelope_model.rs` — BIND or REMOVE
3. `accepted_run_atomic_admission.rs` — BIND or REMOVE
4. `admission_artifact_model.rs` — BIND or REMOVE
5. `capability_artifact_model.rs` — BIND or REMOVE

If BIND: each spec needs a separate PO with a specific Rust binding path and proof obligation.
If REMOVE: each spec needs a documented removal action in the proof plan.

**Severity**: BLOCKER — State 5 cannot proceed without this decision.

---

### F-ORPHAN-DECISION-MISSING

**Code**: `E_PROOF_OBLIGATION_UNDERSPECIFIED`

**Artifact**: `proof-obligations.planned.jsonl` PO-008

**Message**: PO-008 combines all 5 orphaned specs into a single obligation with a combined `&&` command. This provides no per-spec granularity. If the bead owner decides BIND, each spec needs its own PO with a specific Rust binding path. If REMOVE, each spec needs a documented removal action. The current combined PO is insufficient for tracking and verification.

**Required Fix**: Split PO-008 into 5 separate POs (one per orphaned spec), each with:
- Specific artifact target (single file)
- Specific command (single verus invocation)
- Specific BIND or REMOVE resolution
- Specific expected evidence

---

### F-SCHEMA-ALIAS-FIELD

**Code**: `E_SCHEMA_ALIAS_FIELD`

**Artifact**: `proof-obligations.planned.jsonl`

**Message**: Uses `assumptions` (invalid legacy alias) instead of required `model_bounds` field per `proof-obligation/v1` schema. Uses `artifact` instead of required `target` field. These are not equivalent — `assumptions` and `artifact` are rejected by schema validators and cause downstream tooling failures.

**Required Fix**: Rename field `assumptions` → `model_bounds`; rename field `artifact` → `target`.

---

### F-LANE-WEAK-RATIONALE

**Code**: `E_LANE_DECISION_WEAK`

**Artifact**: `verifier-lane-decisions.jsonl` (multiple rows)

**Message**: Uses `rationale` field name instead of required `decision_reason` per `verifier-lane-decision/v1` schema. Field naming inconsistency may cause downstream schema validation failures.

**Required Fix**: Rename all `rationale` fields to `decision_reason` in `verifier-lane-decisions.jsonl`.

---

### F-WAIVER-SCHEMA-MISSING

**Code**: `E_SCHEMA_ALIAS_FIELD`

**Artifact**: `waiver-candidates.jsonl`

**Message**: Waiver candidates lack `schema_version` and `behavior_affecting` fields required by `waiver-candidate/v1` schema. The `reviewer_status` field should be `review_status`. These are not optional fields — they are mandatory for formal-waiver generation at State 12.

**Required Fix**: Add `schema_version: "waiver-candidate/v1"`, `behavior_affecting: false` to each waiver row. Rename `reviewer_status` to `review_status`.

---

## Positive Findings

- Seeds 001-007 are well-scoped with concrete bounds, reasonable assumptions, and clear resolution paths
- Verus lane is correctly identified as required for all 8 seeds
- Defense-in-depth lanes (TLA+, Kani, Flux, Loom, Miri) are correctly marked `not_applicable` with evidence-based rationales
- proptest and cargo-fuzz are correctly marked `blocked_tooling` (wrong tool class for Verus specs)
- G1 (Seed-005 Unknown variant) is properly documented with two resolution options (Option A: prove unreachable, Option B: add Unknown variant)
- G2 and G3 are properly documented with clear fix requirements
- Trusted base plan correctly identifies 5 trusted assumptions (T1-T5) and 4 gaps (G1-G4)
- Traceability matrix correctly maps all 8 seeds to requirements

---

## Verdict Summary

| Seed | Obligation | Status |
|------|------------|--------|
| 001 | step_state_machine vacuum fix | ACCEPTED |
| 002 | signals_invariant vacuum fix | ACCEPTED |
| 003 | signals_try_take vacuum fix | ACCEPTED |
| 004 | run_loop_termination vacuum fix | ACCEPTED |
| 005 | journal_trace mismatch fix | ACCEPTED (G1 documented) |
| 006 | budget_bounded mismatch fix | ACCEPTED (G2 documented) |
| 007 | idempotency_replay_tracker mismatch fix | ACCEPTED (G3 documented) |
| 008 | 5 orphaned specs BIND/REMOVE | **REJECTED — G4 BLOCKER** |

---

## STATUS: REJECTED

**Reason**: G4 blocker requires bead owner decision before State 5 can proceed. Proof obligations for seeds 001-007 are accepted. Seed-008 (orphaned specs) is rejected as PO-008 is underspecified and requires a BIND or REMOVE decision from bead owner Lewis.

**Next Action**: Bead owner must decide BIND or REMOVE for each of the 5 orphaned specs. Then proof-planner must resplit PO-008 into 5 separate POs and update all schema field names before State 5 can begin.

---

## Reviewer Sign-off

```
reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: ppr-vb-rqmw-2026-05-22
review_state: 4
artifacts_reviewed: proof-strategy.md, verifier-lane-decisions.jsonl, proof-obligations.planned.jsonl, proof-coverage-matrix.md, trusted-base-plan.md, waiver-candidates.jsonl, traceability-matrix.jsonl
lanes_reviewed: 64 (8 seeds × 8 verifiers)
accepted_lanes: 63
rejected_lanes: 1 (vld-008/PO-008 — G4 blocker)
findings: 5
critical_blockers: 1 (F-G4-BLOCKER)
STATUS: REJECTED
```
