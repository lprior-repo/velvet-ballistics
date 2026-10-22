# vb-qz2yn Implementation v2

## Approach

Changed ONLY the default artifact submission APIs (`submit_artifact` and `submit_artifact_with_contracts`). The CLI `RunSubmission` path and `ExecutorCatalog` packed path remain as separate parent follow-ups.

## Architecture

### Counter Storage

- New dedicated `artifact_acceptance` keyspace in `FjallJournal`
- Single private literal key: `b"last_issued"` (no prefix; keyspace isolates)
- Value: u64 big-endian 8 bytes (0 = no artifacts issued yet)
- Malformed value length → `MalformedKeyspaceRow` error
- u64 overflow → `SequenceOverflow` error
- Safe fixed array `TryFrom` for 8-byte slice→array conversion

### Sequence Type

- Private `IssuedArtifactSequence` wrapping `std::num::NonZeroU64`
- Checked constructor validates via `NonZeroU64::new(value).ok_or(...)`
- Not exported publicly — only `EventSeq` appears in the public API
- `to_event_seq()` converts to the embedded `accepted_at_seq` field

### Transactional Pattern

Private issuance flows through 5 small private functions, each ≤ 25 lines:

```
issue_artifact(journal, artifact, policy):
  1. journal.acquire_write_lock()
  2. last = read_counter(journal)
  3. (next, stamped) = prepare_artifact(last, artifact)
  4. (record, encoded) = encode_record(stamped)
  5. batch = build_batch(journal, next, stamped, encoded)
  6. commit_batch(journal, batch, policy)
  7. journal.release_write_lock()
  8. return stamped

issue_next_artifact_sequence(journal, artifact, policy)
  → pub(crate) wrapper delegating to issue_artifact
```

### Locking

- `write_lock` held continuously from batch creation through commit
- Provides RMW serialization for artifact acceptance counter
- Not all journal writes use the lock — limited to this path

## Files Changed

| File | Change |
|------|--------|
| `constants.rs` | Added `KEYSPACE_ARTIFACT_ACCEPTANCE`, `PREFIX_ARTIFACT_ACCEPTANCE`, `ARTIFACT_ACCEPTANCE_LAST_ISSUED` |
| `journal/core.rs` | Added `artifact_acceptance` keyspace field, `open` creates it, updated `declared_keyspaces()` to 12, updated `probe_health()` |
| `journal/readonly.rs` | Updated `declared_keyspaces()` return type to 12 |
| `journal/artifact_acceptance.rs` | NEW — private counter read/increment/stamp helpers |
| `journal/mod.rs` | Registered `artifact_acceptance` module |
| `admission.rs` | Updated `submit_artifact_with_contracts` to use counter within batch transaction |

## Key Design Decisions

1. **Private batch API**: No public batch API exposed. Uses raw `fjall::OwnedWriteBatch` internally via `build_batch` helper. No `pub fn` added to `FjallJournal`.

2. **Counter staged in same batch as record**: The counter update and compiled IR insert are committed atomically in one `fjall::OwnedWriteBatch`. No separate counter persistence.

3. **Strict durability via batch**: For `RuntimePolicy::Strict`, sets `batch = batch.durability(Some(fjall::PersistMode::SyncAll))` before commit. No separate `persist_strict` call — the batch handles durability.

4. **No counter key reserved hash**: Only the `last_issued` key exists. No per-artifact counter keys.

5. **Digest rebind after stamp**: The artifact's `accepted_at_seq` is set, then `bind_artifact_digest` is called to recompute the digest (which includes the sequence number).

6. **Readback verification preserved**: After commit, the existing `verify_artifact_persisted` readback still runs. If it fails, that's an IO error post-commit (not a rollback scenario).

## Error Handling

- Counter read failure (IO) → propagates
- Counter value malformed → `MalformedKeyspaceRow`
- u64 overflow → `SequenceOverflow`
- Artifact validation fails (prepare) → never reaches counter
- Batch commit failure → propagates (IO or `BatchAborted`)
- Readback failure → `ArtifactMalformed` or wrapped `CompiledIrReadback`

## Test Hook Compatibility

- `persist_strict` fault hook: honored for Strict policy (checked before durability setting)
- `batch_commit` fault hook: honored for all policies (checked before batch commit)
- Both hooks are private `#[cfg(test)]` methods on `FjallJournal`
- Existing `compiled_ir_readback` hook: still active for readback verification

## Limitations (intentional)

- Issuance logic split across 5 private functions to maintain ≤ 25 lines each. Total issuance flow is ~80 logical lines.
- No public API changes — callers still pass `workflow`, `policy`, `action_contracts` and receive `AcceptedArtifact`. The sequence number is embedded in the artifact, not returned separately.
- Readback verification after commit may still fail due to IO errors post-acknowledgment; this is an IO error, not a rollback scenario.
