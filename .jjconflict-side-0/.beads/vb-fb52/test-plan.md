# Test Plan: vb-fb52 — Atomic Journal and Index Write Batches

## Scope

`JournalWriteBatch<'j>` — a Fjall-backed atomic write batch that accumulates inserts
across nine keyspaces and commits them atomically with WAL fsync.

## Testing Trophy Distribution

| Layer | Count | Description |
|---|---|---|
| Unit tests | 18 | Isolated encoding, staging, len/invariant checks |
| Integration tests | 24 | Full commit across keyspaces, durability modes, process lock |
| Property-based (proptest) | 10 | Invariant checking across随机ised inputs |
| **Total** | **52** | |

---

## 1. Unit Tests — Encoding and Staging Logic

### 1.1 Structural Invariants

| # | Test name | Signature | Verifies |
|---|---|---|---|
| U1 | `new_batch_is_empty_with_zero_length` | `fn new_batch_is_empty_with_zero_length(journal: &FjallJournal)` | I2, I3: constructed batch has `len() == 0`, `is_empty() == true` |
| U2 | `len_increments_after_each_put_operation` | `fn len_increments_after_each_put_operation(journal: &FjallJournal)` | I3: each successful `put_*` increments `len()` by 1 |
| U3 | `is_empty_false_after_single_staged_put` | `fn is_empty_false_after_single_staged_put(journal: &FjallJournal)` | I2: `is_empty() == false` after any put |
| U4 | `batch_is_not_send_or_sync` | `fn batch_is_not_send_or_sync(journal: &FjallJournal)` | I1: `JournalWriteBatch` asserts `!Send + !Sync` bounds |

### 1.2 Encoding Invariants

| # | Test name | Signature | Verifies |
|---|---|---|---|
| U5 | `workflow_source_record_header_is_60_bytes` | `fn workflow_source_record_header_is_60_bytes()` | I5: `RECORD_HEADER_BYTES == 60` for workflow source |
| U6 | `blob_record_header_is_60_bytes` | `fn blob_record_header_is_60_bytes()` | I5: `RECORD_HEADER_BYTES == 60` for blob |
| U7 | `run_event_header_is_60_bytes` | `fn run_event_header_is_60_bytes()` | I5: `RECORD_HEADER_BYTES == 60` for run event |
| U8 | `run_event_magic_is_0x5642_4A45` | `fn run_event_magic_is_0x5642_4A45()` | I6: `MAGIC_JOURNAL_EVENT` exclusively on run_event |
| U9 | `workflow_source_magic_is_0x5642_5352` | `fn workflow_source_magic_is_0x5642_5352()` | I7: `MAGIC_WORKFLOW_SOURCE` exclusively on workflow_source |
| U10 | `compiled_ir_magic_is_0x5642_4952` | `fn compiled_ir_magic_is_0x5642_4952()` | I8: `MAGIC_COMPILED_ARTIFACT` exclusively on compiled_ir |
| U11 | `snapshot_magic_is_0x5642_534E` | `fn snapshot_magic_is_0x5642_534E()` | I9: `MAGIC_SNAPSHOT` exclusively on run_snapshot |
| U12 | `blob_magic_is_0x5642_424C` | `fn blob_magic_is_0x5642_424C()` | I10: `MAGIC_BLOB` exclusively on blob |
| U13 | `index_record_magic_is_0x5642_4958` | `fn index_record_magic_is_0x5642_4958()` | I11: `MAGIC_INDEX_RECORD` on run_header/index keyspaces |

### 1.3 Key Layout Invariants

| # | Test name | Signature | Verifies |
|---|---|---|---|
| U14 | `digest_keyed_record_key_is_33_bytes` | `fn digest_keyed_record_key_is_33_bytes(digest: &[u8; 32])` | I12: `[prefix_u8][32_byte_digest]` = 33 bytes |
| U15 | `run_event_key_is_17_bytes` | `fn run_event_key_is_17_bytes(run_id: u64, seq: u64)` | I13: `[0x11][run_id_8be][seq_8be]` = 17 bytes |
| U16 | `run_header_key_is_9_bytes` | `fn run_header_key_is_9_bytes(run_id: u64)` | I14: `[0x10][run_id_8be]` = 9 bytes |
| U17 | `run_snapshot_key_is_17_bytes` | `fn run_snapshot_key_is_17_bytes(run_id: u64, seq: u64)` | I15: `[0x12][run_id_8be][seq_8be]` = 17 bytes |

### 1.4 Digest Verification (BH-02)

| # | Test name | Signature | Verifies |
|---|---|---|---|
| U18 | `put_workflow_source_rejects_digest_mismatch` | `fn put_workflow_source_rejects_digest_mismatch(journal: &FjallJournal, record: WorkflowSourceRecord, forged_digest: Digest)` | I19: `PayloadDigestMismatch` when content does not hash to `record.digest` |

---

## 2. Integration Tests — Journal/Index Atomicity

### 2.1 Happy Path — Single Keyspace Commits

| # | Test name | Signature | Verifies |
|---|---|---|---|
| I1 | `batch_put_workflow_source_with_valid_digest_commits` | `fn batch_put_workflow_source_with_valid_digest_commits(journal: &FjallJournal, record: WorkflowSourceRecord)` | HP-2: source readable by digest after commit |
| I2 | `batch_put_blob_with_valid_digest_commits` | `fn batch_put_blob_with_valid_digest_commits(journal: &FjallJournal, record: BlobRecord)` | HP-3: blob readable by digest after commit |
| I3 | `batch_put_compiled_ir_commits_and_is_readable` | `fn batch_put_compiled_ir_commits_and_is_readable(journal: &FjallJournal, record: CompiledIrRecord)` | HP-4 variant: compiled_ir readable after commit |
| I4 | `batch_put_run_header_commits_and_is_readable` | `fn batch_put_run_header_commits_and_is_readable(journal: &FjallJournal, record: RunHeaderRecord)` | HP-4: header readable by run ID after commit |
| I5 | `batch_put_snapshot_commits_and_is_readable` | `fn batch_put_snapshot_commits_and_is_readable(journal: &FjallJournal, snapshot: SnapshotRecord)` | HP-5: snapshot readable by run+seq after commit |
| I6 | `batch_append_event_commits_and_is_readable` | `fn batch_append_event_commits_and_is_readable(journal: &FjallJournal, event: JournalEvent)` | HP-6 variant: event readable after commit |

### 2.2 Happy Path — Multi-Keyspace Atomicity

| # | Test name | Signature | Verifies |
|---|---|---|---|
| I7 | `batch_commits_across_multiple_keyspaces` | `fn batch_commits_across_multiple_keyspaces(journal: &FjallJournal)` | HP-1, I16: 8 operations across all keyspaces committed atomically |
| I8 | `batch_mixed_operations_across_keyspaces_commit_atomically` | `fn batch_mixed_operations_across_keyspaces_commit_atomically(journal: &FjallJournal)` | HP-7: 7 mixed ops; all keyspaces reflect writes |
| I9 | `batch_empty_commit_succeeds` | `fn batch_empty_commit_succeeds(journal: &FjallJournal)` | HP-8: empty batch `commit()` returns `Ok(())` without Fjall interaction |
| I10 | `empty_strict_batch_commit_succeeds` | `fn empty_strict_batch_commit_succeeds(journal: &FjallJournal)` | HP-9: empty strict batch `commit()` returns `Ok(())` |
| I11 | `batch_index_operations_increment_len_without_payloads` | `fn batch_index_operations_increment_len_without_payloads(journal: &FjallJournal)` | HP-10: index markers present after commit |
| I12 | `batch_strict_mode_commits_successfully` | `fn batch_strict_mode_commits_successfully(journal: &FjallJournal)` | HP-6, I18: strict batch with `SyncAll` returns `Ok` |

### 2.3 Error Path — Digest Mismatch (BH-02 Regression)

| # | Test name | Signature | Verifies |
|---|---|---|---|
| I13 | `batch_forged_workflow_source_digest_rejected` | `fn batch_forged_workflow_source_digest_rejected(journal: &FjallJournal, record: WorkflowSourceRecord, forged_digest: Digest)` | EP-1, I19: `PayloadDigestMismatch`; `len == 0` unchanged |
| I14 | `batch_forged_blob_digest_rejected` | `fn batch_forged_blob_digest_rejected(journal: &FjallJournal, record: BlobRecord, forged_digest: Digest)` | EP-2, I19: `PayloadDigestMismatch`; `len == 0` unchanged |
| I15 | `batch_put_blob_rejects_digest_mismatch` | `fn batch_put_blob_rejects_digest_mismatch(journal: &FjallJournal, record: BlobRecord)` | EP-4: `PayloadDigestMismatch`; `len == 0` after failed put |

### 2.4 Error Path — Process Lock

| # | Test name | Signature | Verifies |
|---|---|---|---|
| I16 | `second_journal_open_on_same_path_is_prevented_by_process_lock` | `fn second_journal_open_on_same_path_is_prevented_by_process_lock(journal_path: PathBuf)` | EP-5, I17: second `open()` returns `JournalError::ProcessLockHeld` |
| I17 | `no_keyspace_created_when_lock_fails` | `fn no_keyspace_created_when_lock_fails(journal_path: PathBuf)` | EP-6: directory unchanged after failed second open |
| I18 | `process_lock_file_removed_after_journal_close` | `fn process_lock_file_removed_after_journal_close(journal_path: PathBuf)` | Process lock cleanup on drop |

### 2.5 Error Path — Duplicate Events and Sequences

| # | Test name | Signature | Verifies |
|---|---|---|---|
| I19 | `append_strict_rejects_duplicate_event` | `fn append_strict_rejects_duplicate_event(journal: &FjallJournal, event: JournalEvent)` | EP-7, I20: `DuplicateEvent` on second append with same run_id+seq |
| I20 | `events_for_run_rejects_sequence_gap` | `fn events_for_run_rejects_sequence_gap(journal: &FjallJournal, run_id: u64, events: Vec<JournalEvent>)` | EP-8, I21: `SequenceGap` when replay detects non-contiguous sequence |
| I21 | `event_seq_overflow_rejected` | `fn event_seq_overflow_rejected(journal: &FjallJournal)` | EP-13: `SequenceOverflow` when `EventSeq::new(u64::MAX)` + 1 |

### 2.6 Error Path — Decoding Failures

| # | Test name | Signature | Verifies |
|---|---|---|---|
| I22 | `decode_rejects_all_zero_bytes` | `fn decode_rejects_all_zero_bytes()` | EP-9: `BadMagic` or `UnexpectedEof` on all-zero input |
| I23 | `decode_rejects_all_ff_bytes` | `fn decode_rejects_all_ff_bytes()` | EP-10: `BadMagic` on all-0xFF input |
| I24 | `decode_rejects_valid_header_with_corrupt_payload` | `fn decode_rejects_valid_header_with_corrupt_payload()` | EP-11: `PayloadDigestMismatch` when payload differs from digest |
| I25 | `decode_rejects_future_schema_version_in_full_record` | `fn decode_rejects_future_schema_version_in_full_record()` | EP-12: `UnsupportedSchemaVersion` |
| I26 | `crc_single_bit_flip_detected` | `fn crc_single_bit_flip_detected()` | EP-17: `HeaderChecksumMismatch` on single-bit flip in header |
| I27 | `encode_rejects_payload_at_boundary_with_small_max` | `fn encode_rejects_payload_at_boundary_with_small_max(record: Record, max_size: usize)` | EP-14: `PayloadTooLarge` when encoded payload exceeds per-record maximum |
| I28 | `header_decode_rejects_oversized_declared_payload` | `fn header_decode_rejects_oversized_declared_payload()` | EP-15: `PayloadTooLarge` when header declares size > remaining buffer |
| I29 | `header_decode_rejects_wrong_header_len` | `fn header_decode_rejects_wrong_header_len()` | EP-16: `HeaderLengthMismatch` when header length != 60 |

---

## 3. Property-Based Tests (proptest) — Invariant Checking

### 3.1 Structural Invariants

| # | Test name | Signature | Verifies |
|---|---|---|---|
| P1 | `len_equals_staged_count_after_random_operations` | `fn len_equals_staged_count_after_random_operations(journal: &FjallJournal, ops: Vec<BatchOp>)` | I2, I3: `len()` always equals actual staged operation count |
| P2 | `is_empty_equals_len_zero_invariant` | `fn is_empty_equals_len_zero_invariant(journal: &FjallJournal, ops: Vec<BatchOp>)` | I2: `is_empty() == (len() == 0)` holds after every operation |
| P3 | `batch_len_never_decreases` | `fn batch_len_never_decreases(journal: &FjallJournal, ops: Vec<BatchOp>)` | I3: `len()` monotonically increases (never decreases) |
| P4 | `commit_leaves_batch_in_consumed_state` | `fn commit_leaves_batch_in_consumed_state(journal: &FjallJournal, ops: Vec<BatchOp>)` | I4: after `commit()` succeeds, batch is consumed; caller must re-create |

### 3.2 Atomicity Invariants

| # | Test name | Signature | Verifies |
|---|---|---|---|
| P5 | `all_or_nothing_commit_across_keyspaces` | `fn all_or_nothing_commit_across_keyspaces(journal: &FjallJournal, ops: Vec<BatchOp>)` | I16: commit is all-or-nothing; no partial state visible |
| P6 | `strict_mode_requires_fsync_before_return` | `fn strict_mode_requires_fsync_before_return(journal: &FjallJournal, ops: Vec<BatchOp>)` | I18: strict durability guarantees fsync before `commit()` returns |
| P7 | `digest_verification_mandatory_on_workflow_source` | `fn digest_verification_mandatory_on_workflow_source(journal: &FjallJournal, record: WorkflowSourceRecord)` | I19: BLAKE3 digest verification cannot be skipped for `workflow_source` |
| P8 | `digest_verification_mandatory_on_blob` | `fn digest_verification_mandatory_on_blob(journal: &FjallJournal, record: BlobRecord)` | I19: BLAKE3 digest verification cannot be skipped for `blob` |

### 3.3 Encoding and Key Layout Invariants

| # | Test name | Signature | Verifies |
|---|---|---|---|
| P9 | `encoded_record_header_always_60_bytes` | `fn encoded_record_header_always_60_bytes(records: Vec<Record>)` | I5: `RECORD_HEADER_BYTES == 60` for all record kinds |
| P10 | `magic_bytes_match_record_kind_family` | `fn magic_bytes_match_record_kind_family(records: Vec<Record>)` | I6–I11, I70: magic is consistent with record kind family |

---

## 4. BDD Given-When-Then Scenarios

### Scenario 1: Successful Atomic Batch Commit
```
Given a valid FjallJournal with 9 keyspaces open
And a JournalWriteBatch created via journal.batch()
When I stage 8 operations across different keyspaces
And I call commit() on the batch
Then all 8 records are readable from their respective keyspaces
And len() was 8 before commit
And commit() returns Ok(())
```

### Scenario 2: Empty Batch Commit
```
Given a valid FjallJournal with keyspaces open
And a JournalWriteBatch created via journal.batch()
When I call commit() without staging any operations
Then commit() returns Ok(()) immediately
And no Fjall interaction occurs
And is_empty() == true and len() == 0 throughout
```

### Scenario 3: Digest Mismatch Rejection — workflow_source
```
Given a valid FjallJournal with workflow_source keyspace
And a JournalWriteBatch created via journal.batch()
And a WorkflowSourceRecord with a forged digest that does NOT match content
When I call put_workflow_source(record) on the batch
Then JournalError::PayloadDigestMismatch (0x4013) is returned
And len() remains 0
And no record is staged
```

### Scenario 4: Digest Mismatch Rejection — blob
```
Given a valid FjallJournal with blob keyspace
And a JournalWriteBatch created via journal.batch()
And a BlobRecord with a forged digest that does NOT match content
When I call put_blob(record) on the batch
Then JournalError::PayloadDigestMismatch (0x4013) is returned
And len() remains 0
And no record is staged
```

### Scenario 5: Duplicate Event Rejection
```
Given a valid FjallJournal with run_event keyspace
And a JournalWriteBatch created via journal.batch()
And an existing event with run_id=X and seq=1 already committed
When I stage another event with run_id=X and seq=1 via append_event
Then JournalError::DuplicateEvent (0x4004) is returned
And len() remains 0
And no duplicate is staged
```

### Scenario 6: Sequence Gap Detection on Replay
```
Given a valid FjallJournal with events for run_id=X
And events with seq=1, seq=2 committed
When I replay and encounter seq=4 (gap at 3)
Then JournalError::SequenceGap (0x4009) is returned
And replay halts
```

### Scenario 7: Process Lock Prevents Dual Writer
```
Given a FjallJournal already open at path=/tmp/test.db
When a second process attempts to open the same path
Then JournalError::ProcessLockHeld (0x401A) is returned
And no keyspace files are created by the second process
```

### Scenario 8: Strict Mode Durability Guarantee
```
Given a valid FjallJournal with SyncAll capability
And a JournalWriteBatch created via journal.batch().strict()
When I stage one record and call commit()
Then commit() blocks until fsync completes
And commit() returns Ok(()) only after data is flushed to disk
```

### Scenario 9: Encoded Record Header Size Enforcement
```
Given any Record to be encoded via codec::encode_record
When encoding completes successfully
Then the encoded header is exactly 60 bytes
And the header contains correct magic, schema_version, record_kind, payload_len
And the header_crc32c validates against the header bytes
```

### Scenario 10: CRC Detection on Corrupted Header
```
Given a valid encoded record in storage
When a single bit is flipped in the 60-byte header region
And I attempt to decode the record
Then JournalError::HeaderChecksumMismatch (0x4012) is returned
And the corrupted record is not returned
```

### Scenario 11: Future Schema Version Rejection
```
Given a record encoded with schema_version = current + 1
When I attempt to decode the record
Then JournalError::UnsupportedSchemaVersion (0x400C) is returned
```

### Scenario 12: Monotonic Sequence Enforcement
```
Given a run with events seq=1 through seq=N committed
When append_event() is called with seq = N + 2 (gap at N+1)
Then JournalError::SequenceGap (0x4009) is returned
And the event is not staged
```

### Scenario 13: Payload Too Large Boundary
```
Given a record whose encoded form equals the max allowed size + 1 byte
When I call encode_record with max_record_size = exact_boundary
Then JournalError::PayloadTooLarge (0x4011) is returned
```

### Scenario 14: All-Keyspace Atomicity Under Failure
```
Given a valid FjallJournal with all 9 keyspaces
And a JournalWriteBatch created via journal.batch()
And 8 operations staged across 8 different keyspaces
When Fjall OwnedWriteBatch::commit() fails midway
Then no records from any keyspace are visible
And the batch is in a failed state
And len() reflects 0 committed records
```

---

## 5. Test Configuration

| Parameter | Value |
|---|---|
| Test harness | `#[test]` + `#[tokio::test]` for async |
| Property framework | `proptest 1.5` |
| Temp dir strategy | ` tempfile::TempDir` per test |
| Fjall mode | `StorageMode::LowMemory` for tests |
| Strategy | `JournallJournal::open()` with unique temp paths per test |

---

*Generated by test-planner for vb-fb52*
