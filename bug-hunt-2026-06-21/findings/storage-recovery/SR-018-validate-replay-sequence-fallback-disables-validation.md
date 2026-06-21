# SR-018: `validate_replay_sequence` falls back to `event.seq()` when `expected` is `None`

- **Severity**: Low
- **Category**: bug
- **Location**: `crates/vb_storage/src/journal/replay.rs:127`
- **Confidence**: confirmed

## Description

`validate_replay_sequence` updates `expected` to `Some(next_seq)` on every
iteration and is initialized to `Some(first_event)` by its only caller. The
expression `expected.unwrap_or_else(|| event.seq())` therefore never
actually falls back. But the existence of the fallback masks the
invariant: if a future refactor breaks the `Some`-always invariant, the
fallback would silently disable sequence validation for that event instead
of failing closed.

## Evidence

```rust
fn validate_replay_sequence(
    run: vb_core::RunId,
    expected: &mut Option<EventSeq>,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    let expected_seq = expected.unwrap_or_else(|| event.seq());
    crate::codec::validate_replayed_event(run, expected_seq, event)?;
    *expected = Some(crate::codec::next_seq(expected_seq)?);
    Ok(())
}
```

The caller (replay.rs:95):
```rust
let mut expected = Some(first_event);
...
validate_replay_sequence(run, &mut expected, &event)?;
```

`expected` is reassigned to `Some(...)` at the end of every call, so the
`None` branch is unreachable today. But the fallback uses the event's own
`seq`, meaning "validate the event against itself" — which always passes.
That is a no-op validation, exactly the wrong default.

## Adversarial Check

A reading is "this is defensive code; the unwrap_or_else is just to satisfy
the type system." But `Option::unwrap` is forbidden in this codebase, so
the choice of fallback *is* a real decision — and choosing `event.seq()`
means "if we ever lose track of the expected sequence, just trust the
event." The safe fallback is to return a `ReplayDivergence` error
("expected sequence lost"). The current code is a latent trap.

## Suggested Fix

Replace the fallback with a hard error:
```rust
let expected_seq = match expected {
    Some(seq) => seq,
    None => return Err(JournalError::ReplayDivergence {
        run,
        detail: format!("expected sequence lost before event seq {}", event.seq().get()),
    }),
};
```
Or refactor `expected` to be `EventSeq` (not `Option<EventSeq>`) so the
type system enforces the invariant.
