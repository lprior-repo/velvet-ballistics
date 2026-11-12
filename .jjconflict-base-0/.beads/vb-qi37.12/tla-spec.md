# TLA+ Temporal Model Plan: vb-qi37.12

## Boundary
- Temporal behavior: mutation request, required journal/storage persist, acknowledgement, runtime state transition, recovery/replay observation, and fail-closed error propagation.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/proptest/tests later: concrete Rust enum layout, concrete error type constructors, postcard byte decoding, fjall internals, compiler AST validation logic.
- External systems abstracted: Fjall database, filesystem process locks, deterministic engine, caller/API/CLI boundary.

## TLA+-Owned Clauses
- INV-001 / POST-001: no success acknowledgement after a required persist failure.
- INV-003 / POST-005: corrupt recovery-critical data eventually reaches typed failure, not successful empty hydration.
- INV-002 / POST-003: runtime engine-drive failure cannot be transformed into terminal failure while losing the causal diagnostic.

## Model Shape
- Module/model path: `.beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla`.
- Config path: `.beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg`.
- Module name: `SilentDiscardLifecycle`.
- Variables: `op_state`, `persist_result`, `ack_state`, `runtime_state`, `recovery_input`, `diagnostic`, `discard_classification`.
- Init action: `InitNoMutation`.
- Next/actions: `StartMutation`, `PersistOk`, `PersistFail`, `AckSuccess`, `ReturnTypedError`, `BeginRecovery`, `DecodeOk`, `DecodeCorrupt`, `HydrateSuccess`, `HydrateFailClosed`, `EngineDriveOk`, `EngineDriveFail`, `TerminalFailureWithCause`, `TerminalFailureWithoutCause`.
- State constraints: finite operations `{journal_append, batch_commit, lock_metadata, recovery_decode, runtime_drive, compiler_validate}`; finite results `{ok, failed, corrupt, absent}`; finite classifications `{must_propagate, must_accumulate, typed_optional, typed_best_effort_discard, unclassified}`.
- Symmetry sets: operations may be symmetric only when they share durability/recovery criticality; runtime and storage operations must remain distinct.
- Bounded model limits: at least two operations and one recovery cycle to catch stale acknowledgement plus replay behavior.

## Properties
- Safety invariants:
  - `NoAckAfterFailedRequiredPersist`: if a required persist fails, success acknowledgement is never emitted for that mutation.
  - `NoUnclassifiedDiscard`: no production fallible operation reaches terminal success with `unclassified` discard classification.
  - `CorruptionDoesNotHydrateEmptySuccess`: corrupt recovery-critical data cannot lead to successful empty hydration.
  - `DiagnosticCausePreserved`: terminal runtime failure after engine error retains a cause token.
- Liveness/eventuality:
  - `PersistFailureEventuallyTypedError`: every required persist failure eventually returns or records a typed error.
  - `RecoveryCorruptionEventuallyFailClosed`: every corrupt recovery-critical decode eventually reaches fail-closed recovery.
- Fairness assumptions: weak fairness on enabled `ReturnTypedError`, `HydrateFailClosed`, and `TerminalFailureWithCause`; no fairness assumed for external filesystem/database success.
- Deadlock freedom: discharged by State 5 repair. `.beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg` does not disable deadlock checking, `Next` contains no explicit unconditional `Stutter`, and TLC exits 0 with no deadlock error.
- Refinement to Rust/runtime behavior: Rust journal append, process lock, recovery replay, and runtime drive functions refine model actions by operation kind, run id when available, record kind when available, result, and diagnostic cause token.

## Evidence Command
- `tlc -config .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla`.

## Waivers
- None for temporal safety/liveness/deadlock modeling.
