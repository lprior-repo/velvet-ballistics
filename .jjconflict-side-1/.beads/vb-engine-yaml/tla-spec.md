# TLA+ Temporal Model Plan: vb-engine-yaml

## Boundary

- Temporal/workflow behavior: accepted-artifact lifecycle, strict persist-before-ack, direct/IPC submission, bounded backpressure, run execution/suspension/terminal transitions, recovery/replay, capability/idempotency gate admission.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/proptest/fuzz/Miri/tests: numeric indexing, resource arithmetic, taint lattice joins, record envelope decoding, checked array access, finite value handling.
- External systems abstracted: Fjall as a durable key-value log with atomic batch/fail behavior; Postcard/envelope decoder as a typed record validity predicate; direct API/IPC as bounded command producers; actions as idempotency-classed outcomes.
- Non-applicability rationale: not applicable; this bead contains lifecycle, persistence, queue, and recovery temporal behavior.

## TLA+-Owned Clauses

- POST-003 / INV-004 -> `verification/tla/EngineYamlAdmission.tla::PersistBeforeAck`
- POST-005 / INV-005 -> `verification/tla/EngineYamlRunLifecycle.tla::MonotonicJournalAndTerminalConsistency`
- POST-006 / INV-008 -> `verification/tla/EngineYamlRecovery.tla::NoYamlReparseAndFailClosedRecovery`
- PRE-006 / POST-007 -> `verification/tla/EngineYamlIngress.tla::BoundedIngressNoBypass`
- INV-007 -> existing capability lifecycle model `verification/tla/CapabilityLifecycle.tla` with all configs.

## Model Shape

- Module/model paths planned:
  - `verification/tla/EngineYamlAdmission.tla` with config `verification/tla/EngineYamlAdmission.cfg` (to be written by proof-writer if not present).
  - `verification/tla/EngineYamlRunLifecycle.tla` with config `verification/tla/EngineYamlRunLifecycle.cfg` (to be written by proof-writer if not present).
  - `verification/tla/EngineYamlRecovery.tla` with config `verification/tla/EngineYamlRecovery.cfg` (to be written by proof-writer if not present).
  - `verification/tla/EngineYamlIngress.tla` with config `verification/tla/EngineYamlIngress.cfg` (required model-level ingress/backpressure proof; not replaceable by Loom).
  - Existing `verification/tla/CapabilityLifecycle.tla` with `verification/tla/CapabilityLifecycleAll.cfg`.
- Variables:
  - `artifact_state`: absent, candidate, verified, accepted, rejected.
  - `durable_records`: source, artifact, header, accepted, indexes, journal, snapshot, blob.
  - `ack_state`: none, acknowledged, failed_before_ack.
  - `run_state`: new, accepted, running, suspended, cancelled, finished, failed, recovering, replaying.
  - `seq`: per-run journal sequence number.
  - `ingress_queue`: bounded direct/IPC command queue.
  - `proof_gates`: bounded set of required verification gates and pass/fail/missing state.
  - `capabilities`: required, granted, denied.
  - `recovery_source`: snapshot_tail, full_journal, corrupt, missing, digest_mismatch.
- Init actions: `InitAdmission`, `InitLifecycle`, `InitRecovery`, `InitIngress`.
- Next/actions: `ValidateYamlCold`, `CompileNumericIr`, `BuildArtifact`, `VerifyGates`, `PersistBatch`, `AckAccepted`, `FailBeforeAck`, `SubmitDirect`, `SubmitIpc`, `RejectBackpressure`, `StartRun`, `Step`, `Suspend`, `AppendJournal`, `CompleteAction`, `Retry`, `Cancel`, `Finish`, `Fail`, `BeginRecovery`, `HydrateFromDurableRecords`, `DetectMismatch`, `FailClosedRecovery`, `Replay`.
- State constraints: finite runs, finite artifacts, bounded queue capacity, bounded sequence range for TLC, finite gate/capability sets.
- Symmetry sets: runs and command producers may be symmetric where model permits.
- Bounded model limits: 2 runs, 2 artifacts, queue capacity 2, sequence range 0..5, gate set size 5 for smoke; larger bounds for deep proof lane.

## Properties

- Safety invariants:
  - `NoAckWithoutDurableAcceptedRecords`: acknowledged strict run has source, accepted artifact, run header, RunAccepted event, and required indexes.
  - `NoRawIrBypass`: runtime accepted state implies artifact_state = accepted and all required gates pass.
  - `NoRuntimeYaml`: recovery/execution states never transition through YAML parsing action.
  - `SeqMonotonic`: per-run journal sequence increases strictly.
  - `BoundedIngress`: queue length never exceeds capacity; overflow rejects rather than blocks or grows.
  - `BackpressureRejectsWithoutGrowth`: a direct or IPC submit attempted while its bounded queue is full records a typed rejection and leaves queue length at or below capacity.
  - `NoIngressBypass`: accepted runtime submission is reachable only through typed direct/API or binary IPC commands carrying accepted-artifact identity; loose YAML, JSON, HTTP, and text commands are rejected before runtime admission.
  - `TypedOperatorOutcome`: every accepted, rejected, backpressured, invalid-frame, and unsupported-protocol ingress outcome has a typed diagnostic class observable by operator surfaces.
  - `CapabilityGateRequired`: missing or excessive capability grants cannot reach accepted runtime dispatch.
  - `FailClosedRecovery`: corrupt/missing/mismatched durable records reach failed recovery, not running.
- Temporal properties:
  - `EventuallyAckOrFailBeforeAck`: every strict persist attempt eventually acknowledges after durable batch or fails before acknowledgement.
  - `EventuallyTerminalOrSuspended`: every admitted run under fairness reaches terminal, suspended, or explicit failed state within bounded execution model.
  - `RecoveryEventuallyHydratesOrFailsClosed`: every recovery attempt eventually hydrates valid state or fails closed.
  - `IngressEventuallyAcceptedOrTypedRejected`: every well-typed direct/API or binary IPC submit attempt eventually either enqueues/admit-hands-off within capacity or receives a typed rejection/backpressure diagnostic.
- Fairness assumptions: weak fairness for enabled persist, dequeue, journal append, recovery hydrate, and terminal transition actions; no fairness for unavailable external action completion.
- Deadlock freedom: TLC must report no deadlock except explicit terminal states if terminal stuttering is modeled.
- Refinement to Rust/runtime behavior: Rust events refine TLA+ actions by run id, artifact digest, journal sequence, command kind, durability result, gate result, and recovery result.

## Evidence Commands

- Existing capability lifecycle: `tlc -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla`
- Planned admission model: `tlc -config verification/tla/EngineYamlAdmission.cfg verification/tla/EngineYamlAdmission.tla`
- Planned lifecycle model: `tlc -config verification/tla/EngineYamlRunLifecycle.cfg verification/tla/EngineYamlRunLifecycle.tla`
- Planned recovery model: `tlc -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla`
- Planned ingress/backpressure model: `tlc -config verification/tla/EngineYamlIngress.cfg verification/tla/EngineYamlIngress.tla`

## Waivers

- No TLA+ waiver for lifecycle/admission/recovery/ingress behavior. Planned model files that do not yet exist are BLOCKED obligations for proof-writer discovery/writing, not waived.
