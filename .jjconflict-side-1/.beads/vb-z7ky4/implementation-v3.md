# vb-z7ky4 Implementation Notes (v3)

## Corrections Applied

This version applies coordinator-inspected corrections to the previous implementation.

### 1. Method Renamed to Match Interface

- `append_policy_decision_strict_inner` renamed to `pub(crate) fn append_policy_decision_strict`
- File: `crates/vb_runtime/src/shard/impl_parts/policy_decision.rs`
- No wrapper; direct implementation on `Shard`

### 2. Ownership Guard Visibility

- `require_pending_action_ownership` changed from `fn` to `pub(super) fn`
- File: `crates/vb_runtime/src/shard/lifecycle/chunk_004.rs`
- Enables sibling module `impl_parts/policy_decision.rs` to call the exact ownership guard used elsewhere
- No behavior change

### 3. Pre-Append Checks Preserved

Order of operations in `Shard::append_policy_decision_strict`:
1. Verify pending ticket ownership (`require_pending_action_ownership`)
2. Get current sequence (`journal_sequence_for`)
3. Compute checked next sequence (`checked_next_seq`) — returns error on overflow
4. Reserve map capacity (`reserve_journal_sequence_slot`)
5. Call journal strict port (`journal.append_policy_decision_strict`)
6. Publish next sequence (`journal_sequences.insert`) — only after success

If any step 1-5 fails, pending state and sequence map remain unchanged.

### 4. Size and API Constraints

- `append_policy_decision_strict`: 8 logical lines, 2 args
- `checked_next_seq`: 4 logical lines, 1 arg
- No `panic`, `unwrap`, `expect`, or unchecked arithmetic
- Uses typed `RuntimeResult` throughout

## Files Modified

- `crates/vb_runtime/src/shard/impl_parts/policy_decision.rs` — method rename
- `crates/vb_runtime/src/shard/lifecycle/chunk_004.rs` — visibility change

## Files Unchanged (verified correct)

- `crates/vb_runtime/src/journal/chunk_001.rs` — trait definition with default `UnsupportedOperation`
- `crates/vb_runtime/src/journal/chunk_002.rs` — `StorageRuntimeJournal` uses `FjallJournal::append_strict`
- `crates/vb_runtime/src/journal/chunk_003.rs` — `QueuedStorageRuntimeJournal` drains prefix, appends strictly, queue remains accepting
- `crates/vb_runtime/src/shard/impl_.rs` — include statement

## Not Yet Done

- Tests (coordinator will run real gates)
- No compile/verification commands run yet