# Proof Review — TLC Fix Round 2 for `vb-rpch`

Provenance:
- Reviewer agent: `proof-reviewer`
- Review date: 2026-05-24
- Workspace: `/home/lewis/src/vb-jpq7-jj-fix`
- Bead: `vb-rpch`
- Review scope: adversarial review of TLC fix round 2 planner, proof-writer, and formal-verifier artifacts only.
- Rubrics loaded: `proof-reviewer`; `tla-plus`.
- Files written by reviewer: `.beads/vb-rpch/proof-review-tlc-fix-round2.md`, `.beads/vb-rpch/proof-findings-tlc-fix-round2.jsonl`, `.beads/vb-rpch/proof-repair-guide-tlc-fix-round2.md`.
- JSONL parse check: `proof-obligations.tlc-fix-round2.planned.jsonl` OK 20 rows; `verifier-lane-decisions.tlc-fix-round2.jsonl` OK 8 rows; `trusted-base-ledger.tlc-fix-round2.jsonl` OK 8 rows; `verification-ledger.tlc-fix-round2.jsonl` OK 20 rows.
- Raw-log inspection: primary and all required witness logs under `evidence/specs/*.tlc.log` were inspected; no proof artifacts, production Rust, tests, harnesses, or configs were repaired.

## Findings

### CRITICAL — `TLC-R2-004` / `TLC-R2-015` / `TLC-R2-016`: ABI and policy mismatch witnesses are unconditional error injectors, not causal mismatch transitions

Artifact/evidence:
- `specs/tla/RecoveryReplayFull.tla` lines 216-222 define `CheckActionAbiDigest` and `CheckPolicyDigest` as `0 # 1` followed by `RecordErrorWithUnchangedStage(...)`.
- `specs/tla/RecoveryReplayFull-error-action-abi-mismatch.cfg` lines 7-9 sets `EnabledEventTypes = {}`, `MAX_SEQ = 1`, `MAX_EVENTS = 1`.
- `evidence/specs/RecoveryReplayFull-error-action-abi-mismatch.tlc.log` lines 20-38 shows `ActionAbiMismatch` reached from the empty initial state by `CheckActionAbiDigest`, with no modeled expected/found ABI input.
- `.beads/vb-rpch/proof-obligations.tlc-fix-round2.planned.jsonl` lines 15-16 required abstract expected/found mismatch witnesses and a GAP-3 caveat, not bare nondeterministic error injection.

The reports correctly caveat these as abstract typed error witnesses, but the model does not even encode the planned abstraction. `0 # 1` is a tautological guard. The resulting counterexamples prove only that TLC can take an always-enabled action that writes an error string. That does not close causal reachability for these variants.

### HIGH — `TLC-R2-009` / `TLC-R2-017`: non-idempotent replay blocking is still weakened to “completed event exists”

Artifact/evidence:
- `specs/tla/RecoveryReplayFull.tla` lines 224-227 define `DetectNonIdempotentResolved` as any `ActionCompleted` or `ActionFailed` event followed by `NonIdempotentActionBlocked`.
- `specs/tla/RecoveryReplayFull-nonvacuity-no-reexecution.cfg` lines 7-9 enables only `ActionCompleted` with `MAX_EVENTS = 1`.
- `evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log` lines 31-40 reaches the witness after a single `ActionCompleted`; there is no later same-action schedule candidate.
- `evidence/specs/RecoveryReplayFull-error-non-idempotent-action-blocked.tlc.log` lines 31-47 reaches `NonIdempotentActionBlocked` after a single `ActionCompleted`, again with no replay candidate.
- `.beads/vb-rpch/proof-obligations.tlc-fix-round2.planned.jsonl` lines 9 and 17 explicitly require a completed/failed action and a later same action/step schedule candidate that is blocked or raises the error.

This repeats the prior semantic weakness in a split witness. The invariant `NoResolvedReExecution` may pass, but the non-vacuity/error witness does not exercise the transition shape that the contract cares about.

### HIGH — `TLC-R2-014`: corrupt snapshot is modeled as “no snapshot loaded,” not a corrupt marker/mismatch input

Artifact/evidence:
- `specs/tla/RecoveryReplayFull.tla` lines 212-214 define `LoadCorruptSnapshot` as `snapshot_seq = -1` followed by `CorruptSnapshot`.
- `evidence/specs/RecoveryReplayFull-error-corrupt-snapshot.tlc.log` lines 20-38 reaches `CorruptSnapshot` from the initial state where `snapshot_seq = -1` and `journal = <<>>`.
- `.beads/vb-rpch/proof-obligations.tlc-fix-round2.planned.jsonl` line 14 allowed an abstract corrupt marker or snapshot/run mismatch input; `.beads/vb-rpch/trusted-base-ledger.tlc-fix-round2.jsonl` line 6 says TLC proves a typed error from a modeled corrupt input.

The implementation caveat is honest that payload decoding is out of scope, but the model has no corrupt input at all. It aliases the initial absence of a snapshot with corruption. That is too weak for the ledger claim “modeled corrupt input.”

## Checks that passed review

- Primary bounded TLC evidence is real for the stated finite abstraction: `evidence/specs/RecoveryReplayFull.tlc.log` lines 22-28 show `Model checking completed. No error has been found.`, `36532738 states generated`, `4088199 distinct states found`, `0 states left on queue`, depth `11`.
- The primary cfg is clearly bounded: `specs/tla/RecoveryReplayFull.cfg` lines 3-9 set singleton run/step/action/attempt and `MAX_SEQ = 3`, `MAX_EVENTS = 3`; reports limit the claim to this finite abstraction.
- `DigestVerificationOrder` is materially improved: `digest_stage` is in `TypeOK`, `Init`, and `vars` (`specs/tla/RecoveryReplayFull.tla` lines 70-89); `CheckIrDigest` requires workflow checked first (lines 194-205); the invariant states `IrChecked => WorkflowChecked` (lines 265-267).
- Witness logs for `TLC-R2-005` through `TLC-R2-019` are present and contain the expected invariant violation names. Presence of an expected violation is not enough where the underlying modeled action is non-causal, as noted above.
- Evidence sync is credible: the reviewer re-ran the planned `cmp -s` source/evidence comparison and observed exit `0`.
- Optional large-stress cfg is not represented as proof: `verification-ledger.tlc-fix-round2.jsonl` line 2 classifies it `WAIVED`, and `.beads/vb-rpch/formal-verification-report-tlc-fix-round2.md` lines 92-96 says it was not run and no proof claim depends on it.

## Verdict

Round 2 fixed the exploding primary TLC cfg and the previous digest-order tautology, but it still does not close the causal error-reachability obligation. Several required witnesses are satisfied by always-enabled or under-modeled error setters rather than by the planned abstract inputs. This TLC round cannot advance to bridge/evidence packaging until the causal witness semantics are repaired and re-executed.

STATUS: REJECTED
