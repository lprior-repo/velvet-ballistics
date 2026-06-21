# SA-001: `put_run_header` and `put_snapshot` do not abort the batch on encode failure, breaking all-or-nothing

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_storage/src/batch/write_records.rs:53-78`
- **Confidence**: confirmed

## Description

The two batch-staging methods `put_run_header` and `put_snapshot` propagate encode errors via `?` but never set `self.state = BatchState::Aborted`. Their siblings (`put_workflow_source`, `put_blob`) abort the batch on the same error class. A subsequent `commit()` (`crates/vb_storage/src/batch/write.rs:77-83`) treats the batch as still-Open and commits whatever was previously staged — a partial write that violates the cross-keyspace atomicity the batch type advertises.

## Evidence

```rust
// crates/vb_storage/src/batch/write_records.rs:53-64  -- missing abort
pub fn put_run_header(&mut self, record: &RunHeaderRecord) -> Result<(), JournalError> {
    let key = run_header_key(record.run)?;
    let value = encode_record(
        MAGIC_INDEX_RECORD,
        RecordKind::RunHeader,
        record.run.get(),
        record,
        MAX_RUN_HEADER_BYTES,
    )?;                                     // <-- no state = Aborted on Err
    self.inner.insert(&self.journal.run_header, key, value);
    Ok(())
}
```

Compare with `put_workflow_source` (`crates/vb_storage/src/batch/write_records.rs:18-50`), which is identical in shape but sets `self.state = BatchState::Aborted;` in every error arm. The asymmetry is uniform across the file: `put_run_header`/`put_snapshot` skip the abort, `put_workflow_source`/`put_blob` apply it.

`JournalWriteBatch::commit` (`crates/vb_storage/src/batch/write.rs:77-83`) returns `Ok(())` for both `Open` and `Aborted` states (separate finding), but for `Open` it commits the inner batch unconditionally:

```rust
pub fn commit(self) -> Result<(), JournalError> {
    if self.state.is_aborted() { return Ok(()); }
    self.inner.commit()?;
    Ok(())
}
```

So after a failed `put_run_header`, a subsequent successful `append_event` followed by `commit` will commit the event but not the run header — half of an atomic operation.

## Adversarial Check

The most plausible counter-argument is that callers should treat any `Err` from a batch method as fatal and never call `commit` afterwards. But the type's own API contradicts this: `BatchState::Aborted` exists precisely so that subsequent `commit` calls become safe no-ops, and the other write methods in the same file use it for exactly this purpose. A caller that follows the documented pattern ("on Err, abort and commit") works correctly for `put_workflow_source`/`put_blob` and silently corrupts for `put_run_header`/`put_snapshot`. This is a real atomicity defect.

## Suggested Fix

Mirror the abort pattern from `put_workflow_source`:

```rust
pub fn put_run_header(&mut self, record: &RunHeaderRecord) -> Result<(), JournalError> {
    let key = match run_header_key(record.run) {
        Ok(k) => k,
        Err(e) => { self.state = BatchState::Aborted; return Err(e); }
    };
    let value = match encode_record(MAGIC_INDEX_RECORD, RecordKind::RunHeader,
                                    record.run.get(), record, MAX_RUN_HEADER_BYTES) {
        Ok(v) => v,
        Err(e) => { self.state = BatchState::Aborted; return Err(e); }
    };
    self.inner.insert(&self.journal.run_header, key, value);
    Ok(())
}
```

Apply the same fix to `put_snapshot`.
