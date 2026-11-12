# Semantic Audit: vb-fyktj Action Index Own-Patch

## Scope

- Historical commit: `bd2920512bde46a801fc5663f392d3362272a525` (vb-3wn7x)
- Current frozen source: `0dd55b94cbdd05c21951e6e239313d393816a99d`
- Files in scope: 9 actual crate delta files (excluded 33877 conflict artifacts)
- Actual Git unified hunks: 14

## Summary

All 14 historical hunks are semantically present in the current codebase. The action index maintenance functionality introduced by vb-3wn7x has been preserved and refactored into a broader recovery index subsystem. No production behavior from the historical commit is missing.

## Per-Hunk Classification

### Hunk 1: `crates/vb_storage/src/batch/action_index.rs` (new file)

**Classification: Semantically present (refactored)**

Historical: Added `stage_pending_action_index_op` with vb-3wn7x-specific doc comments.
Current: File exists with same core function `stage_pending_action_index_op`, now wrapped by `stage_recovery_index_ops`. Doc comments updated to reflect broader recovery index scope.

- Function signature identical
- Event classification logic (`pending_action_index_op_for`) unchanged
- Insert/remove semantics preserved
- All event variants handled identically

### Hunk 2: `crates/vb_storage/src/batch/append_event.rs`

**Classification: Semantically present (renamed call)**

Historical: `self.journal.stage_pending_action_index_op(&mut self.inner, event)?;`
Current: `self.journal.stage_recovery_index_ops(&mut self.inner, event)?;`

Call location shifted from after inner insert to before inner insert (after byte admission check). This is a stricter ordering that ensures index staging fails the batch before the event row is written.

### Hunk 3: `crates/vb_storage/src/batch/mod.rs`

**Classification: Present**

`mod action_index;` declared in current source.

### Hunk 4: `crates/vb_storage/src/batch/putters.rs`

**Classification: Present**

Both `put_action_index` and `delete_action_index` methods exist in current `JournalWriteBatch` implementation with identical signatures and semantics.

### Hunk 5: `crates/vb_storage/src/index_maintenance_tests.rs` (new file)

**Classification: Semantically present (expanded)**

Historical: 601 lines of tests.
Current: 647 lines of tests (46 lines more).

All 20 historical tests preserved. Additional test `batch_append_event_run_accepted_does_not_fabricate_admission_indexes` added to verify RunAccepted doesn't synthesize indexes. Import list updated to include `IndexStatusState` and `WorkflowId`.

### Hunk 6: `crates/vb_storage/src/indexes.rs`

**Classification: Present**

Both `put_action_index` and `delete_action_index` methods on `FjallJournal` exist with identical semantics.

### Hunk 7: `crates/vb_storage/src/journal/internal.rs` (doc comment)

**Classification: Semantically present (reworded)**

Historical doc comment explicitly mentioned vb-3wn7x and listed all event variants. Current doc comment is more concise but preserves the atomic batch semantics description. Function name changed from `append_journaled` to `append_unfsynced`.

### Hunk 8: `crates/vb_storage/src/journal/internal.rs` (code)

**Classification: Semantically present (renamed call)**

Historical: `self.stage_pending_action_index_op(&mut batch, event)?;`
Current: `self.stage_recovery_index_ops(&mut batch, event)?;`

Same atomic batch semantics preserved.

### Hunk 9: `crates/vb_storage/src/lib.rs`

**Classification: Present**

`index_maintenance_tests` module declared with vb-3wn7x comment preserved.

### Hunks 10-14: `crates/workspace_tests/tests/journal_side_index_contracts.rs`

**Classification: Present**

All 5 hunks add proptests for side-index atomicity contracts (PO-002, PO-004, PO-008, PO-009, PO-010, PO-011, PO-012, PO-013, PO-014). All tests exist in current source with vb-3wn7x annotations preserved where relevant.

## Missing Items

None. All production behavior and test coverage from the historical commit is present in the current codebase.

## Refactoring Notes

1. `stage_pending_action_index_op` → `stage_recovery_index_ops` (wrapper) + `stage_pending_action_index_op` (preserved)
2. `append_journaled` → `append_unfsynced`
3. Test file expanded from 601 to 647 lines with additional RunAccepted negative test
4. Import lists updated for new dependencies

## Conclusion

The vb-3wn7x action index maintenance patch is fully integrated into the current codebase. The refactoring to a broader "recovery index" subsystem preserved all original behavior while improving the API surface. No further integration work is required for this bead's scope.