# vb-kyyf TLA+ Temporal Model Plan

## Boundary
- Temporal/workflow behavior: run execution, persistence, reopen/replay, corruption/digest failure, side-effect replay policy, and generated/IR observation convergence over repeated attempts.
- Rust/core behavior excluded from TLA+: concrete byte decoding, concrete normalization implementation, exact storage API signatures, concrete generated Rust source emission.
- External systems abstracted: Fjall as a durable map of run evidence; action side effects as counted dispatch tokens with replay-safety class; CLI/runtime/storage public calls as observation actions.

## TLA+-Owned Clauses
- POST-002 -> `verification/tla/VbKyyfReplayDeterminism.tla::ReplayIsReproducible`
- POST-003 -> `verification/tla/VbKyyfReplayDeterminism.tla::NoUnsafeSideEffectReexecution`
- POST-004 -> `verification/tla/VbKyyfReplayDeterminism.tla::BadEvidenceFailsStably`
- INV-003 -> `verification/tla/VbKyyfReplayDeterminism.tla::JournalSequenceWellFormed`
- INV-004 -> `verification/tla/VbKyyfReplayDeterminism.tla::DigestMismatchNeverContinues`
- INV-005 -> `verification/tla/VbKyyfReplayDeterminism.tla::SideEffectDispatchBoundedByPolicy`

## Model Shape
- Module/model path to be created by proof state: `verification/tla/VbKyyfReplayDeterminism.tla`
- Config path to be created by proof state: `verification/tla/VbKyyfReplayDeterminism.cfg`
- Variables:
  - `runs`: finite set of run ids.
  - `store`: durable evidence per run: records, snapshots, digests, corruption flags, sequence numbers.
  - `observations`: normalized reports returned by public surfaces.
  - `actionClass`: action replay class per scheduled action.
  - `sideEffectDispatches`: count of external dispatches per action ticket.
  - `status`: `NotStarted`, `Running`, `Persisted`, `Reopened`, `Replayed`, `Blocked`, `Failed`, `Finished`.
  - `generatedMode`: `Unsupported`, `SupportedIr`, `SupportedGenerated`, `Compared`.
- Init action: `Init` creates bounded accepted runs with finite records, digest state, and action replay class.
- Next/actions: `RunOnce`, `PersistEvidence`, `DropAndReopen`, `ReplayFromEvidence`, `ObserveViaPublicSurface`, `CorruptRecord`, `DetectDigestMismatch`, `ScheduleAction`, `CompleteAction`, `AttemptUnsafeReplay`, `CompareGeneratedAndIr`, `FailClosedUnsupportedGeneratedSubset`.
- State constraints:
  - `Cardinality(runs) <= 2` for TLC smoke bounds.
  - sequence numbers are bounded `0..MAX_SEQ` with explicit overflow/error state; no unbounded Nat assumption for production counters.
  - finite action tickets and finite observation values.
- Symmetry sets: run ids and action ticket ids are symmetric when digest/action class assignments match.
- Bounded model limits: `MAX_SEQ = 4`, `MAX_RECORDS = 4`, `MAX_REPLAY_ATTEMPTS = 3`, `MAX_ACTIONS = 2` for initial TLC; proof planner may tighten/expand.

## Properties
- Safety invariants:
  - `JournalSequenceWellFormed`: persisted valid records are contiguous and monotonic per run.
  - `ReplayIsReproducible`: same persisted evidence and public-surface request yields same normalized observation on repeated replay.
  - `DigestMismatchNeverContinues`: digest mismatch moves to typed failure, never to `Finished` by replay.
  - `BadEvidenceFailsStably`: corrupt/gapped/duplicate evidence returns stable typed failure on every repeated attempt.
  - `NoUnsafeSideEffectReexecution`: non-replay-safe side effects dispatch at most once after scheduled evidence exists.
  - `GeneratedIrObservationParity`: supported generated and IR observations are equal before parity evidence is accepted.
- Liveness/eventuality:
  - Under weak fairness for enabled observe/replay actions, every persisted valid run eventually reaches `Finished`, `Blocked`, or `Failed` observation.
  - Unsupported generated subset eventually reaches fail-closed diagnostic if parity is requested.
- Fairness assumptions:
  - Weak fairness on `ReplayFromEvidence`, `ObserveViaPublicSurface`, and `DetectDigestMismatch` when enabled.
  - No fairness assumption for external action completion; blocked state is valid and terminal for replay policy.
- Deadlock freedom:
  - Required under finite bounds except explicit terminal `Finished`, `Blocked`, or `Failed` states.
- Refinement to Rust/runtime behavior:
  - Storage/runtime/CLI calls refine `ObserveViaPublicSurface` by returning a `NormalizedObservation`.
  - Journal append/reopen/refetch refines `PersistEvidence` and `DropAndReopen`.
  - Typed Rust errors refine TLA `Failed` variants.
  - Action dispatch hooks refine `sideEffectDispatches` updates.

## Evidence Command
- Planned exact command after proof artifacts exist: `tlc -config verification/tla/VbKyyfReplayDeterminism.cfg verification/tla/VbKyyfReplayDeterminism.tla`
- Expected evidence: TLC reports no invariant violations, no unexpected deadlock, and temporal properties satisfied for configured finite bounds.

## Waivers
- None for temporal replay/recovery behavior. TLA+ is applicable because this bead is explicitly state-over-time and replay-policy scoped.
