# vb-qz2yn: Default Artifact Submission Sequence Counter

## Scope

This bead changes ONLY the default artifact submission APIs:
- `submit_artifact`
- `submit_artifact_with_contracts`

It does NOT change:
- CLI `RunSubmission` path (parent bead vb-xxxxx)
- `ExecutorCatalog` packed path (parent bead vb-xxxxx)
- Raw `put_compiled_ir` (remains raw storage, no counter)
- Existing test suite (tests written after this implementation)

## Requirements

### 1. Sequence Type
- Private `IssuedArtifactSequence` wrapping `NonZeroU64` (or equivalent validated private type)
- NO public `next_sequence`, setters, or `caller_at_seq` API
- First issued sequence is 1 (checked_add from 0)

### 2. Metadata Keyspace
- NEW dedicated `artifact_acceptance` metadata keyspace in `FjallJournal`
- `artifact_acceptance` added to `open` keyspace creation, `declared_keyspaces`, and `probe_health`
- `declared_keyspaces()` returns 12 elements (11 old + 1 new)
- Single private fixed key: `last_issued`
- Value: u64 big-endian 8 bytes
- Absent key means 0 (no artifacts issued yet)
- Malformed value length → existing `MalformedKeyspaceRow` error
- u64 overflow → existing `SequenceOverflow` error
- No reserved compiled hash key

### 3. Transactional Update Pattern
- One private journal helper method performs the atomic counter increment
- Counter read, stamp/rebind, encoding, and batch commit happen within ONE `OwnedWriteBatch`
- Strict policy: `durability(Some(PersistMode::SyncAll))` on that batch
- Counter helper persists ONLY within that batch — no independent persistence
- The helper must NOT be a public API extension (no new `pub fn` on `FjallJournal`)

### 4. Issuance Flow
Within `submit_artifact_with_contracts`:
1. Prepare and validate artifact (existing logic, pure, seq=0)
2. Read last issued typed value
3. Increment and stamp `EventSeq`
4. Use existing `bind_artifact_digest` to set BOTH `digest` and `verification.digest`
5. Encode with existing `encode_record` (MAGIC_COMPILED_ARTIFACT, RecordKind::CompiledIr, 0, MAX_COMPILED_IR_BYTES)
6. Construct proper `CompiledIrRecord` and stage with existing `batch.put_compiled_ir`
7. Stage the updated counter in the same batch
8. Commit the batch (durability per policy)
9. Existing readback verification after commit

### 5. Locking and Concurrency
- Internal issuance holds the actual `write_lock` mutex continuously across:
  - Read counter → stamp/rebind → encode → same-batch commit
- `write_lock` provides authority RMW serialization for artifact acceptance
- Not all journal writes use the lock — claim limited to artifact acceptance path only

### 6. Error and Fault Hooks
- Existing `persist_strict` fault hook remains active for Strict policy
- Existing compiled IR readback hook remains active
- Batch commit hook honors the new batch path
- Add private `#[cfg(test)]` pre-commit hook(s) matching actual error variants
- No production flags or waivers for test hooks
- Validation, overflow, and encoding failures happen before durable writes
- Readback IO errors after commit → do NOT claim rollback for all Err

### 7. Implementation Constraints
- New private helper method(s) ≤ 25 logical lines each
- Existing files ≤ 300 lines (except already-oversized unchanged files)
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `dbg`
- No unchecked indexing, slicing, casts, or arithmetic
- Safe fixed array TryFrom with `map_err`; checked arithmetic for u64 increment
- No new dependencies or lint waivers
- No new `pub fn` or `pub struct` on `FjallJournal` (private helper only)

### 8. Allowed Files
- `crates/vb_storage/src/constants.rs`
- `crates/vb_storage/src/admission.rs`
- `crates/vb_storage/src/journal/mod.rs`
- `crates/vb_storage/src/journal/core.rs`
- `crates/vb_storage/src/journal/readonly.rs`
- `crates/vb_storage/src/journal/artifact_acceptance.rs` (NEW)
- `crates/vb_storage/src/admission/artifact_acceptance.rs` (NEW if needed)

## Acceptance Criteria

1. `cargo build --release --all` succeeds
2. All existing tests pass unchanged
3. Counter increments monotonically across repeated submissions
4. First submission gets EventSeq(1), second gets EventSeq(2), etc.
5. No public API changes to `submit_artifact` or `submit_artifact_with_contracts`
6. Readback verification still works after counter integration
