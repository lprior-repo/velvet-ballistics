# RS-208-help: Snapshot writes reuse the snapshot sequence for the next journal event

- **Severity**: High
- **Category**: correctness / durability
- **Location**: `crates/vb_runtime/src/shard/impl_parts/journal_helpers.rs:147`
- **Confidence**: confirmed

## Description

`journal_sequences` is used by `append_journal_event` as the next event sequence to assign. `write_snapshot_for_run` adds one to that value for the snapshot, then stores the snapshot sequence back as the next sequence, causing the next journal event to reuse the same sequence as the snapshot.

## Evidence

Event appends treat `journal_sequences[run]` as the next event sequence:

```rust
// journal_helpers.rs:13-31
let seq = self.journal_sequence_for(run);
let next_seq = seq.get().checked_add(1).map(EventSeq::new) ...?;
self.journal.append_sequenced(event, seq)?;
self.journal_sequences.insert(run, next_seq);
```

Snapshot writes skip one value, then store the snapshot sequence itself:

```rust
// journal_helpers.rs:147-155
let current_seq = self.journal_sequence_for(run);
let snapshot_seq = match current_seq
    .get()
    .checked_add(1)
    .map(vb_storage::EventSeq::new)
{
    Some(seq) => seq,
    None => { ... }
};

// journal_helpers.rs:205-210
Ok(()) => {
    self.journal_sequences.insert(run, snapshot_seq);
    SnapshotWriteOutcome::Written
}
```

Example: after writing event sequence 0, `journal_sequences[run]` is 1. The snapshot writes at sequence 2 and stores 2. The next event append reads 2 and writes event sequence 2, reusing the snapshot sequence.

## Adversarial Check

The local comment says the snapshot sequence must be strictly greater than prior journal events, but the code must also make later journal events strictly greater than the snapshot. Because the same `journal_sequences` map drives later event appends, storing `snapshot_seq` instead of the next value violates that monotonic handoff. If recovery replays events after a snapshot by sequence, an event with the same sequence as the snapshot is at risk of being skipped or ordered ambiguously.

## Suggested Fix

Treat `journal_sequences` consistently as the next assignable sequence. Use `current_seq` for the snapshot if it is already the next sequence, then store `current_seq + 1` after a successful snapshot write. If snapshots intentionally require a gap after events, store `snapshot_seq + 1` after success so later journal events cannot reuse the snapshot sequence.
