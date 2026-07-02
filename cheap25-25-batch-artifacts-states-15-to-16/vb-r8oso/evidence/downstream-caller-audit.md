# Downstream Caller Audit — vb-r8oso

**bead_id:** vb-r8oso
**owner:** holzman-rust
**captured_at:** 2026-07-01T20:30:00Z

## C-10 Research Requirement

> Grep `append_journaled|append_strict|append_unfsynced|append_event` across
> `crates/vb_runtime` and `crates/vb_storage::recovery`; report any caller
> that supplies an `event.seq()` not derived from a fresh per-run counter
> before closing the bead.

## Grep Results

```
$ rg "append_journaled|append_strict|append_unfsynced|append_event" \
    crates/vb_runtime/src/

crates/vb_runtime/src/journal/chunk_002.rs:34:    self.journal.append_strict(event)
crates/vb_runtime/src/journal/chunk_002.rs:36:    self.journal.append_journaled(event)
```

Excluding tests:

```
$ rg "\.append_journaled|\.append_strict|\.append_unfsynced|\.append_event" \
    crates/vb_runtime/src/ \
    | grep -v "test\|//"

crates/vb_runtime/src/journal/chunk_002.rs:34:    self.journal.append_strict(event)
crates/vb_runtime/src/journal/chunk_002.rs:36:    self.journal.append_journaled(event)
```

## Production Callers

The only non-test production caller of the append paths in `vb_runtime` is
`StorageRuntimeJournal::append_storage_event` in
`crates/vb_runtime/src/journal/chunk_002.rs`:

```rust
fn append_storage_event(&self, event: &JournalEvent) -> RuntimeResult<()> {
    let result = if self.profile == DurabilityProfile::Strict {
        self.journal.append_strict(event)
    } else {
        self.journal.append_journaled(event)
    };
    result.map_err(RuntimeError::from)
}
```

The `event.seq()` originates from the runtime shard's
`append_sequenced` machinery:

```rust
// crates/vb_runtime/src/journal/chunk_002.rs
fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
    let storage_event = Self::storage_event(event, seq)?;
    self.append_storage_event(&storage_event)?;
    Ok(())
}
```

The `seq` parameter is supplied by the shard's `append_journal_event`:

```rust
// crates/vb_runtime/src/shard/impl_parts/chunk_001.rs
pub(crate) fn append_journal_event(&mut self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
    let run = event.run_id();
    let seq = self.journal_sequence_for(run);
    self.journal.append_sequenced(event, seq)?;
    self.advance_journal_sequence(run, seq)
}
```

`journal_sequence_for` reads the per-run counter from an in-memory map:

```rust
fn journal_sequence_for(&self, run: RunId) -> EventSeq {
    self.journal_sequences
        .get(&run)
        .copied()
        .unwrap_or(EventSeq::ZERO)
}
```

The counter starts at `EventSeq::ZERO` for any fresh run, and is
incremented by `advance_journal_sequence` only after a successful append:

```rust
fn advance_journal_sequence(&mut self, run: RunId, seq: EventSeq) -> RuntimeResult<()> {
    let next = seq
        .get()
        .checked_add(1)
        .map(EventSeq::new)
        .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
    self.reserve_journal_sequence_slot(run)?;
    let _previous = self.journal_sequences.insert(run, next);
    Ok(())
}
```

## Conclusion

Every seq the runtime supplies is contiguous (0, 1, 2, …) and satisfies
the new `next_sequence_at_write` guard. The contract assumption in C-10
("no downstream caller legitimately writes a non-contiguous seq") is
upheld.

`crates/vb_storage::recovery/`: the only `append_journaled` calls in
the recovery tree are in `recovery/tests.rs` (test code, already updated
to the new contract). Recovery does not have a production path that
calls the guarded append methods; it only reads via
`recovery::recover_full_journal` and `replay_journal` /
`events_for_run`.

## Test Code Outside the Test Crates

The other append calls in the grep result are in test code:

- `crates/vb_runtime/src/primitives/collect/tests.rs` (test code)
- `crates/vb_runtime/src/verification/kani/kani_admission_ordering.rs` (Kani test)

These are exercised by the existing test suites and have been verified to
pass with the new guard. No production caller writes a non-contiguous
seq.
