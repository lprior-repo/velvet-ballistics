# TLA+ Temporal Model Plan: Atomic Accepted Run Admission

## Boundary

- Temporal behavior: strict accepted-run admission from validated input through staging, commit, acknowledgement, injected failure, and restart/readback.
- Rust/core behavior excluded and handled by Verus/Kani/proptest: digest equality, artifact envelope discriminator, sequence equality predicate, deterministic index derivation, typed error taxonomy.
- External systems abstracted: Fjall batch commit as an atomic action that either makes all staged records visible or no accepted-run visibility; wall-clock time excluded.

## TLA+-Owned Clauses

- POST-001: all required records visible after success.
- POST-002 and INV-002: commit strictly precedes acknowledgement.
- POST-005 and INV-001: injected failure leaves no accepted-run partial visibility.
- INV-005: index visibility is consistent with required records.
- INV-007: restart/readback observes only durable committed state.

## Model Shape

- Planned module/model path: `verification/tla/AtomicAcceptedRunAdmission.tla`.
- Planned config path: `verification/tla/AtomicAcceptedRunAdmission.cfg`.
- Variables: `runs`, `source`, `artifact`, `header`, `acceptedEvent`, `indexes`, `staged`, `committed`, `acked`, `runtimeAllocated`, `failures`, `restarted`, `readback`.
- Init action: `Init` with no committed accepted runs and no acknowledgements.
- Next/actions: `ValidateInput`, `StageSource`, `StageArtifact`, `StageHeader`, `StageAcceptedEvent`, `StageIndexes`, `CommitBatch`, `FailBeforeCommit`, `FailCommit`, `Acknowledge`, `AllocateRuntime`, `Restart`, `Readback`, `RejectInvalidArtifact`.
- State constraints: finite sets for runs, workflows, record kinds, indexes, and failure points.
- Symmetry sets: run ids and workflow ids may be symmetric under bounded TLC models.
- Bounded model limits: at least two runs, two workflows, all required record kinds, and failure points before each staging step plus commit failure.

## Properties

- Safety invariants:
  - `NoAckBeforeCommit`: `acked[r] => committed[r]`.
  - `AllRecordsOrNoAcceptedRun`: accepted visibility for `r` iff source, artifact, header, accepted event, and indexes are all visible.
  - `NoRuntimeAllocationBeforeCommit`: `runtimeAllocated[r] => committed[r]`.
  - `IndexesOnlyCommitted`: every visible index points to a committed accepted run.
  - `NoPartialAfterFailure`: failed non-committed attempts never satisfy accepted-run visibility.
  - `ReadbackOnlyCommitted`: readback returns accepted only for committed runs.
- Liveness/eventuality:
  - `EventuallyAckOrFail`: every valid strict admission attempt eventually commits then acknowledges, or fails without acknowledgement.
  - `EventuallyReadableAfterCommit`: every committed accepted run is eventually readable after restart.
- Fairness assumptions: weak fairness on `CommitBatch`, `Acknowledge`, and `Readback` when enabled; no fairness assumed for injected failures.
- Deadlock freedom: TLC must check deadlock freedom for all bounded failure placements.
- Refinement to Rust/runtime behavior: storage API calls refine staging actions, Fjall strict batch commit refines `CommitBatch`, CLI/runtime returned success refines `Acknowledge`, in-memory run insertion refines `AllocateRuntime`, and restart inspection refines `Readback`.

## Evidence Command

- After State 4 writes the model, run: `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`
- Expected evidence: TLC exits 0 with no invariant violations, no deadlock, and temporal properties satisfied under configured bounds.

## Waivers

- None. This bead is temporal/durability/admission workflow work; TLA+ coverage is required.
