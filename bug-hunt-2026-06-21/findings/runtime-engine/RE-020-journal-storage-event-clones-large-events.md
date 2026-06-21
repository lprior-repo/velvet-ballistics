# RE-020: `storage_event` clones large runtime events before matching the real variant

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_runtime/src/journal/chunk_002.rs:267-274`
- **Confidence**: confirmed

## Description

`StorageRuntimeJournal::storage_event` clones the full `RuntimeJournalEvent` for each converter stage. Large events such as `SlotWritten` and `ActionCompletedEnvelope` carry `Vec<u8>` payloads, so the conversion path can allocate and copy large encoded values multiple times before reaching the matching branch.

## Evidence

`crates/vb_runtime/src/journal/chunk_002.rs:267-274`:

```rust
fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent> {
    if let Some(storage_event) = Self::run_storage_event(event.clone(), seq) {
        return Ok(storage_event);
    }
    if let Some(storage_event) = Self::action_storage_event(event.clone(), seq) {
        return Ok(storage_event);
    }
    match Self::boundary_storage_event(event.clone(), seq)? {
```

`RuntimeJournalEvent::SlotWritten` carries encoded value bytes and optional extra bytes at `crates/vb_runtime/src/journal/chunk_001.rs:154-166`:

```rust
SlotWritten {
    run: RunId,
    slot: SlotIdx,
    value: Vec<u8>,
    taint: Taint,
    extra: Option<Vec<u8>>,
},
```

A `SlotWritten` event is cloned for `run_storage_event`, cloned again for `action_storage_event`, and cloned a third time for `boundary_storage_event`, even though only the boundary converter can handle it.

## Adversarial Check

This is not a cold-path micro-optimization. Slot-written journaling sits on the deterministic evidence path, and the payload is already encoded bytes. Copying those bytes repeatedly adds allocator pressure exactly where the runtime is trying to persist evidence. The code can be made simpler and cheaper with a single match.

## Suggested Fix

Replace the staged clone-based converter chain with one `match event` that constructs the storage event directly. If helper functions are kept, make them borrow `&RuntimeJournalEvent` and clone only the fields required for the matched output.
