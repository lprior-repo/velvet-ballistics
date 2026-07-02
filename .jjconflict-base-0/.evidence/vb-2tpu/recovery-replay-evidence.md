# vb-2tpu RecoveryReplayFull TLC evidence

## Scope and split

`RecoveryReplayFull` previously timed out because it combined arbitrary journal-event generation, large `MAX_SEQ=100` / `MAX_EVENTS=20` bounds, digest-stage nondeterminism, and error-coverage liveness in one state space. This repair splits the obligation honestly:

- `specs/tla/RecoveryReplayFull.tla` / `.cfg`: safety obligations for replay order, snapshot-tail causality, incomplete-run discovery, resolved-action non-reexecution, digest verification order, and deadlock freedom.
- `specs/tla/RecoveryReplayErrors.tla` / `.cfg`: obligation-specific liveness model for exhaustive `RecoveryError` coverage.

## Commands and raw evidence

All commands were run from `/home/lewis/src/vb-2tpu-recovery-replay-tla-gpt55` with `TMPDIR` and `JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=...` under `.evidence/vb-2tpu/`; no `/tmp` or `/tmp/opencode` path was used.

| Result | Command | Raw log |
|---|---|---|
| PASS | `TMPDIR="$PWD/.evidence/vb-2tpu/java-tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/.evidence/vb-2tpu/java-tmp" timeout 300 tlc -metadir ".evidence/vb-2tpu/metadir/full" -config "specs/tla/RecoveryReplayFull.cfg" "specs/tla/RecoveryReplayFull.tla"` | `.evidence/vb-2tpu/logs/tlc-RecoveryReplayFull.log` |
| PASS | `TMPDIR="$PWD/.evidence/vb-2tpu/java-tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/.evidence/vb-2tpu/java-tmp" timeout 300 tlc -metadir ".evidence/vb-2tpu/metadir/errors" -config "specs/tla/RecoveryReplayErrors.cfg" "specs/tla/RecoveryReplayErrors.tla"` | `.evidence/vb-2tpu/logs/tlc-RecoveryReplayErrors.log` |
| EXPECTED FAIL | `tlc -metadir .evidence/vb-2tpu/metadir/nonvacuity-seq -config .evidence/vb-2tpu/nonvacuity/RecoveryReplayBadSeq.cfg .evidence/vb-2tpu/nonvacuity/RecoveryReplayBadSeq.tla` | `.evidence/vb-2tpu/logs/tlc-nonvacuity-badseq.log` |
| EXPECTED FAIL | `tlc -metadir .evidence/vb-2tpu/metadir/nonvacuity-errors -config .evidence/vb-2tpu/nonvacuity/RecoveryReplayErrorsStuck.cfg .evidence/vb-2tpu/nonvacuity/RecoveryReplayErrorsStuck.tla` | `.evidence/vb-2tpu/logs/tlc-nonvacuity-errors-stuck.log` |

## TLC outcomes

- `RecoveryReplayFull`: TLC exit 0; `73058 states generated, 6147 distinct states found, 0 states left on queue`; complete search depth `12`; no error found.
- `RecoveryReplayErrors`: TLC exit 0; liveness checked; `9235 states generated, 2305 distinct states found, 0 states left on queue`; complete search depth `10`; no error found.
- Non-vacuity bad-sequence probe: TLC found `Invariant ReplaySeqOrder is violated` after appending `<<[seq |-> 1], [seq |-> 0]>>`.
- Non-vacuity stuck-error probe: TLC found `Temporal properties were violated` when `pending_errors` never decreases.

## Bounds, constraints, reductions

`RecoveryReplayFull.cfg` finite bounds:

- `RunId = {1}`
- `StepId = {1}`
- `ActionId = {1}`
- `Attempt = {1}`
- `MAX_SEQ = 2`
- `MAX_EVENTS = 2`
- generated event types are constrained in the model to the representative replay/snapshot terminal subset: `RunAccepted`, `ActionScheduled`, `ActionCompleted`, `RunFinished`, `RunCancelled`, `RunFailedEvent`.

No symmetry set was used. No state constraint was used. Deadlock checking remains enabled for both PASS runs. `RecoveryReplayErrors` uses an explicit `Done` stutter after all error variants are covered so terminal completion is not hidden behind disabled deadlock checking.

## Invariants/properties checked

`RecoveryReplayFull` checked:

- `TypeOK`
- `TailCausalAfterSnapshot`
- `ReplaySeqOrder`
- `OnlyIncompleteRuns`
- `NoResolvedReExecution`
- `DigestVerificationOrder`

`RecoveryReplayErrors` checked:

- `TypeOK`
- `EventuallyAllRecoveryErrorsCovered` under weak fairness for the error-recording action.

## Rust refinement map

- `journal` and event sequence order refine `FjallJournal::events_for_run` consumers in `crates/vb_storage/src/recovery/replay/core.rs:150-214` and summary/frame recovery in `crates/vb_storage/src/recovery/recover.rs:140-238`.
- `snapshot_seq` and `TailCausalAfterSnapshot` refine `recover_snapshot_plus_tail` checks in `crates/vb_storage/src/recovery/replay/core.rs:193-214` and `validate_tail_events_after_snapshot` / `snapshot_input_violation_to_error` in `crates/vb_storage/src/recovery/hydrate.rs:197-239`.
- `tracker` and `NoResolvedReExecution` refine `ActionReplayTracker` in `crates/vb_storage/src/recovery/types.rs:335-370` and replay code that records completed/failed actions.
- `recovered_runs` and `OnlyIncompleteRuns` refine `recover_all_incomplete_runs` in `crates/vb_storage/src/recovery/recover.rs:217-238`.
- `digest_level`, `workflow_verified`, and `ir_verified` refine `DigestCheck` and `verify_digests` ordering in `crates/vb_storage/src/recovery/types.rs:378-388` and `crates/vb_storage/src/recovery/recover.rs:83-101`.
- `RecoveryErrors` refines `RecoveryError` variants in `crates/vb_storage/src/recovery/types.rs:19-109`. The TLA split covers the nine variants in the original obligation; newer/extra Rust variants (`Journal`, slot-taint variants, `TerminalStateMismatch`) remain outside the original `RecoveryReplayFull` obligation and need separate beads if required by product scope.

## Liveness stance

The replay safety model is safety-only (`Spec == Init /\ [][Next]_vars`) with deadlock checking enabled. Error coverage is the only liveness claim and is isolated in `RecoveryReplayErrors`; it uses weak fairness on the error-recording action and TLC checked the temporal property over the complete finite state space.

## Non-vacuity checklist

- Invariant failure probe exists and fails for unordered replay sequence (`tlc-nonvacuity-badseq.log`).
- Temporal failure probe exists and fails for stuck error coverage (`tlc-nonvacuity-errors-stuck.log`).
- PASS models include non-empty reachable state spaces and non-trivial depths (`6147` and `2305` distinct states respectively).
- No symmetry, simulation-only evidence, or disabled deadlock checking was used as closure evidence.

## Residual limitations

The checked replay safety bound is intentionally small and representative. It proves the transition shape under one run/action/step/attempt and two journal events; it is not a proof over the original unbounded `MAX_EVENTS=20` / `MAX_SEQ=100` space. If broader combinatorial coverage is needed, create a follow-up model-reduction bead rather than relabeling this bounded evidence as exhaustive over large production cardinalities.
