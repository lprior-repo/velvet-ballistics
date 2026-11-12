# TLA+ Temporal Model Plan: vb-qi37.1

## Boundary
- Temporal/workflow behavior: recovery lifecycle from persisted headers/events/snapshot through summary, frame seed, runtime hydration, fail-closed errors, and crash restart before/after acknowledgement.
- Rust/core behavior excluded from TLA+: byte decoding, concrete `RunFrame` memory layout, Rust enum layout, Fjall internals, postcard serialization internals.
- External systems abstracted: Fjall journal as an append/read sequence store; snapshots as validated records; accepted artifacts as digest tokens.
- Existing model status: no recovery-hydration TLA+ model found in State 2 artifacts. This State 3 plan defines the required model; proof-writer owns model creation.

## TLA+-owned clauses
- PRE-001 -> `verification/tla/RecoveryHydration.tla::HasDurableInput` plus `RestartEventuallyRecoversOrFailsClosed`: recovery starts only from journal events or snapshot input and reaches summary, frame, or typed fail-closed state.
- PRE-002 -> `verification/tla/RecoveryHydration.tla::NoMixedRunRecovery`: events for a requested run cannot include another run.
- PRE-003 -> `verification/tla/RecoveryHydration.tla::SnapshotThenTailOnly`: snapshot run identity matches and all tail events are strictly after the snapshot sequence.
- POST-001 -> `verification/tla/RecoveryHydration.tla::JournalSeqMonotonic` plus `HasDurableInput`: successful summary has non-empty ordered bounds for the requested run.
- POST-002 -> `verification/tla/RecoveryHydration.tla::FrameSeedCompleteOrUnsupported`: frame seed preserves durable pc/slot/taint/step/action/wait/ask/retry/collect/terminal facts or marks unsupported/fail-closed.
- POST-004 -> `verification/tla/RecoveryHydration.tla::SnapshotThenTailOnly`: snapshot facts precede and bound tail-event replay.
- POST-007 -> `verification/tla/RecoveryHydration.tla::CrashRestartPreservesDurableFactsOrFailsClosed` and `RestartEventuallyRecoversOrFailsClosed`: before-ack and after-ack crashes recover durable facts or fail closed.
- INV-001 -> `verification/tla/RecoveryHydration.tla::JournalSeqMonotonic`.
- INV-004 -> `verification/tla/RecoveryHydration.tla::NoYamlRecoveryInput`.
- INV-006 -> `verification/tla/RecoveryHydration.tla::TerminalConsistent`.

## Model shape
- Module/model path: `verification/tla/RecoveryHydration.tla` and `verification/tla/RecoveryHydration.cfg` (planned; not yet present).
- Variables: `runs`, `headers`, `journal`, `snapshots`, `accepted_digest`, `expected_digest`, `pc`, `slots`, `taint`, `steps`, `pending_actions`, `waits`, `asks`, `retries`, `collect_state`, `terminal`, `hydration`, `errors`, `acked`.
- Init action: `Init` creates finite runs with optional persisted headers, empty or non-empty journal prefixes, optional snapshot, expected digest, and no hydration result.
- Next/actions: `PersistHeader`, `AppendEvent`, `PersistSnapshot`, `AcknowledgeRun`, `Crash`, `Restart`, `VerifyDigest`, `RecoverSummary`, `RecoverFrameSeed`, `HydrateRuntimeFrame`, `FailClosed`.
- State constraints: finite sets for runs, events, slots, steps, actions, waits, asks, retries; bounded sequences for TLC.
- Symmetry sets: runs and action identifiers may be symmetry sets when event order remains explicit.
- Bounded model limits: at least 2 runs, 4 events/run, 3 slots, 3 steps, 2 actions, one snapshot, and both before-ack/after-ack crashes.

## Properties
- Safety invariants:
  - `JournalSeqMonotonic`: each recovered event sequence for a run is strictly increasing.
  - `NoMixedRunRecovery`: recovery input for one run never includes another run's event.
  - `NoSilentEmptyFrame`: non-empty durable input cannot produce successful empty live-frame hydration.
  - `NoFabricatedSlotOrTaint`: slot values and taint in a successful frame are present in snapshot/journal/replay facts.
  - `UnsupportedRejectsHydration`: unsupported state implies fail-closed error, not success.
  - `TerminalConsistent`: recovered terminal equals the terminal event facts.
- Liveness/eventuality:
  - `RestartEventuallyRecoversOrFailsClosed`: after crash and restart with finite durable data, recovery eventually reaches `RecoveredSummary`, `RecoveredFrame`, or typed `FailedClosed`.
- Fairness assumptions: weak fairness on `Restart`, `VerifyDigest`, `RecoverSummary`, `RecoverFrameSeed`, and `FailClosed` when enabled; no fairness for corrupt external writes.
- Deadlock freedom: every non-terminal recovery state must have `Recover*` or `FailClosed` enabled.
- Refinement to Rust/runtime behavior: `JournalEvent` append/read traces refine `AppendEvent`; `RunSnapshot` refines `PersistSnapshot`; `RecoveryHydration::{Summary,FrameSeed}` refines `RecoverSummary`/`RecoverFrameSeed`; `RuntimeRecoveryBoundary::hydrate_run_frame` refines `HydrateRuntimeFrame` or `FailClosed`.

## Evidence command
- `tlc -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla`.
- Expected evidence: TLC reports no invariant violations for all listed invariants and temporal property satisfaction for configured finite bounds. If `CHECK_DEADLOCK FALSE` remains in the cfg, State 5 must either remove it or add an explicit terminal-state deadlock design and a reviewer-approved deadlock waiver before State 6 approval.

## Waivers
- None. Temporal recovery/crash behavior is in scope and requires TLA+ coverage.
