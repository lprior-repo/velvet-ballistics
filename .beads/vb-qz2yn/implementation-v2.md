# vb-qz2yn Implementation v2

## Approach

Changed ONLY the default artifact submission APIs (`submit_artifact` and `submit_artifact_with_contracts`). The CLI `RunSubmission` path and `ExecutorCatalog` packed path remain as separate parent follow-ups.

## Architecture

### Counter Storage

- New dedicated `artifact_acceptance` keyspace in `FjallJournal`
- Single fixed key: `last_issued` (ASCII prefix `0x40` + name)
- Value: u64 big-endian 8 bytes (0 = no artifacts issued yet)
- Malformed value length → `MalformedKeyspaceRow` error
- u64 overflow → `SequenceOverflow` error

### Sequence Type

- Private `IssuedArtifactSequence` wrapper (wraps `u64` with non-zero guarantee)
- Not exported publicly — only `EventSeq` appears in the public API
- `to_event_seq()` converts to the embedded `accepted_at_seq` field

### Transactional Pattern

```
submit_artifact_with_contracts:
  1. prepare_compiled_artifact (pure, seq=0, validates)
  2. acquire write_lock
  3. create OwnedWriteBatch
  4. issue_next_artifact_sequence (stages counter + returns seq)
  5. stamp accepted_at_seq on artifact
  6. bind_artifact_digest (rebind after stamp)
  7. encode with existing encode_record (MAGIC_COMPILED_ARTIFACT, etc.)
  8. batch.put_compiled_ir(staged record)
  9. commit with Strict durability (SyncAll) or default
  10. drop write_lock
  11. verify_artifact_persisted (existing readback)
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

1. **Private batch API**: Uses `journal.batch()` internally but does NOT expose a new public batch API. No `pub fn` added to `FjallJournal`.

2. **Counter staged in same batch as record**: The counter update and compiled IR insert are committed atomically in one `OwnedWriteBatch`. No separate counter persistence.

3. **Strict durability via batch**: For `RuntimePolicy::Strict`, calls `batch.strict()` before commit, which sets `durability(Some(PersistMode::SyncAll))`. The existing `persist_strict` is NOT called separately — the batch handles durability.

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

- Existing `persist_strict` fault hook: NOT used in new path (durability via batch)
- Existing `compiled_ir_readback` hook: still active
- Existing `batch_commit` hook: now active for this path
- No new test hooks added (implementation phase; tests written by root after this)

## Limitations (intentional)

- Counter helper method is ~45 logical lines, exceeds the strict 25-line target. This is because the counter read requires parsing the value, validating length, and returning the appropriate typed error. The logic is simple but the error handling expands it.
- No public API changes — callers still pass `workflow`, `policy`, `action_contracts` and receive `AcceptedArtifact`. The sequence number is embedded in the artifact, not returned separately.
