# Domain Model Review - vb-qi37.12.2

STATUS: CONTRACT_NARROWED

## Decision

The domain model must distinguish typed failure classification from exact source identity.

- `RuntimeState` distinguishes `Running`, `Resumable`, `Resuming`, and terminal states; R3 depends on restoring `Resumable` after a failed `Resumed` append.
- `ResumeError::NotResumable { run_id, current_state }` remains the public illegal-state carrier for R4.
- `ResumeError::JournalAppendFailed` as a unit variant can carry only the semantic class "resume journal append failed". It cannot bind an operation-specific runtime/storage source by value.
- Exact source preservation is a property of error shapes or APIs that actually carry source detail. It is not a property of a unit variant.
- Hidden ambient source state is outside the domain model and is forbidden because it permits stale-source theft.

## Consequence for Downstream States

- State 4/7 proof work must prove semantic error classification, no false success, retry restoration, deterministic fallback, and no hidden source side channel.
- State 8 tests must stop expecting `JournalAppendFailed` unit values to expose per-error source identity. Tests may only assert source binding through public carriers or approved non-ambient APIs.
- State 10 implementation must not fake R5 with globals, thread locals, task locals, or stale stored source state.
