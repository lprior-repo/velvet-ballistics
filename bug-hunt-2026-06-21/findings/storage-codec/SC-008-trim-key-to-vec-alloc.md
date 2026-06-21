# SC-008: `trim_events_for_run` allocates `key.to_vec()` per trim candidate

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_storage/src/trimming/logic.rs:87`
- **Confidence**: confirmed

## Description

The trim loop calls `batch.remove(&self.events, key.to_vec())` for every event being removed, allocating a fresh `Vec<u8>` per iteration. The Fjall `OwnedWriteBatch::remove` accepts `Into<Vec<u8>>`, but the underlying key bytes are already available as a borrowed slice inside `item.key()` — there is no need to copy them into a heap-allocated `Vec` solely to pass them across the API boundary.

## Evidence

```rust
// crates/vb_storage/src/trimming/logic.rs:84-89
let seq_u64 = u64::from_be_bytes(seq_bytes);

if seq_u64 < cutoff_seq.get() {
    batch.remove(&self.events, key.to_vec());        // <-- heap alloc per removed event
    deleted_count = deleted_count.saturating_add(1);
}
```

For a run with thousands of trimmable events, this is thousands of small (17-byte) heap allocations per trim pass.

## Adversarial Check

The trim path runs in the background but takes the global journal write lock indirectly through Fjall. Each `Vec` allocation is small but the volume (bounded by `MAX_BATCH_COUNT = 10_000` per batch, repeatedly across all eligible runs) makes this a measurable allocator hit on hot LSM-tree paths. Whether Fjall offers a borrowed-key `remove` API is a downstream question; if not, a small-ring scratch buffer that reuses a single `Vec<u8>` and `clear()`s per iteration would eliminate the allocation pressure.

## Suggested Fix

Reuse a single scratch buffer outside the loop:

```rust
let mut key_buf: Vec<u8> = Vec::with_capacity(17);
for item in self.events.prefix(prefix_key) {
    let key = item.key().map_err(TrimError::from)?;
    ...
    if seq_u64 < cutoff_seq.get() {
        key_buf.clear();
        key_buf.extend_from_slice(key.as_ref());
        batch.remove(&self.events, key_buf.clone()); // or pass &key_buf if Fjall allows
        deleted_count = deleted_count.saturating_add(1);
    }
}
```
