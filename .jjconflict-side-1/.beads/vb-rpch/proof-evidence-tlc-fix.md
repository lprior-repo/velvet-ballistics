# Proof Evidence — TLC Fix Pass for `vb-rpch`

Status: **PARTIAL / PENDING_FORMAL_EXECUTION**.

## Tool discovery

Command run:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && command -v tlc && command -v java && tlc -version || true
```

Observed output summary:

- `tlc`: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`
- `java`: `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java`
- TLC banner: `TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)`
- `tlc -version` is not accepted by this wrapper and prints usage; this is tool behavior, not a proof failure.

## Commands actually run

### Smoke TLC — `TLC-FIX-005`

Command:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && tlc -config specs/tla/RecoveryReplayFull-smoke.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-smoke.tlc.log 2>&1
```

Raw output artifact: `evidence/specs/RecoveryReplayFull-smoke.tlc.log`

Result summary from raw log:

- Parsed and semantically processed `RecoveryReplayFull.tla`.
- `Model checking completed. No error has been found.`
- States generated: `5883676`
- Distinct states found: `505140`
- States left on queue: `0`
- Complete graph depth: `9`
- Runtime: `11s`

Smoke bounds:

- `RunId = {1}`
- `StepId = {1}`
- `ActionId = {1}`
- `Attempt = {1}`
- `EnabledEventTypes =` all modeled event types
- `MAX_SEQ = 3`
- `MAX_EVENTS = 3`

Checked invariants:

- `TypeOK`
- `TailCausalAfterSnapshot`
- `ReplaySeqOrder`
- `OnlyIncompleteRuns`
- `NoResolvedReExecution`
- `DigestVerificationOrder`

### Non-vacuity TLC — `TLC-FIX-007`

Command:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && tlc -config specs/tla/RecoveryReplayFull-nonvacuity.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity.tlc.log 2>&1
```

Raw output artifact: `evidence/specs/RecoveryReplayFull-nonvacuity.tlc.log`

Result summary from raw log:

- Parsed and semantically processed `RecoveryReplayFull.tla`.
- Expected result: `Error: Invariant NotAllNonVacuityWitnessesReached is violated.`
- This is the intended reachability witness: TLC found a state where all modeled non-vacuity predicates in `AllNonVacuityWitnessesReached` hold.
- Witness includes `RunAccepted`, `ActionCompleted`, `RunFinished`, `snapshot_seq = 0`, and `recovered_runs = {2}`.
- States generated before witness: `477860`
- Distinct states found before witness: `82285`
- Search depth at witness: `6`

Non-vacuity bounds:

- `RunId = {1, 2}`
- `StepId = {1}`
- `ActionId = {1}`
- `Attempt = {1}`
- `EnabledEventTypes = {"RunAccepted", "ActionCompleted", "RunFinished"}`
- `MAX_SEQ = 4`
- `MAX_EVENTS = 3`

### Evidence/source sync — `TLC-FIX-008`

Command:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && cmp -s specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && cmp -s specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg && cmp -s specs/tla/RecoveryReplayFull-smoke.cfg evidence/specs/RecoveryReplayFull-smoke.cfg && cmp -s specs/tla/RecoveryReplayFull-nonvacuity.cfg evidence/specs/RecoveryReplayFull-nonvacuity.cfg > evidence/specs/RecoveryReplayFull-sync.cmp.log 2>&1
```

Raw output artifact: `evidence/specs/RecoveryReplayFull-sync.cmp.log`

Result summary: command exited successfully and produced an empty log, meaning compared source/evidence proof artifacts are byte-identical for the four checked files.

## Pending formal executions

`PENDING_FORMAL_EXECUTION` for formal-verifier:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla
```

Expected raw output location after formal-verifier: `evidence/specs/RecoveryReplayFull.tlc.log`.

The smoke and non-vacuity commands above were run by proof-writer. The full primary cfg was not run to completion by proof-writer because the configured bounds are intentionally much larger and may require deep execution discipline.

## RecoveryErrorExhaustive caveat

`RecoveryErrorExhaustive` is **scoped partial/pending**. `ErrorDomain` names all variants, but this model only has causal transitions for digest mismatch variants via `CheckWorkflowDigest`/`CheckIrDigest`, plus direct `RecordError(NoneError)`. Other variants are not causally generated. Membership in `TypeOK` is not reachability proof.
