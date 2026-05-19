# Proof Plan Review Input — vb-0sps (State 4, attempt 3-of-7)

## Review decision requested

Approve or reject this State 4 proof plan for `vb-0sps` generated-vs-IR BDD parity evidence gap `VB-BDD-CATALOG-007`. This is the proof-planner child delegate writing after the State 3 canonical ledger repair (exact clause rows for `PRE-003`, `PRE-004`, `PRE-005`, `INV-001`, `INV-004`, `INV-005`, `INV-006` are now present).

## Authority

- Dispatch manifest: `.beads/vb-0sps/dispatch-manifest-state4-after-contract-repair-attempt3.json`
- Delegate: `proof-planner`
- Attempt: 3-of-7
- Previous attempts: attempt 1 (INVALIDATED_DELEGATE_MISMATCH), attempt 2 (APPROVED_FOR_STATE5, but downstream State 5/6 rejected for TLA vacuity and missing canonical clause rows)
- Routing context: State 6 (proof-review + contract-verification-review) rejected with `rerun_from: State 3`. State 3 repair completed and approved. State 4 proof-planner now regenerates from repaired canonical ledger.

## Inputs consumed

```
.beads/vb-0sps/STATE.md
.beads/vb-0sps/contract.md
.beads/vb-0sps/domain-model-review.md
.beads/vb-0sps/tla-spec.md
.beads/vb-0sps/lean-contract.md
.beads/vb-0sps/verification-layers.md
.beads/vb-0sps/proof-obligations.jsonl         ← repaired canonical ledger
.beads/vb-0sps/traceability-matrix.jsonl
.beads/vb-0sps/proof-review.md                ← rejection evidence
.beads/vb-0sps/contract-verification-review.md ← rejection evidence
```

## Planned artifacts (proof-planner-owned, attempt 3)

```
.beads/vb-0sps/proof-strategy.md              (written)
.beads/vb-0sps/proof-obligations.planned.jsonl (written)
.beads/vb-0sps/proof-plan-review-input.md      (this file)
```

## Key changes from attempt 2

| Aspect | Attempt 2 | Attempt 3 |
|---|---|---|
| TLA status | `blocked_tooling` (no model) | `planned` (model exists, vacuity repair needed) |
| TLA commands | single monolithic `.cfg` | 5 split configs per tla-spec.md |
| Verus status | `blocked_tooling` | `waived` with full WAIVER-VERUS-ADAPTERS-001 metadata |
| Missing clause rows | absent from canonical ledger | now present in canonical ledger (State 3 repair) |
| TLA-DIVERGENCE-SANITY | absent as separate row | now explicit row with separate divergence sanity config |
| Split config existence | all 5 MISSING | all 5 MISSING (State 5 responsibility) |

## Scope containment check

- [x] Plan covers only `VB-BDD-CATALOG-007` generated-vs-IR parity
- [x] Explicitly rejects generated/maxperf release reactivation (INV-007 / NON-GOAL-001)
- [x] Bead closes BDD catalog evidence gap only; does not claim maxperf/release readiness
- [x] No production code or tests written in this state
- [x] No whole-fleet verification; focused commands only

## Coverage analysis

### Traceability matrix → planned obligations

- 19 TM clauses × 20 planned PO rows = all clauses covered
- `PRE-001`: BDD-PRE-001
- `PRE-002`: BDD-PRE-002
- `PRE-003`: WAIVER-VERUS-PRE-003
- `PRE-004`: TLA-PRE-004
- `PRE-005`: TLA-PRE-005 + BDD-POST-006 (shared)
- `POST-001`: BDD-POST-001 + WAIVER-VERUS-PRE-003 (shared)
- `POST-002`: WAIVER-VERUS-POST-002
- `POST-003`: TLA-POST-003
- `POST-004`: TLA-POST-004
- `POST-005`: TLA-POST-005 + TLA-DIVERGENCE-SANITY (shared)
- `POST-006`: BDD-POST-006 + TLA-INV-006 (shared)
- `POST-007`: BDD-POST-007
- `INV-001`: BDD-INV-001
- `INV-002`: WAIVER-VERUS-INV-002
- `INV-003`: WAIVER-VERUS-INV-003
- `INV-004`: TLA-INV-004
- `INV-005`: TLA-INV-005
- `INV-006`: TLA-INV-006 + TLA-PRE-005 (shared)
- `INV-007`: NON-GOAL-INV-007

### Status distribution

| Status | Count | Obligations |
|---|---|---|
| `planned` | 15 | BDD lanes (9), TLA lanes (6), NON-GOAL (1) |
| `waived` | 5 | WAIVER-VERUS-PRE-003, WAIVER-VERUS-POST-002, WAIVER-VERUS-INV-002, WAIVER-VERUS-INV-003, plus BDD-POST-001 portion (handled by BDD-POST-001 row) |

Note: `blocked_tooling` is NOT used for any lane. TLA is `planned` (model exists). Verus is `waived` (valid metadata).

## Reviewer checklist

1. **Scope containment:** Plan limits to BDD catalog gap only; no maxperf/reactivation claims.
2. **Canonical ledger alignment:** All 19 clause rows from `proof-obligations.jsonl` have corresponding planned obligation rows.
3. **TLA non-vacuity requirement:** No TLA waiver is claimed; vacuity repair is documented as a State 5 blocker; divergence sanity config is a separate obligation proving non-vacuity.
4. **Verus waiver validity:** `WAIVER-VERUS-ADAPTERS-001` metadata is complete: owner, reason, limitation, expiry, follow_up, compensating_evidence.
5. **Split config commands:** All 5 TLC commands are the exact split-config commands from `tla-spec.md`; not the monolithic config.
6. **JSONL validity:** `proof-obligations.planned.jsonl` is valid JSONL; each row has all required fields including `waiver: null` for non-waived rows.
7. **Risk tags:** All risk categories are assigned; no risk is silently omitted.
8. **Waived lanes explicit:** Lean/Aeneas/Hax waived per `lean-contract.md` (`THM-WAIVER-001`); not silently omitted.
9. **TLA-DIVERGENCE-SANITY as separate obligation:** The negative sanity config has its own obligation row with expected non-zero TLC exit and explicit refinement rationale.
10. **NO-VACUUM enforcement:** No proof obligation uses `blocked_tooling` to avoid an active required lane; TLA is `planned` (model exists), Verus is `waived` (valid metadata).

## Risk hotspots for reviewer attention

1. **TLA vacuity (LETHAL per State 6):** The existing model at `GeneratedIrParity.tla` uses `LockstepDo` which writes identical state to both sides. State 5 must replace with separate IR/generated transition relations. The proof-obligations.planned.jsonl documents this as a `blocker` on each TLA row.
2. **TLA split configs missing:** All 5 split config files are MISSING from the filesystem. State 5 must author them. The planned obligations reference them as required artifacts.
3. **TLA divergence sanity required:** The negative sanity config must exit non-zero with an expected violation. If it exits 0, the model is still vacuous.
4. **Verus waiver expiry:** `WAIVER-VERUS-ADAPTERS-001` expires when `compare_observed_runs`, `normalize_error`, or event-sequence adapters exist. State 6 reviewers must verify adapters do not already exist before approving the waiver.
5. **resumeQueue/sourceEmitted reachability:** State 6 review found `resumeQueue` starts empty and `sourceEmitted` starts FALSE with no transitions to make them non-empty/TRUE. State 5 must model reachable resume inputs and reachable supported-source-emission paths.
6. **Journal fields completeness:** `SameJournalPrefix` must compare all POST-005 fields (taint, action_id, retry, deadline, event, prompt/answer metadata, terminal event detail, typed failure fields).

## Recommended outcome

**Approve** if:
- JSONL validates without errors
- All 19 contract clauses have at least one planned obligation row
- TLA lanes are `planned` with vacuity blocker documented (not `blocked_tooling`)
- Verus lanes are `waived` with complete metadata (not `blocked_tooling`)
- No maxperf/release readiness claim appears anywhere
- Divergence sanity obligation is separate and expects non-zero TLC exit

**Reject** if:
- Any required contract clause is absent from the obligation ledger
- A required proof lane is silently marked `not_applicable` without a valid waiver
- JSONL fails to parse
- TLA is claimed as `blocked_tooling` when the model already exists (vacuity ≠ tooling absence)

## Blockers this state hands forward

1. **TLA vacuity repair (State 5):** Replace `LockstepDo` with separate IR/generated transition relations; prove `ObservationRefinesOracle` can fail.
2. **TLA split configs (State 5):** Author all 5 split config files; replace monolithic `.cfg`.
3. **TLA TLC completion (State 5):** All 5 TLC runs must complete; divergence sanity must exit non-zero with expected violation.
4. **Verus adapter existence (State 5/6):** Waiver expires when `compare_observed_runs`, `normalize_error`, or event-sequence adapters are added.
5. **resumeQueue/sourceEmitted reachability (State 5):** Model must have transitions that populate resumeQueue and set sourceEmitted TRUE on supported paths.
6. **Catalog closure (State 6):** `VB-BDD-CATALOG-007` must be updated before `deferred_follow_up_bead` is cleared.
