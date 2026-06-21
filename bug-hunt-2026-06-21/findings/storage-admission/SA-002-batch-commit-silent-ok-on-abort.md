# SA-002: `JournalWriteBatch::commit` silently returns `Ok(())` for an aborted batch

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_storage/src/batch/write.rs:77-83`
- **Confidence**: confirmed

## Description

When the batch state is `Aborted`, `commit` returns `Ok(())` without persisting anything. Callers cannot distinguish a successful commit from a previously-aborted batch — both look identical. This converts what should be a loud "your data was not written" signal into a silent success.

## Evidence

```rust
// crates/vb_storage/src/batch/write.rs:77-83
pub fn commit(self) -> Result<(), JournalError> {
    if self.state.is_aborted() {
        return Ok(());
    }
    self.inner.commit()?;
    Ok(())
}
```

`state.is_aborted()` becomes true when an upstream write method (e.g. `put_workflow_source` line 25, `put_blob` line 83, `append_event` line 20, `put_compiled_ir` test helper line 14) hit a digest mismatch, duplicate, or encoding error. The caller may have caught that error and decided to "commit anyway" expecting it to surface the failure, but instead the commit silently no-ops.

## Adversarial Check

The intended contract is "callers must propagate the upstream `Err`". That is a reasonable convention, but the type system does not enforce it: `commit` is `pub fn commit(self) -> Result<(), JournalError>` with no marker indicating prior abort. The `len()` method (line 43-49) already returns 0 when aborted, which a defensive caller could check — but the API surface encourages calling `commit` directly. The combination of (a) silent Ok-on-abort and (b) batch-staging methods that DO abort (per SA-001's siblings) and DO propagate Err means a careless caller pattern like `if let Err(e) = batch.put_blob(...) { warn!(?e); } batch.commit()?;` will silently drop the put without warning. Other storage engines (RocksDB, Fjall itself) surface this as an `Err(InvalidArgument)` or similar.

## Suggested Fix

Return a dedicated error variant:

```rust
pub fn commit(self) -> Result<(), JournalError> {
    if self.state.is_aborted() {
        return Err(JournalError::BatchAborted);
    }
    self.inner.commit()?;
    Ok(())
}
```

If silent success is genuinely desired for some workflows, gate it behind an explicit `commit_or_discard` method.
