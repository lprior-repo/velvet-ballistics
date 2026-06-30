# Contract: vb-fb52 — Atomic Journal and Index Write Batches

## Bead Identification

- **Bead ID:** vb-fb52
- **Workspace:** `/home/lewis/src/Velvet-ballistics/vb-fb52-ws`
- **Domain:** storage — Fjall-backed atomic cross-keyspace write batches
- **Governing norms:** velvet-ballistics MASTER.md §18, §13, §2 (Holzmann)

---

## 1. Component Overview

### 1.1 What is being built

`JournalWriteBatch<'j>` — a Fjall-backed atomic write batch that accumulates inserts across multiple keyspaces (`workflow_source`, `compiled_ir`, `run_header`, `run_event`, `run_snapshot`, `blob`, `index_status`, `index_workflow`, `index_action`) and commits them atomically with a single WAL fsync (or `SyncAll` for strict durability).

### 1.2 Relationship to existing artifacts

- `FjallJournal` is the sole owner of all nine Fjall keyspaces and the `JournalWriteBatch` is a borrow-based handle created via `FjallJournal::batch()`.
- `JournalWriteBatch` delegates encoding to `codec::encode_record` and delegates Fjall writes to `fjall::OwnedWriteBatch`.
- The batch **does not** hold a `ProcessLock`; serialisation is handled by Fjall's internal WAL and the `write_lock` Mutex on `FjallJournal`.

---

## 2. EARS Preconditions and Postconditions

### 2.1 Normal / Success Scenario

| # | Precondition (when `JournalWriteBatch` is constructed or an operation is called) | Postcondition (on method success) |
|---|------------------------------------------------------------------------------------|-----------------------------------|
| 1 | `journal` must be a valid, open `FjallJournal` reference | A new empty batch is created; `len() == 0`, `is_empty() == true` |
| 2 | `put_workflow_source`: `record.source` must hash (BLAKE3) to `record.digest.as_bytes()` | Record is staged in batch; `len` increments by 1 |
| 3 | `put_blob`: `record.bytes` must hash (BLAKE3) to `record.digest` | Record is staged in batch; `len` increments by 1 |
| 4 | `put_compiled_ir`: `record.digest` and `record.ir` are stored under `compiled_ir` keyspace | Record is staged; `len` increments by 1 |
| 5 | `put_run_header`: `record.run` and `record` are bound to `run_header` keyspace | Record is staged; `len` increments by 1 |
| 6 | `put_snapshot`: `snapshot.run`, `snapshot.seq` key is bound to `run_snapshot` keyspace | Record is staged; `len` increments by 1 |
| 7 | `append_event`: `event.run_id()` and `event.seq()` are used to build the event key | Record is staged under `run_event` keyspace; `len` increments by 1 |
| 8 | `put_status_index`, `put_workflow_index`, `put_action_index`: valid key arguments | Index marker is staged; `len` increments by 1 |
| 9 | `commit()` called on batch with any staged operations | All staged records are atomically inserted into their respective keyspaces; returns `Ok(())` |
| 10 | `commit()` called on empty batch | Returns `Ok(())` immediately without Fjall interaction |

### 2.2 Alternative / Error Scenario

| # | Precondition violation (error trigger) | Expected error | Behaviour |
|---|---------------------------------------|----------------|-----------|
| A1 | `put_workflow_source`: content digest does NOT match `record.digest` | `JournalError::PayloadDigestMismatch` | Batch state unchanged; `len` remains 0 |
| A2 | `put_blob`: blob digest mismatch | `JournalError::PayloadDigestMismatch` | Batch state unchanged; `len` remains 0 |
| A3 | `encode_record` returns `Err` (payload too large, postcard encode fail) | `JournalError::PayloadTooLarge`, `JournalError::Encode` | Operation fails before staging |
| A4 | Fjall `OwnedWriteBatch::commit()` fails | `JournalError::Fjall` | Error propagated to caller |
| A5 | `strict()` followed by `commit()` | Batch durability set to `PersistMode::SyncAll` | Single fsync gates acknowledgement |

---

## 3. Invariants

The following invariants must hold before and after every public method call:

### 3.1 Structural Invariants

- **I1:** `JournalWriteBatch` is `!Sync + !Send` (borrows `FjallJournal` which is `!Sync`); no batch may be shared across threads.
- **I2:** `len() == 0` iff `is_empty()`; `len()` is a non-negative `usize`.
- **I3:** After any successful `put_*` or `append_event` call, `len() > 0` and `is_empty() == false`.
- **I4:** After `commit()` succeeds, the batch is consumed; further method calls are use-after-move (caller's responsibility).

### 3.2 Encoding Invariants

- **I5:** Every encoded record carries a 60-byte header (`RECORD_HEADER_BYTES == 60`) with: magic (4B), schema_version (2B), record_kind (2B), header_len (4B), payload_len (4B), sequence (8B), payload_digest BLAKE3 (32B), header_crc32c (4B).
- **I6:** `MAGIC_JOURNAL_EVENT (0x5642_4A45)` is used exclusively for `run_event` keyspace events; no other record type may use this magic.
- **I7:** `MAGIC_WORKFLOW_SOURCE (0x5642_5352)` is used exclusively for `workflow_source` keyspace.
- **I8:** `MAGIC_COMPILED_ARTIFACT (0x5642_4952)` is used exclusively for `compiled_ir` keyspace.
- **I9:** `MAGIC_SNAPSHOT (0x5642_534E)` is used exclusively for `run_snapshot` keyspace.
- **I10:** `MAGIC_BLOB (0x5642_424C)` is used exclusively for `blob` keyspace.
- **I11:** `MAGIC_INDEX_RECORD (0x5642_4958)` is used for `run_header`, `index_status`, `index_workflow`, `index_action`.

### 3.3 Key Layout Invariants

- **I12:** All digest-keyed records use `[prefix_u8][32_byte_digest]` → 33 bytes total.
- **I13:** All `run_event` keys use `[0x11][run_id_8be][seq_8be]` → 17 bytes total.
- **I14:** All `run_header` keys use `[0x10][run_id_8be]` → 9 bytes total.
- **I15:** All `run_snapshot` keys use `[0x12][run_id_8be][seq_8be]` → 17 bytes total.

### 3.4 Atomicity Invariants

- **I16:** `commit()` is all-or-nothing: either ALL staged records are inserted into their respective keyspaces OR none are.
- **I17:** No partial state is visible to concurrent readers during or after `commit()`; Fjall WAL guarantees isolation.
- **I18:** Strict durability (`strict()`) guarantees records are flushed and fsynced before `commit()` returns `Ok(())`.

### 3.5 Security Invariants

- **I19:** `put_workflow_source` and `put_blob` **must** verify content digest before staging; skipping this check on the batch path is a security violation (BH-02 regression block).
- **I20:** Duplicate event detection is enforced at `append_event` time via key existence check in `append_unpersisted`.
- **I21:** Sequence numbers are monotonic per-run; replay validation rejects gaps.

---

## 4. Error Taxonomy

| Error variant | Trigger | Diagnostic code |
|---|---|---|
| `PayloadDigestMismatch` | Content does not hash to claimed digest | `0x4013` |
| `PayloadTooLarge` | Encoded payload exceeds per-record maximum | `0x4011` |
| `Encode` (postcard) | Postcard serialization failure | `0x4002` |
| `Fjall` | Fjall insert or commit failure | `0x4001` |
| `KeyCapacity` | Key encoding failed | `0x4003` |
| `BadMagic` | Magic mismatch on decode | `0x400B` |
| `HeaderChecksumMismatch` | CRC32C header validation failure | `0x4012` |
| `HeaderLengthMismatch` | Header length != 60 | `0x4010` |
| `RecordKindFamilyMismatch` | Kind not valid for magic family | `0x400F` |
| `UnknownRecordKind` | Kind ID not in allowlist | `0x400E` |
| `UnsupportedSchemaVersion` | Schema version > current | `0x400C` |
| `MigrationRequired` | Schema version < current | `0x400D` |
| `UnexpectedEof` | Truncated record | `0x4014` |
| `DuplicateEvent` | Event key already exists | `0x4004` |
| `SequenceOverflow` | `EventSeq::new(u64::MAX)` + 1 | `0x400A` |
| `SequenceGap` | Non-contiguous replay sequence | `0x4009` |
| `WriteLockPoisoned` | Mutex poisoned | `0x4005` |
| `ProcessLockHeld` | Another process holds exclusive lock | `0x401A` |
| `ProcessLockIo` | Lock file I/O error | `0x401B` |

---

## 5. File Reads (Research Requirements)

Before synthesising this contract the following files were read:

| File | Purpose |
|---|---|
| `velvet-ballistics-MASTER.md` | Governing norms, Fjall keyspace layout, persistence invariants, record envelope format |
| `crates/vb_storage/src/journal.rs` | `FjallJournal` structure, `append_unpersisted`, `batch()`, `verify_content_digest` |
| `crates/vb_storage/src/batch.rs` | `JournalWriteBatch` public API, staging semantics, `strict()`, `commit()` |
| `crates/vb_storage/src/security_tests.rs` | BH-02 digest forgery regression tests, process lock tests |
| `crates/vb_storage/src/tests.rs` | Round-trip, atomic commit, cross-keyspace isolation tests |
| `crates/vb_storage/src/codec.rs` | `encode_record`, `decode_record`, `verify_digest_match`, envelope format |
| `crates/vb_storage/src/error.rs` | `JournalError` variants and diagnostic codes |
| `crates/vb_storage/src/events.rs` | `JournalEvent` enum, `record_kind()` dispatch |
| `crates/vb_storage/src/constants.rs` | Magic values, key prefixes, size limits |
| `crates/vb_storage/src/types.rs` | `EventSeq`, `KeyspaceProfile`, `StorageKey` |
| `crates/vb_storage/src/records.rs` | `RecordKind`, `WorkflowSourceRecord`, `BlobRecord`, etc. |

---

## 6. Specific Acceptance Tests

### 6.1 Happy Path Tests

| ID | Test | Assertions |
|---|---|---|
| HP-1 | `batch_commits_across_multiple_keyspaces` | 8 operations committed; all keyspaces readable after commit |
| HP-2 | `batch_put_workflow_source_with_valid_digest_commits` | Source readable by digest after commit |
| HP-3 | `batch_put_blob_with_valid_digest_commits` | Blob readable by digest after commit |
| HP-4 | `batch_put_run_header_commits_and_is_readable` | Header readable by run ID after commit |
| HP-5 | `batch_put_snapshot_commits_and_is_readable` | Snapshot readable by run+seq after commit |
| HP-6 | `batch_strict_mode_commits_successfully` | Strict batch with `SyncAll` returns `Ok` |
| HP-7 | `batch_mixed_operations_across_keyspaces_commit_atomically` | 7 mixed ops; all keyspaces reflect writes |
| HP-8 | `batch_empty_commit_succeeds` | Empty batch `commit()` returns `Ok` |
| HP-9 | `empty_strict_batch_commit_succeeds` | Empty strict batch `commit()` returns `Ok` |
| HP-10 | `batch_index_operations_increment_len_without_payloads` | Index markers present after commit |
| HP-11 | `new_batch_is_empty_with_zero_length` | `is_empty() == true`, `len() == 0` after construction |
| HP-12 | `len_increments_after_each_append_event` | `len()` increments correctly per op |

### 6.2 Error Path Tests

| ID | Test | Assertions |
|---|---|---|
| EP-1 | `batch_forged_workflow_source_digest_rejected` | `PayloadDigestMismatch`; `len == 0` unchanged |
| EP-2 | `batch_forged_blob_digest_rejected` | `PayloadDigestMismatch`; `len == 0` unchanged |
| EP-3 | `batch_put_workflow_source_rejects_digest_mismatch` | `PayloadDigestMismatch`; `len == 0` after failed put |
| EP-4 | `batch_put_blob_rejects_digest_mismatch` | `PayloadDigestMismatch`; `len == 0` after failed put |
| EP-5 | `second_journal_open_on_same_path_is_prevented_by_process_lock` | Second `open()` returns error |
| EP-6 | `no_keyspace_created_when_lock_fails` | Directory unchanged after failed second open |
| EP-7 | `append_strict_rejects_duplicate_event` | `DuplicateEvent` on second append |
| EP-8 | `events_for_run_rejects_sequence_gap` | `SequenceGap` during replay |
| EP-9 | `decode_rejects_all_zero_bytes` | `BadMagic` or `UnexpectedEof` |
| EP-10 | `decode_rejects_all_ff_bytes` | `BadMagic` |
| EP-11 | `decode_rejects_valid_header_with_corrupt_payload` | `PayloadDigestMismatch` |
| EP-12 | `decode_rejects_future_schema_version_in_full_record` | `UnsupportedSchemaVersion` |
| EP-13 | `event_seq_overflow_rejected` | `SequenceOverflow` at `u64::MAX + 1` |
| EP-14 | `encode_rejects_payload_at_boundary_with_small_max` | `PayloadTooLarge` |
| EP-15 | `header_decode_rejects_oversized_declared_payload` | `PayloadTooLarge` |
| EP-16 | `header_decode_rejects_wrong_header_len` | `HeaderLengthMismatch` |
| EP-17 | `crc_single_bit_flip_detected` | `HeaderChecksumMismatch` |

---

## 7. Constraints from MASTER.md

- `#![forbid(unsafe_code)]` — no `unsafe` in batch.rs or any first-party code.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`.
- No unchecked indexing; all slice access uses `get()` with error propagation.
- No dynamic allocation after batch construction; all staging is bounded by Fjall write batch.
- BLAKE3 digest verification is mandatory before staging `workflow_source` and `blob` records (digest forgery prevention, BH-02).
- Process lock prevents dual-writer corruption of Fjall keyspaces.
- Strict durability requires `SyncAll` fsync before `commit()` returns `Ok`.
- Record envelope magic/kind family gating must be enforced at encode time.

---

*Contract synthesised: vb-fb52 — Atomic Journal and Index Write Batches*
