# RS-001: Coalesce buffer flush assigns contiguous sequences across interleaved runs, corrupting per-run journal sequences

- **Severity**: Critical
- **Category**: correctness / bug
- **Location**: `crates/vb_runtime/src/shard/impl_parts/journal_helpers.rs:69-93`
- **Confidence**: confirmed

## Description

`flush_coalesce_buffer` uses the *first* buffered event's per-run sequence as the batch start and calls `journal.append_sequenced_batch(&events, first_seq)`. The batch implementation (`crates/vb_runtime/src/journal/chunk_001.rs:265-281` and `chunk_002.rs:324-351`) assigns *contiguous* sequences `seq_start, seq_start+1, seq_start+2, …` to the supplied slice. When the coalesce buffer contains events for more than one run (the common case under default `coalesce_window_ticks: 10`), events from non-first runs are persisted at sequences that do not match the per-run sequence the shard already advanced in `journal_sequences`.

## Evidence

`append_journal_event` advances `journal_sequences[run]` *before* the flush and records the run-specific starting sequence in the buffer:

```rust
// journal_helpers.rs:13-32
let seq = self.journal_sequence_for(run);           // per-run current seq
let next_seq = seq.get().checked_add(1)…;
if self.current_coalesce_window_remaining > 0 {
    self.coalesce_buffer.push((event, seq));        // buffer carries per-run seq
    self.journal_sequences.insert(run, next_seq);   // advanced BEFORE flush
}
```

`flush_coalesce_buffer` then ignores every buffered `(event, seq)` pair except the first, and hands a contiguous-sequence batch to the journal:

```rust
// journal_helpers.rs:74-89
let events: Vec<RuntimeJournalEvent> = self.coalesce_buffer.iter()
    .map(|(event, _seq)| event.clone()).collect();
let first_seq = self.coalesce_buffer.first()
    .map(|(_, seq)| *seq).unwrap_or(EventSeq::ZERO);
self.journal.append_sequenced_batch(&events, first_seq)?;
```

The default `ShardConfig::default()` ships `coalesce_window_ticks: 10` (`config.rs:111`), so this code path is the production default, not an opt-in.

Concrete corruption: with `journal_sequences = {A:5, B:3}` and buffer = `[(A5,seq=5), (B3,seq=3), (A6,seq=6)]`, the flush writes the three events at sequences `5, 6, 7`. Run B's event lands at sequence 6 (expected 3) and run A's second event lands at 7 (expected 6). On recovery, replay for run B looks for seq=3 and finds nothing; replay for run A sees a phantom gap at 6.

The contract documented at `journal/chunk_001.rs:253-260` ("the first event is assigned `seq_start`, the next `seq_start + 1`") only holds when every event in the batch belongs to a single run with contiguous sequences — a property the buffer does not enforce.

## Adversarial Check

A defender might argue that the coalesce window is sized so that only one run is active per tick, or that `dispatch` only processes one command per tick (`dispatch.rs:31` pops exactly one command). Neither holds: `dispatch_command` for one Submit can append many events (`RunSubmitted`, `RunAdmission`, `StepStarted`, …) for one run, but two ticks within the same window can dispatch commands for *different* runs (Submit for run B in tick 2 while run A's events are still buffered from tick 1). The default window of 10 makes this inevitable under any non-trivial workload. The fix is not "single-run buffer" — it is to flush per run or pass per-event sequences.

## Suggested Fix

Either flush the coalesce buffer grouped by run, preserving each run's recorded starting sequence, or change the journal API to accept per-event `(event, seq)` pairs and commit them in a single `JournalWriteBatch`:

```rust
pub(crate) fn flush_coalesce_buffer(&mut self) -> RuntimeResult<()> {
    if self.coalesce_buffer.is_empty() { return Ok(()); }
    // Group by run, preserving each run's earliest seq, then commit each
    // group with append_sequenced_batch inside one storage batch.
    …
}
```

The simplest correct fix: change `RuntimeJournal::append_sequenced_batch` to accept `&[(RuntimeJournalEvent, EventSeq)]` and let the storage layer key each event by its own sequence (the Fjall batch already supports per-event keys).
