# TLA+ Temporal Model Plan

## Boundary
- Temporal/workflow behavior: persisted header admission, ordered journal replay, snapshot plus tail recovery, latest-attempt filtering, wait/ask/action/collect recovery continuity, fail-closed transitions.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/proptest/tests: byte decoding, exact Rust enum construction, frame dimension arithmetic, digest byte comparison, postcard slot decoding.
- External systems abstracted: Fjall persistence is modeled as an append-only durable event log and snapshot store with crash cuts; runtime action execution is modeled by ticket states rather than real I/O.

## TLA+-Owned Clauses
- PRE-002 -> ordered durable replay input.
- PRE-003 -> snapshot watermark and tail order.
- POST-002 -> full journal restart reconstructs latest attempt exactly.
- POST-003 -> snapshot plus tail monotonic recovery.
- POST-004 -> wait state survives restart.
- POST-005 -> ask answer value/taint survives restart.
- POST-006 -> action ticket pending/resolved behavior is preserved or rejected.
- POST-007 -> collect cursor/page state survives restart through durable extra.
- INV-002 -> deterministic replay.
- INV-003 -> no stale attempt mixing.
- INV-004 -> no monotonic fact erasure.
- INV-007 -> only sequenced events affect recovered state.

## Model Shape
- Planned module/model path: `verification/tla/RecoveryCrashRestart.tla` with configs under `verification/tla/RecoveryCrashRestart*.cfg`.
- Variables: `headers`, `events`, `snapshots`, `attempt`, `pc`, `steps`, `slots`, `taints`, `waits`, `asks`, `actions`, `collects`, `terminal`, `errors`, `crashed`, `recovered`.
- Init action: `InitNoRun` or `InitPersistedHeader` depending on bounded scenario.
- Next/actions: `PersistHeader`, `AppendStepStarted`, `AppendStepSucceeded`, `AppendSlotWritten`, `AppendWait`, `AppendAsk`, `AppendAnswer`, `AppendActionScheduled`, `AppendActionResolved`, `AppendRetry`, `PersistSnapshot`, `Crash`, `RecoverFullJournal`, `RecoverSnapshotTail`, `RejectCorruptOrUnsupported`.
- State constraints: finite bounded runs, attempts, steps, slots, tickets, wait ids, ask ids, collect cursors, and at most one active recovery target per model run.
- Symmetry sets: slots with equivalent dependency role may be symmetric; action tickets may be symmetric when idempotency class matches.
- Bounded model limits: start with 1 run, 2 attempts, 4 steps, 4 slots, 2 waits, 2 asks, 2 action tickets, 2 collect pages, 1 snapshot watermark, and corrupt/mismatch toggles.

## Properties
- Safety invariants: `NoSuccessWithoutDurableState`, `NoStaleAttemptMixing`, `SnapshotTailAfterWatermark`, `TaintExact`, `ActionTicketNotDuplicated`, `CollectIdentityExact`, `TypedFailureForInvalidInput`.
- Liveness/eventuality: under weak fairness for recovery actions, any run with sufficient durable data eventually reaches `Recovered` or a typed rejection state.
- Fairness assumptions: weak fairness on `RecoverFullJournal`, `RecoverSnapshotTail`, and `RejectCorruptOrUnsupported` when enabled; no fairness assumption on external action completion after crash.
- Deadlock freedom: model must deadlock only in terminal `Recovered` or `Rejected` states; TLC deadlock check remains enabled.
- Refinement to Rust/runtime behavior: `JournalEvent` append order refines `Append*` actions; `RunSnapshot` refines `PersistSnapshot`; `recover_*` APIs refine `Recover*`; typed `RecoveryError`, `RuntimeError`, and `EngineError` variants refine `Rejected` states.

## Evidence Command
- `moon run :verify-proof`
- Under that gate, the TLA lane must execute the planned recovery model and report no invariant violations, no unexpected deadlock, and temporal properties satisfied for the bounded configs.

## Waivers
- None. This bead is temporal by nature; TLA+ is required for recovery ordering, crash cuts, and fail-closed lifecycle behavior.
