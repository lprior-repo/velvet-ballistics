# Proof Plan Review Input - vb-qi37.12.2

STATUS: READY_FOR_PROOF_REVIEW

## Reviewer Focus

State 3 narrowed R5. Reject any proof/test obligation that demands exact runtime/storage source identity from unit `ResumeError::JournalAppendFailed`. Accept obligations that prove semantic typed fallback, deterministic conversion, no stale-source theft, no false success, retry-state restoration, and semver-compatible public API.

## Skill Citation

This plan cites `/home/lewis/.claude/skills/proof-planner/SKILL.md` and applies `planner_not_writer`, `traceability_required`, `mandatory_verification_gate`, `anti_hallucination`, and `schema.obligation_row`.

## Contract Inputs

- `contract.md`: `STATUS: CONTRACT_NARROWED`; R5a/R5b permit unit `JournalAppendFailed` as semantic fallback; R5c requires source detail only where a public carrier/API binds it; R5d forbids ambient/stale source theft; R5e requires deterministic fallback.
- `traceability-matrix.jsonl`: R1-R5 map to primary obligations and test/verification lanes.
- `proof-obligations.jsonl`: primary rows already reflect narrowed R5 and are sufficient.
- `state3-contract-repair-report.md`: exact source binding through unit `JournalAppendFailed` is impossible without semver break or fake side channel.

## Planned Obligation Summary

- `PO-R1-NO-DISCARD-001`: R1 typed durable-write failure propagation; focused test command; owner_state 8; rerun_from 8.
- `PO-R2-NO-FALSE-RESUMED-001`: R2/POST-001/INV-001 no `Ok(Resumed)` on drive or append failure; focused test command; owner_state 8; rerun_from 8.
- `PO-R3-RESTORE-RESUMABLE-001`: R3/POST-002/INV-002 failed `Resumed` append restores `Resumable`; focused test command; owner_state 8; rerun_from 8.
- `PO-R4-NOT-RESUMABLE-SHAPE-001`: R4/POST-003 `NotResumable` carries `run_id` and `current_state`; focused test/API shape command; owner_state 8; rerun_from 8.
- `PO-R5-DETERMINISTIC-FALLBACK-001`: R5a/R5b/R5e deterministic unit fallback with no claimed source identity; focused test command; owner_state 8; rerun_from 8.
- `PO-R5-NO-AMBIENT-SOURCE-001`: R5d/INV-003 no globals/thread locals/task locals/cached stale source side channels; clippy/static review command; owner_state 10; rerun_from 10.
- `PO-R5-SOURCE-ONLY-WHEN-CARRIED-001`: R5c source assertions only through public carrier/source chain or explicit non-ambient API; focused test/review command; owner_state 8; rerun_from 8.
- `PO-API-SEMCVER-001`: INV-004 semver-compatible unit variant remains valid; semver command; owner_state 10; rerun_from 10.
- `PO-TLA-RESUME-WORKFLOW-001`: planned TLA waiver for bounded workflow model; no executable `specs/vb_qi37_12_2_resume.tla`/`.cfg` artifacts exist in the isolated workspace and State 4 cannot create proof artifacts. Compensating evidence is required from the focused test/static/API obligations for no false success, restore-on-failed-append, deterministic fallback, no source claim without carrier, no ambient source side channel, and semver compatibility; owner_state 4; rerun_from 4.

## Removed Stale Demand

- `PO-SOURCE-PRESERVE-001` removed from planned obligations because it required `ResumeError::JournalAppendFailed` unit variant to preserve `RuntimeError::StorageJournalAppend` source identity. State 3 declares that impossible under semver-compatible unit shape.

## Discovery Evidence For Review

- Workspace command confirmed isolated path: `/home/lewis/src/vb-qi37-12-2`.
- Required inputs exist: `.beads/vb-qi37.12.2/contract.md`, `.beads/vb-qi37.12.2/traceability-matrix.jsonl`, `.beads/vb-qi37.12.2/delivery-scope.jsonl`.
- Scoped safety/state discovery found runtime state transition and retry-relevant logic in delivery files, plus `#![forbid(unsafe_code)]` in source/test files.
- Scoped verifier discovery found no existing Kani/Loom/proptest/fuzz/TLA/Miri markers in touched source files and no `specs/vb_qi37_12_2_resume.tla`/`.cfg` model artifacts in the isolated workspace. The TLA row is now a concrete planned waiver, not completed evidence.

## Review Questions

- Confirm planned IDs exactly mirror primary IDs in `proof-obligations.jsonl`.
- Confirm no planned row requires per-error source identity from unit `JournalAppendFailed`.
- Confirm `PO-TLA-RESUME-WORKFLOW-001` has a concrete valid waiver with owner, modeling limitation, compensating evidence, expiry, and follow-up trigger.
