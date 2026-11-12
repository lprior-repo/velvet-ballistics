# TLA+ Temporal Model Plan: vb-core-ipc-sync-evidence

## Boundary
- Current executable scope: bounded finite-state safety/enabledness model for IPC submit admission, bounded queues, terminal races, timer exclusion, shutdown monotonicity, slow-client buffers, and no unbounded fanout.
- Explicit non-claim: existing configs do not prove temporal liveness/fairness because they contain only `INVARIANT` declarations and `CHECK_DEADLOCK FALSE`.
- Rust/core behavior excluded from TLA+ and handled by Verus/static scan/tests/blockers: header arithmetic, capacity constructors, digest equality, production API refinement, static dependency bans, and source fanout classification.
- External systems abstracted: filesystem/journal durability, socket readiness, artifact storage, wall-clock time, and OS scheduling.

## TLA+-Owned Clauses
- CON-IPC-001 -> `IpcSyncEvidence::StrictAdmissionBeforeRuntimeSubmit` and bounded enabledness predicate `AcceptedSubmitEventuallyQueuedOrRejected`.
- CON-IPC-002 -> `IpcSyncEvidence::QueueBoundsHold`, `NoSilentDrop`, and bounded enabledness predicate `FullSubmitEventuallyRejected`.
- CON-IPC-003 -> `IpcSyncEvidence::SingleTerminalOutcomePerRun`, `TerminalStateStable`, and bounded resolved-state predicate `RaceEventuallyResolved`.
- CON-IPC-004 -> `IpcSyncEvidence::NoTimerAfterTerminal`, `TimerOrderPreserved`, and bounded enabledness predicate `EligibleTimerEventuallyFiresOrBecomesIneligible`.
- CON-IPC-005 -> `IpcSyncEvidence::ShutdownNeverReopensAdmission`, `NoSubmitAcceptedAfterShutdown`, and bounded shutdown-state predicate `ShutdownEventuallyDrainedOrExplicitlyRejected`.
- CON-IPC-006 -> `IpcSyncEvidence::ClientBuffersBounded` and bounded enabledness predicate `SlowClientEventuallyWritableOrDisconnected`.
- CON-IPC-007 -> `IpcSyncEvidence::QueueBoundsHold` plus bounded worker/fanout state abstraction in `verification/tla/IpcSyncEvidence.tla`; source-level fanout remains owned by `SCAN-IPC-007`.

## Existing Model Shape
- Module/model path: `verification/tla/IpcSyncEvidence.tla`.
- Capacity-2 config: `verification/tla/IpcSyncEvidence.cfg`.
- Capacity-1 config: `verification/tla/IpcSyncEvidenceCap1.cfg`.
- Variables: `artifact_ok`, `accepted`, `rejected`, `queued`, `queue_len`, `runtime_submitted`, `terminal`, `terminal_count`, `timer_eligible`, `timer_fired`, `timer_after_terminal`, `shutdown`, `admission_open`, `drained`, `buffer_used`, `connected`.
- Init action: `Init`.
- Spec/next relation: `Spec == Init /\ [][Next]_vars`.
- Actions: `AcceptSubmit`, `RejectMissingArtifact`, `RejectFullQueue`, `RejectAfterShutdown`, `DrainOne`, `CompleteRun`, `CancelRun`, `StaleTerminalEvent`, `FireTimer`, `StaleTimerEvent`, `StartShutdown`, `MarkDrained`, `WriteToClient`, `DisconnectSlowClient`.
- State constraints: `RUNS = {r1, r2}`, `CLIENTS = {c1, c2}`, `QUEUE_CAPACITY` in `{1,2}`, `BUFFER_CAPACITY` in `{1,2}` via the two checked configs.

## Properties Currently Claimed
- Safety invariants: `TypeOk`, `StrictAdmissionBeforeRuntimeSubmit`, `NoSilentDrop`, `QueueBoundsHold`, `SingleTerminalOutcomePerRun`, `TerminalStateStable`, `NoTimerAfterTerminal`, `TimerOrderPreserved`, `ShutdownNeverReopensAdmission`, `NoSubmitAcceptedAfterShutdown`, `ClientBuffersBounded`.
- Bounded enabledness/state predicates currently named like eventuality but checked as invariants: `AcceptedSubmitEventuallyQueuedOrRejected`, `FullSubmitEventuallyRejected`, `RaceEventuallyResolved`, `EligibleTimerEventuallyFiresOrBecomesIneligible`, `ShutdownEventuallyDrainedOrExplicitlyRejected`, `SlowClientEventuallyWritableOrDisconnected`.
- Fairness assumptions: none are currently executable in the existing configs.
- Deadlock stance: existing configs set `CHECK_DEADLOCK FALSE`; therefore no deadlock-freedom claim is made.
- Refinement boundary: production event traces from IPC server, runtime submit, shard queue, timer handling, shutdown, and client buffers must refine the abstract variables/actions before final proof closure. This is tracked by `REFINE-IPC-001` through `REFINE-IPC-005`, `PROP-IPC-006`, and `SCAN-IPC-007`.

## Evidence Commands
- `tlc -config verification/tla/IpcSyncEvidence.cfg verification/tla/IpcSyncEvidence.tla`
- `tlc -config verification/tla/IpcSyncEvidenceCap1.cfg verification/tla/IpcSyncEvidence.tla`
- Expected evidence: TLC reports `Model checking completed. No error has been found.` for both configs, with no invariant violations for the listed safety/enabledness predicates.

## Blockers and Waivers
- `BLOCK-TLA-LIVENESS`: real temporal liveness/fairness/deadlock proof is blocked. Owner: State 5 proof-writer/proof repair. Reason: existing configs have no `PROPERTY`, fairness clauses, or deadlock checking. Compensating evidence: bounded safety/enabledness TLC passes above. Required repair: add real temporal properties/fairness/deadlock stance or keep all future claims downgraded to safety/enabledness.
- No TLA+ waiver for CON-IPC-001 through CON-IPC-007 bounded safety/enabledness coverage.
- CON-IPC-008 is non-temporal static source/dependency policy and remains assigned to static scan.
