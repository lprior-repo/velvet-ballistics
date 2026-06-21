# SJ-001: Dead duplicate-handling catch in `append_queued_indexed_unpersisted`

- **Severity**: Low
- **Category**: simplification
- **Location**: `crates/vb_storage/src/journal/append/journal_impl.rs:65`
- **Confidence**: confirmed

## Description

`append_queued_indexed_unpersisted` wraps `append_indexed_unpersisted` in a
`match` arm that re-handles `JournalError::DuplicateEvent`. That arm is dead:
`append_indexed_unpersisted` already catches `DuplicateEvent` returned from
`append_event_and_index` and converts it via `accept_equal_duplicate` into
either `Ok(())` or a different error variant. The outer catch can therefore
never observe a `DuplicateEvent`.

## Evidence

```rust
pub(crate) fn append_indexed_unpersisted(
    &self,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    let intent = super::mrwe6_action_index_intent(event);
    if matches!(intent, ActionIndexIntent::None) {
        return self.append_unpersisted(event);
    }
    match self.append_event_and_index(event, intent) {
        Ok(()) => Ok(()),
        Err(JournalError::DuplicateEvent { run, seq }) => {
            self.accept_equal_duplicate(event, run, seq)   // <-- swallows DuplicateEvent
        }
        Err(e) => Err(e),
    }
}

pub(crate) fn append_queued_indexed_unpersisted(
    &self,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    match self.append_indexed_unpersisted(event) {
        Ok(()) => Ok(()),
        Err(JournalError::DuplicateEvent { run, seq }) => {           // unreachable
            self.accept_equal_duplicate(event, run, seq)
        }
        Err(e) => Err(e),
    }
}
```

`accept_equal_duplicate` only re-emits `DuplicateEvent` when the existing
payload diverges; that path is already covered by the inner catch. The outer
`Err(JournalError::DuplicateEvent { .. })` arm is therefore unreachable.

## Adversarial Check

A counter-argument is that `append_queued_indexed_unpersisted` exists for a
subtle reason — perhaps a future change to `append_indexed_unpersisted` could
re-introduce a DuplicateEvent path. But until such a change exists, this is
literally dead code with no behavioral difference, and a future author reading
the queue path will believe duplicate handling happens here. The dead branch
misleads readers.

## Suggested Fix

Delete the wrapper entirely and call `append_indexed_unpersisted` directly, or
remove the `Err(JournalError::DuplicateEvent { .. })` arm:
```rust
pub(crate) fn append_queued_indexed_unpersisted(
    &self,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    self.append_indexed_unpersisted(event)
}
```
