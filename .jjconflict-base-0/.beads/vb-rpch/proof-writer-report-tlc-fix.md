# Proof Writer Report — TLC Fix Pass for `vb-rpch`

Status: **PARTIAL / PENDING_FORMAL_EXECUTION**. I repaired proof/model artifacts only. I did not edit production Rust, tests, dependency files, CI, or review approvals.

## Obligations touched

- `TLC-FIX-001`: removed undeclared `Digest` cfg assignment; source/evidence cfgs reconciled.
- `TLC-FIX-002`: repaired `SetSnapshot` typing/reachability and explicit `snapshot_seq'`; removed invalid `SetSnapshot(0,0)` from `Next`.
- `TLC-FIX-003`: removed stale fake `Sort(s, less) == s`; retained explicit `BuildSeqFromIndices` construction.
- `TLC-FIX-004`: removed tautological `PROPERTY Spec` from source and evidence cfgs.
- `TLC-FIX-005`: added and ran bounded smoke cfg.
- `TLC-FIX-006`: primary full cfg repaired, but deep full execution remains `PENDING_FORMAL_EXECUTION`.
- `TLC-FIX-007`: added reachability/non-vacuity predicates and a bounded non-vacuity cfg; ran it and obtained the expected invariant violation witness for combined modeled antecedents.
- `TLC-FIX-008`: synced repaired source specs/cfgs to `evidence/specs/` and checked byte identity.

## Files changed

- `specs/tla/RecoveryReplayFull.tla`
- `specs/tla/RecoveryReplayFull.cfg`
- `specs/tla/RecoveryReplayFull-smoke.cfg`
- `specs/tla/RecoveryReplayFull-nonvacuity.cfg`
- `evidence/specs/RecoveryReplayFull.tla`
- `evidence/specs/RecoveryReplayFull.cfg`
- `evidence/specs/RecoveryReplayFull-smoke.cfg`
- `evidence/specs/RecoveryReplayFull-nonvacuity.cfg`
- `evidence/specs/RecoveryReplayFull-smoke.tlc.log`
- `evidence/specs/RecoveryReplayFull-nonvacuity.tlc.log`
- `evidence/specs/RecoveryReplayFull-sync.cmp.log`
- `.beads/vb-rpch/proof-writer-report-tlc-fix.md`
- `.beads/vb-rpch/proof-evidence-tlc-fix.md`
- `.beads/vb-rpch/trusted-base-ledger.tlc-fix.jsonl`

## Model repairs made

1. Added `EnabledEventTypes` as an explicit finite model bound. Full/smoke cfgs enable the original event set; non-vacuity cfg narrows event generation to a witness-oriented subset.
2. Removed cfg assignment `Digest = {0, 1, 2, 3}` because `Digest` is an operator definition in the module, not a declared constant.
3. Removed `PROPERTY Spec`; it was tautological under `SPECIFICATION Spec`.
4. Replaced invalid snapshot transition with `SetSnapshot(run, seq)` where `run \in RunId`, `seq \in EventSeqNum`, `snapshot_seq' = seq`, and all other variables are explicitly unchanged. The action requires existing journal events to be tail-causal relative to the selected snapshot seq.
5. Strengthened `AppendEvent` with monotonic seq, tail-after-snapshot, no-resolved-reexecution, and terminal-run removal from `recovered_runs` so declared invariants are not immediately false by unconstrained model actions.
6. Removed fake identity `Sort`; `BuildSeqFromIndices` is the remaining explicit sequence construction.
7. Added non-vacuity predicates for modeled antecedents. Historical note: the original `RecoveryErrorExhaustiveScopedPartial` wording from this round is superseded by `proof-writer-report-tlc-fix-round3.md`; the current TLA model no longer contains that stale partial-exhaustiveness operator.

## Brutal residual weaknesses

- `RecoveryErrorExhaustive` remains **not proven**. The model can directly represent `last_error` membership and can reach modeled digest mismatch errors, but most variants in `ErrorDomain` have no causal transition. This pass records that as scoped partial/pending, not a pass.
- The non-vacuity run proves a combined witness for modeled antecedents by expected violation of `NotAllNonVacuityWitnessesReached`; it does **not** prove full semantic adequacy of each invariant.
- `EnabledEventTypes` is a model reduction knob. The full cfg uses all modeled event types; the non-vacuity cfg intentionally narrows it. This is a trusted bound/reduction, not production behavior.
- Full primary bounded model (`MAX_SEQ=100`, `MAX_EVENTS=20`, two runs, three steps, two actions, two attempts) was **not** run by me to completion. Existing approvals remain stale until formal-verifier captures fresh raw output.
