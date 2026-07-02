# Contract - vb-qi37.12.2

STATUS: CONTRACT_NARROWED

## State 3 Decision

R5 is narrowed. State 10 established that true per-error source binding cannot be guaranteed while preserving the public semver-compatible unit variant `ResumeError::JournalAppendFailed`. Requiring the unit variant to carry an inspectable runtime/storage source would force either a semver break or an ambient side channel. The defensible contract is therefore semantic preservation at the public error boundary plus exact source preservation only when an API shape actually carries/binds that source.

## Requirements

- R1: Runtime/storage journal append failures must propagate as typed errors; no affected path may silently discard a failed durable write.
- R2: `handle_resume` must not return `Ok(Resumed)` when resume drive fails.
- R3: Failed `Resumed` append must restore `RuntimeState::Resumable` so retry observes retryable state, not `NotResumable`.
- R4: `ResumeError::NotResumable { run_id, current_state }` must carry both run id and current state.
- R5: Source-detail preservation is semver-compatible and shape-bounded:
  - R5a: Public `ResumeError::JournalAppendFailed` may remain a unit variant.
  - R5b: The unit variant guarantees the semantic cause class only: a journal append failed while handling resume.
  - R5c: Per-error runtime/storage source identity is guaranteed only where the returned public error value, a public source chain, or an owner-approved explicit non-ambient API carries and binds that source.
  - R5d: No implementation may satisfy R5 by reading or reusing hidden ambient/stale error state, global side channels, task-local leftovers, thread-local leftovers, or any source value not bound to the failing operation.
  - R5e: Conversion/fallback behavior must be deterministic: when source detail cannot be carried by the public error shape, the result must be the documented typed fallback (`ResumeError::JournalAppendFailed`) and must not claim inspectable per-error source preservation.

## Preconditions

- PRE-001: Resume is invoked for a run id whose current runtime state is read from the authoritative runtime state store.
- PRE-002: Journal append failure injection or storage failure is modeled as a failed durable write, not as a successful append with later observation failure.

## Postconditions

- POST-001 (R1/R2): If resume drive or resume journal append fails, `handle_resume` returns `Err`, never `Ok(Resumed)`.
- POST-002 (R3): If the `Resumed` journal append fails after transition toward resume, the visible retry state is restored to `RuntimeState::Resumable`.
- POST-003 (R4): `NotResumable` errors carry `run_id` and `current_state`.
- POST-004 (R5): Journal append failure conversion is deterministic and semver-compatible: unit `JournalAppendFailed` is an allowed lossy public fallback; exact source detail is required only on paths with a public bound source carrier or approved explicit non-ambient API.

## Invariants

- INV-001: No false success: a failed durable append on affected resume paths is never reported as successful resume.
- INV-002: Retry safety: failure of a `Resumed` append cannot leave the run in `NotResumable` solely because the failed append was attempted.
- INV-003: No hidden stale-source theft: source detail, when exposed, must be bound to the failing operation and must not be obtained from ambient storage or stale global/task/thread-local state.
- INV-004: Semver compatibility: the public `ResumeError::JournalAppendFailed` unit variant remains valid unless the owner explicitly chooses a semver-breaking API change.

## Error Taxonomy

- `ResumeError::JournalAppendFailed`: typed public fallback for resume journal append failure. It guarantees no false success and deterministic failure classification. It does not, by itself, guarantee inspectable per-error source identity.
- `ResumeError::NotResumable { run_id, current_state }`: caller attempted resume from an illegal state; carries both observable identifiers.
- Other existing resume errors: unchanged unless a public carrier explicitly binds source detail.

## Contract Signatures

- Existing public signatures remain semver-compatible. Fallible operations continue to return `Result<_, ResumeError>` or existing enclosing runtime error types.
- Any future exact-source preservation for `JournalAppendFailed` requires either an owner-approved semver-breaking variant shape or an owner-approved explicit non-ambient API that binds source detail to the failed operation.

## TLA+-Owned Clauses

- R2, R3, INV-001, INV-002: resume state transition and durable append failure workflow.

## Verus-Owned Clauses

- None required for this repair unless a pure conversion/state-transition kernel is extracted by downstream proof work. Current behavior is I/O shell/state workflow dominated.

## Theorem-Owned Clauses

- None.

## Non-goals

- Guaranteeing per-error source identity through unit `ResumeError::JournalAppendFailed`.
- Adding hidden side channels to recover source detail.
- Breaking public API semver without explicit owner decision.
