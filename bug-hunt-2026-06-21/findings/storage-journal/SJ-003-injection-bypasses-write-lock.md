# SJ-003: `inject_raw_event` / `inject_seq_gap` bypass the `write_lock`

- **Severity**: Medium
- **Category**: concurrency
- **Location**: `crates/vb_storage/src/journal/injection.rs:15`
- **Confidence**: confirmed

## Description

`append_unpersisted` and `append_event_and_index` both acquire
`self.write_lock` before mutating the events keyspace, both for poison
detection and for serializing appends. `inject_raw_event` and `inject_seq_gap`
mutate the same keyspace without acquiring the lock, opening a race window
with concurrent appends and defeating the poison-detection contract.

## Evidence

`internal.rs:28-43` — append path takes the lock:
```rust
pub(crate) fn append_unpersisted(&self, event: &JournalEvent) -> Result<(), JournalError> {
    let _guard = self
        .write_lock
        .lock()
        .map_err(|_| JournalError::WriteLockPoisoned)?;
    let key = run_event_key(event.run_id(), event.seq())?;
    if self.events.contains_key(key)? {
        return Err(JournalError::DuplicateEvent { ... });
    }
    ...
    self.events.insert(key.to_vec(), value)?;
    Ok(())
}
```

`injection.rs:15-32` — injection path skips the lock:
```rust
pub fn inject_raw_event(
    &self,
    run: vb_core::RunId,
    seq: EventSeq,
    kind: crate::records::RecordKind,
    payload: &[u8],
) -> Result<(), JournalError> {
    let key = run_event_key(run, seq)?;
    let value = encode_record(...)?;
    self.events.insert(key.to_vec(), value)?;
    Ok(())
}
```

Injection also skips the duplicate-key check, so it can silently overwrite a
real event at the same `(run, seq)`.

## Adversarial Check

A counter-argument is that injection is a disaster-recovery primitive and
concurrent appends are unlikely during DR. But the public API surface is
`pub fn` on `FjallJournal` with no feature gate, and the lock is not
documented as advisory. The skip means (1) the `WriteLockPoisoned` signal
becomes unreliable for the injection path, and (2) a concurrent append that
races with `inject_raw_event` can interleave `contains_key` / `insert` such
that one of them silently overwrites the other. The lack of duplicate
detection means injection can corrupt an existing event without any error.

## Suggested Fix

Either acquire the lock and perform the duplicate check inside
`inject_raw_event` / `inject_seq_gap`, or feature-gate them behind
`#[cfg(any(test, feature = "test-support"))]` and document that they are
test-only utilities that must not be called concurrently with append paths.
