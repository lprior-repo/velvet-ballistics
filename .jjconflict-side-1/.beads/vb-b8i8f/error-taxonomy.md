# Error Taxonomy: vb-b8i8f Cancel/Kill Lattice Recovery

## Runtime Errors

| Domain condition | Required classification | Existing candidate | Contract |
|---|---|---|---|
| Public cancel for never-live/missing run | User-visible typed rejection | `RuntimeError::RunNotFound` | Must return `Err`, not `Ok(())`. |
| Public kill for never-live/missing run | User-visible typed rejection | `RuntimeError::RunNotFound` | Must return `Err`, not `Ok(())`. |
| Cancel for already terminal run | User-visible typed rejection | `RuntimeError::RunNotFound` or new `RunAlreadyTerminal` | Must append no event and not mutate terminal evidence. |
| Kill for already terminal run | User-visible typed rejection | `RuntimeError::RunNotFound` or new `RunAlreadyTerminal` | Must append no event and not mutate terminal evidence. |
| Stale timer after cancel/kill | Stale authority rejection | `RuntimeError::InvalidTimerFire` or `RunNotFound` | Must not mutate state or journal. |
| Stale action completion/failure after cancel/kill | Stale authority rejection | `RuntimeError::InvalidActionCompletion` or `RunNotFound` | Must not write slots or action events. |
| Stale ask answer after cancel/kill | Stale authority rejection | `RuntimeError::InvalidActionCompletion` or `RunNotFound` | Must not write slots or ask events. |
| Queue full before command accepted | Backpressure | `RuntimeError::QueueFull` | Caller gets `Err`; no implicit drop. |
| Storage append failure for terminal event | Durable boundary failure | `RuntimeError::StorageJournalAppend` | Must not produce false terminal success. |

## Storage Errors

| Domain condition | Existing error | Contract |
|---|---|---|
| Unknown record kind | `JournalError::UnknownRecordKind { kind }` | Kind 28 must not be unknown. |
| Known kind in wrong envelope family | `JournalError::RecordKindFamilyMismatch { magic, kind }` | `MAGIC_JOURNAL_EVENT + 28` must not mismatch. |
| Attempt zero for `RunKilled` | Event validity failure | `JournalEvent::is_valid()` rejects attempt zero. |
| Replay sequence gap after terminal event | Replay sequence failure | Must remain typed and must not silently skip. |

## Error Semantics

- Missing and terminal rejections may intentionally collapse to `RunNotFound` to avoid exposing run existence; what is forbidden is reporting success.
- Stale authority errors are part of correct behavior, not exceptional panics.
- `RecordKindFamilyMismatch { kind: 28 }` is a bug after this bead is implemented.
- No error path may require `unwrap`, `expect`, `panic`, unchecked indexing, unchecked arithmetic, JSON, YAML, or HTTP in runtime core.
