# Codebase Map — vb-qi37.12.2

bead_id: vb-qi37.12.2
phase: 2
attempt: 1-of-7

## Scope
- Crate: `vb_runtime`
- Production files:
  - `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
  - `crates/vb_runtime/src/shard/types.rs`
  - `crates/vb_runtime/src/error/conversions.rs`
- Test file:
  - `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs`

## Relevant APIs
- `Shard::handle_resume`
- `ResumeError`
- `RuntimeError::StorageJournalAppend`
- runtime journal append path via `append_journal_event`

## Risk tags
- reliability
- storage-journal
- error-propagation
- release-plan
- runtime
