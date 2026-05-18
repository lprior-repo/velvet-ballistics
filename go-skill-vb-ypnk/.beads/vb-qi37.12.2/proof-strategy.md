# Proof Strategy - vb-qi37.12.2

STATUS: PLANNED

## Scope

State 4 proof planning is repaired after State 3 narrowed R5. This plan writes planning artifacts only and does not edit production code, tests, proof code, models, harnesses, specs, dependencies, or CI configuration.

Allowed planning outputs:

- `.beads/vb-qi37.12.2/proof-strategy.md`
- `.beads/vb-qi37.12.2/proof-plan-review-input.md`
- `.beads/vb-qi37.12.2/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.12.2/formal-waivers.jsonl`

Delivery scope from `.beads/vb-qi37.12.2/delivery-scope.jsonl`:

- Crate: `vb_runtime`
- Files: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`, `crates/vb_runtime/src/shard/types.rs`, `crates/vb_runtime/src/error/conversions.rs`, `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs`
- APIs: `Shard::handle_resume`, `ResumeError`, `RuntimeError::StorageJournalAppend`
- Risk tags: `reliability`, `storage-journal`, `error-propagation`, `release-plan`

## Proof-Planner Skill Citation

Read and applied `/home/lewis/.claude/skills/proof-planner/SKILL.md`:

- `planner_not_writer`: State 4 must write only planning artifacts under `.beads/<bead-id>/`.
- `traceability_required`: every obligation maps to a requirement, contract clause, invariant, or explicit risk.
- `mandatory_verification_gate`: scoped discovery must run or be recorded before finalizing.
- `anti_hallucination`: do not invent verifier availability, files, pass results, or coverage.
- `schema.obligation_row`: rows must include IDs, requirement mapping, risk, verifier, artifact, command, expected evidence, owner state, rerun source, status, and waiver data where applicable.

## State 3 R5 Narrowing

R5 no longer requires exact per-error source identity from the semver-compatible unit variant `ResumeError::JournalAppendFailed`. That exact binding is impossible without a semver break or fake side channel because the unit variant has no public source carrier.

R5 now requires:

- No false success: affected resume failures return `Err`, not `Ok(Resumed)`.
- Failed `Resumed` append restores `RuntimeState::Resumable` for retry.
- Fresh unit append failures convert deterministically to `ResumeError::JournalAppendFailed` when no public source carrier exists.
- No hidden stale-source theft from global, task-local, thread-local, cached, or otherwise ambient state.
- Exact source detail only where the public error shape/source chain or an owner-approved explicit non-ambient API actually carries and binds it.
- Semver compatibility for the public unit variant unless the owner explicitly chooses a breaking API change.

Removed stale obligation:

- `PO-SOURCE-PRESERVE-001` is invalid after State 3 because it demanded `RuntimeError::StorageJournalAppend` identity from unit `ResumeError::JournalAppendFailed`.

## Discovery Evidence

Commands run from isolated workspace `/home/lewis/src/vb-qi37-12-2` only:

- `pwd -P && test -s ".beads/vb-qi37.12.2/contract.md" && test -s ".beads/vb-qi37.12.2/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.12.2/delivery-scope.jsonl"`
- Output: `/home/lewis/src/vb-qi37-12-2`; all required planning inputs exist.
- Scoped discovery pattern for safety/concurrency/state/error risks over delivery source files found state-transition/runtime-state usage in `chunk_001.rs`, `#![forbid(unsafe_code)]` and queue/state declarations in `types.rs`, and conversion handling in `conversions.rs`.
- Scoped discovery over `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs` found existing assertions for `JournalAppendFailed`, retry state, no stale thread-local source behavior, and test-only `expect`/`panic` usage.
- Verification-artifact discovery found no existing Kani/Loom/proptest/fuzz/TLA/Miri markers in touched source files; `types.rs` and the focused test use `#![forbid(unsafe_code)]`.

## Verifier Strategy

Use the cheapest verifier lane that matches each risk:

- Focused integration tests for observable error propagation, no false success, restore-on-failed-append, deterministic unit fallback, and `NotResumable` shape.
- Focused/source clippy plus static scan for no ambient/stale source side-channel implementation.
- API compatibility check for semver-compatible unit `JournalAppendFailed`.
- `PO-TLA-RESUME-WORKFLOW-001` is explicitly waived in this planning repair because no executable `specs/vb_qi37_12_2_resume.tla`/`.cfg` artifacts exist in the isolated workspace and State 4 cannot create proof artifacts. The waiver is not a proof result; it is a planned waiver with owner, modeling limitation, compensating focused tests/static/API evidence, expiry, and follow-up trigger in `formal-waivers.jsonl` and the obligation row.
- Kani, Verus, Flux, Loom, Miri, fuzz, and dependency/supply-chain proof lanes are not required by current scope because this repair is public error semantics and single-shard state workflow, has no unsafe change, no concurrency primitive change, no dependency change, and no extracted pure proof kernel.

## Primary/Planned ID Validation

Primary IDs in `.beads/vb-qi37.12.2/proof-obligations.jsonl` are sufficient for the narrowed State 3 contract and are mirrored by `.beads/vb-qi37.12.2/proof-obligations.planned.jsonl`:

- `PO-R1-NO-DISCARD-001`
- `PO-R2-NO-FALSE-RESUMED-001`
- `PO-R3-RESTORE-RESUMABLE-001`
- `PO-R4-NOT-RESUMABLE-SHAPE-001`
- `PO-R5-DETERMINISTIC-FALLBACK-001`
- `PO-R5-NO-AMBIENT-SOURCE-001`
- `PO-R5-SOURCE-ONLY-WHEN-CARRIED-001`
- `PO-API-SEMCVER-001`
- `PO-TLA-RESUME-WORKFLOW-001` (planned waiver, no completed-result claim)

No State 3 repair is required for the narrowed R5 semantics. Planned rows remove the impossible unit-source-identity requirement and cover deterministic fallback, no stale-source theft, no false success, restore-on-failed-append, and semver compatibility. The only State 6 rejection addressed here is the invalid optional TLA row; it is now a concrete planned waiver rather than an unwaived optional proof/protocol obligation.

## Handoff

Next owner state: State 7 proof-reviewer to review this planned matrix, then State 8 test-writer for focused tests and State 10 implementation/static/API checks as assigned by each row.
