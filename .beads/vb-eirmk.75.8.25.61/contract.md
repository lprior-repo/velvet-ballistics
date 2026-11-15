# Contract: preserve typed non-resumable resume errors

## Scope

When a queued `ShardCommand::Resume` targets an active run whose `RuntimeState` is not `Resumable` and is not already `Running`, the runtime error must retain the `RunId` and exact current state. Do not classify this condition as `InvalidActionCompletion`. Preserve the current non-persisted resume protocol and the journal/IPC/persisted formats.

## Behavior

- Map `ResumeError::NotResumable { run_id, current_state }` into a typed `RuntimeError` carrying both values.
- Keep `RunIdNotFound` distinct: a terminal run is no longer in the active map and its queued resume continues to return `RuntimeError::RunNotFound`.
- Preserve `AlreadyRunning` and successful `Resumable` behavior.
- Preserve journal append/source-error conversion and all action-completion errors.
- Preserve every existing numeric diagnostic ID. Use an existing non-action diagnostic category for the new runtime error; add no numeric code or persisted/wire variant.

## Tests

- Public queued resume of an active `Initial` run returns the typed non-resumable error with exact run and state and no frame/state/journal mutation.
- Public queued resume of a terminal run remains exact `RunNotFound`.
- A Resumable run retains the current successful resume path; an already Running run retains `AlreadyRunning` behavior.
- Existing journal/source-error and action-completion behavior remains unchanged.

## Constraints

No new dependency, persisted schema, journal event, IPC variant, gate/registry/config change, or recovery authority change. Keep changed production functions within repository limits and propagate every `Result`.
