# Contract Verification Review Request

**Bead**: vb-qi37.3.1
**Artifact set**: contract.md, lean-contract.md, verification-layers.md, proof-obligations.jsonl, traceability-matrix.jsonl, martin-fowler-tests.md
**Review requested by**: State 1 contract synthesizer
**Reviewer required**: Independent contract-verification-reviewer (black-hat reviewer)

## Summary of Synthesis

This bead is **verification-only** for cross-run `CollectPaginationState` isolation. No production code changes were made. The contract proves:

1. **Table isolation**: `CollectStates` keyed by `(RunId, SlotIdx)` — cross-run entries cannot collide
2. **Per-run ownership**: each `RunState` owns its own `CollectStates` — passed explicitly to `drive_deterministic_full`
3. **Evidence isolation**: `drive_deterministic_full` captures evidence using `run.run_id()`

## Reviewer Task

Evaluate whether:
1. The three-layer isolation proof in `contract.md` is complete and correct
2. All 20 proof obligations in `proof-obligations.jsonl` are accurately described
3. The `lean-contract.md` waiver for Lean is justified (structural HashMap property)
4. The `verification-layers.md` layer assignments are appropriate or missing
5. The `martin-fowler-tests.md` Given-When-Then scenarios cover all acceptance criteria
6. The `traceability-matrix.jsonl` correctly maps all clauses to tests and evidence

## Exit Criteria for Reviewer

Write `contract-verification-review.md` with:
- `STATUS: APPROVED` if all artifacts are correct and complete
- `STATUS: REJECTED` if any artifact requires correction

## Artifacts Under Review

| File | Lines | Purpose |
|------|-------|---------|
| `contract.md` | 141 | Three-layer isolation proof with acceptance criteria mapping |
| `lean-contract.md` | 57 | Lean waiver rationale and structural proof justification |
| `verification-layers.md` | 82 | Layer assignment for all 20 clauses |
| `proof-obligations.jsonl` | 19 | One JSON object per contract clause |
| `traceability-matrix.jsonl` | 29 | Clause-to-test/proof/tool mapping |
| `martin-fowler-tests.md` | 253 | 18 Given-When-Then scenarios |
