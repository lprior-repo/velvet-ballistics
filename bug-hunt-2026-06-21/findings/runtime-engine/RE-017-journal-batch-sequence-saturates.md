# RE-017: Journal batch sequence arithmetic saturates and can duplicate `EventSeq`

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_runtime/src/journal/chunk_001.rs:275-278`, `crates/vb_runtime/src/journal/chunk_002.rs:340-343`
- **Confidence**: confirmed

## Description

Both batch append implementations derive per-event sequence numbers with `saturating_add`. If `seq_start + offset` exceeds the representable sequence range, later events are assigned the same saturated `EventSeq` instead of returning an overflow error.

## Evidence

Default trait implementation at `crates/vb_runtime/src/journal/chunk_001.rs:275-278`:

```rust
for (offset, event) in events.iter().enumerate() {
    let offset_u64 = u64::try_from(offset).map_err(|_| RuntimeError::EncodeFailed)?;
    let seq = EventSeq::new(seq_start.get().saturating_add(offset_u64));
    self.append_sequenced(event.clone(), seq)?;
}
```

Storage batch implementation at `crates/vb_runtime/src/journal/chunk_002.rs:340-343` repeats the same arithmetic:

```rust
for (offset, event) in events.iter().enumerate() {
    let offset_u64 = u64::try_from(offset).map_err(|_| RuntimeError::EncodeFailed)?;
    let seq = EventSeq::new(seq_start.get().saturating_add(offset_u64));
    let storage_event = Self::storage_event(event.clone(), seq)?;
```

For `seq_start == u64::MAX` and a two-event batch, both events after saturation use `EventSeq::new(u64::MAX)`. The method contract in `chunk_001.rs:253-260` says the first event is `seq_start`, the next is `seq_start + 1`, and behavior is deterministic; saturation violates that contract and can collapse multiple events onto one sequence key.

## Adversarial Check

This is not theoretical unbounded math. The code uses a bounded integer, explicitly exposes `EventSeq::new(seq_start.get()...)`, and chooses saturating arithmetic. Saturation is safe for counters where "at least max" is meaningful; journal sequence numbers are identifiers. Duplicating an identifier is corruption, not a graceful degradation.

## Suggested Fix

Use `checked_add` and return a dedicated overflow error before appending or staging any event. For the storage batch path, perform the full sequence preflight before `batch.append_event` so overflow preserves the all-or-nothing contract.
