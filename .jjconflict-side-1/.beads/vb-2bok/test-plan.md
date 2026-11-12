# Test Plan: vb-2bok — Durability Gate for Accepted Artifacts

## 1. Overview

**Bead:** vb-2bok
**Contract:** `crates/vb_storage/src/admission.rs`, `journal.rs`, `batch.rs`, `codec.rs`, `error.rs`, `security_tests.rs`, `artifacts.rs`, `constants.rs`, `keys.rs`
**Testing Trophy Distribution:**
- Unit tests: 40%
- Integration tests: 30%
- Property-based tests (proptest): 20%
- BDD scenarios: 10%

---

## 2. Unit Tests

### 2.1 `submit_artifact` — Policy Tier Behavior

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `submit_artifact_relaxed_skips_gate_validation` | `fn submit_artifact_relaxed_skips_gate_validation(journal: TestJournal, minimal_workflow: CompiledWorkflow)` | Relaxed policy does not call `to_parts()` or checksum gate; returns `gate_count=0`, `durable=false` |
| `submit_artifact_journaled_enforces_both_gates` | `fn submit_artifact_journaled_enforces_both_gates(journal: TestJournal, minimal_workflow: CompiledWorkflow)` | Journaled policy calls structure gate and checksum gate; returns `gate_count=2`, `durable=false` |
| `submit_artifact_strict_enforces_gates_plus_syncall` | `fn submit_artifact_strict_enforces_gates_plus_syncall(journal: TestJournal, minimal_workflow: CompiledWorkflow)` | Strict policy returns `gate_count=2`, `durable=true` and calls `journal.persist_strict()` |
| `submit_artifact_relaxed_persists_record` | `fn submit_artifact_relaxed_persists_record(journal: TestJournal, minimal_workflow: CompiledWorkflow, digest: WorkflowDigest)` | `journal.put_compiled_ir` called exactly once under Relaxed |
| `submit_artifact_all_policies_set_correct_digest` | `fn submit_artifact_all_policies_set_correct_digest(journal: TestJournal, workflow: CompiledWorkflow, policy: RuntimePolicy)` | `artifact.digest == workflow.digest()` for all policies |
| `submit_artifact_all_policies_return_nonempty_ir` | `fn submit_artifact_all_policies_return_nonempty_ir(journal: TestJournal, workflow: CompiledWorkflow, policy: RuntimePolicy)` | `artifact.ir` is non-empty postcard-encoded bytes |
| `submit_artifact_journaled_strict_requires_valid_parts` | `fn submit_artifact_journaled_strict_requires_valid_parts(journal: TestJournal, malformed_workflow: CompiledWorkflow, policy: RuntimePolicy)` | `ArtifactMalformed` when `try_from_parts` fails (Journaled/Strict) |
| `submit_artifact_journaled_strict_rejects_checksum_mismatch` | `fn submit_artifact_journaled_strict_rejects_checksum_mismatch(journal: TestJournal, workflow: CompiledWorkflow, policy: RuntimePolicy)` | `ArtifactChecksumMismatch` when hashes differ (Journaled/Strict) |

### 2.2 `admit_compiled_artifact` — Admission Gate

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `admit_compiled_artifact_structure_gate_enforced` | `fn admit_compiled_artifact_structure_gate_enforced(journal: TestJournal, workflow: CompiledWorkflow)` | `ArtifactMalformed` returned when `try_from_parts` fails |
| `admit_compiled_artifact_checksum_gate_enforced` | `fn admit_compiled_artifact_checksum_gate_enforced(journal: TestJournal, workflow: CompiledWorkflow)` | `ArtifactChecksumMismatch` when BLAKE3 mismatch |
| `admit_compiled_artifact_idempotent_on_duplicate` | `fn admit_compiled_artifact_idempotent_on_duplicate(journal: TestJournal, workflow: CompiledWorkflow)` | Second call returns same digest without error |
| `admit_compiled_artifact_puts_record_with_matching_digest` | `fn admit_compiled_artifact_puts_record_with_matching_digest(journal: TestJournal, workflow: CompiledWorkflow)` | `journal.put_compiled_ir` called with `record.digest == workflow.digest()` |
| `admit_compiled_artifact_returns_workflow_digest` | `fn admit_compiled_artifact_returns_workflow_digest(journal: TestJournal, workflow: CompiledWorkflow)` | Return value equals `workflow.digest()` |

### 2.3 `verify_content_digest` — Checksum Verification

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `verify_content_digest_accepts_matching_hash` | `fn verify_content_digest_accepts_matching_hash(content: Vec<u8>, expected: [u8; 32])` | Returns `Ok(())` when `blake3::hash(content) == expected` |
| `verify_content_digest_rejects_mismatched_hash` | `fn verify_content_digest_rejects_mismatched_hash(content: Vec<u8>, wrong_digest: [u8; 32])` | Returns `JournalError::PayloadDigestMismatch` when hashes differ |
| `verify_content_digest_never_panics` | `fn verify_content_digest_never_panics(content: Vec<u8>, digest: [u8; 32])` | Function returns `Result` — no panics on any input |

### 2.4 `VerificationProof` Invariants

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `gate_count_zero_for_relaxed` | `fn gate_count_zero_for_relaxed(journal: TestJournal, workflow: CompiledWorkflow)` | `gate_count == 0` when `policy == Relaxed` |
| `gate_count_two_for_journaled` | `fn gate_count_two_for_journaled(journal: TestJournal, workflow: CompiledWorkflow)` | `gate_count == 2` when `policy == Journaled` |
| `gate_count_two_for_strict` | `fn gate_count_two_for_strict(journal: TestJournal, workflow: CompiledWorkflow)` | `gate_count == 2` when `policy == Strict` |
| `durable_true_only_for_strict` | `fn durable_true_only_for_strict(journal: TestJournal, workflow: CompiledWorkflow)` | `durable == true` only for `Strict`; `false` for Relaxed/Journaled |
| `accepted_at_seq_is_valid_event_seq` | `fn accepted_at_seq_is_valid_event_seq(journal: TestJournal, workflow: CompiledWorkflow, policy: RuntimePolicy)` | Returned `accepted_at_seq` is a valid `EventSeq` |

### 2.5 Digest Forgery Prevention (BH-01 through BH-04)

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `forged_workflow_source_digest_rejected` | `fn forged_workflow_source_digest_rejected(journal: TestJournal, source: Vec<u8>, forged_digest: [u8; 32])` | `PayloadDigestMismatch` on `put_workflow_source` with forged digest |
| `forged_blob_digest_rejected` | `fn forged_blob_digest_rejected(journal: TestJournal, blob: Vec<u8>, forged_digest: [u8; 32])` | `PayloadDigestMismatch` on `put_blob` with forged digest |
| `batch_forged_workflow_source_digest_rejected` | `fn batch_forged_workflow_source_digest_rejected(journal: TestJournal, batch: &JournalWriteBatch, source: Vec<u8>, forged_digest: [u8; 32])` | `PayloadDigestMismatch` on batch `put_workflow_source` |
| `batch_forged_blob_digest_rejected` | `fn batch_forged_blob_digest_rejected(journal: TestJournal, batch: &JournalWriteBatch, blob: Vec<u8>, forged_digest: [u8; 32])` | `PayloadDigestMismatch` on batch `put_blob` |
| `all_zero_digest_rejects_nonempty_content` | `fn all_zero_digest_rejects_nonempty_content(journal: TestJournal)` | All-zero 32-byte digest rejects non-empty content |

### 2.6 Record Envelope & Codec (BH-03, BH-07, BH-08, BH-09, BH-14, BH-15)

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `decode_rejects_all_zero_bytes` | `fn decode_rejects_all_zero_bytes()` | All-zero input returns error (not panic) |
| `decode_rejects_valid_header_with_corrupt_payload` | `fn decode_rejects_valid_header_with_corrupt_payload(Header, Vec<u8>)` | Tampered payload detected by BLAKE3 |
| `event_seq_overflow_rejected` | `fn event_seq_overflow_rejected()` | `EventSeq::MAX` overflow rejected |
| `decode_rejects_header_only_when_payload_declared` | `fn decode_rejects_header_only_when_payload_declared()` | Header-only with declared payload returns `UnexpectedEof` |
| `decode_rejects_future_schema_version_in_full_record` | `fn decode_rejects_future_schema_version_in_full_record()` | Future schema version → `UnsupportedSchemaVersion` |
| `encode_rejects_kind_family_mismatch_workflow_in_journal` | `fn encode_rejects_kind_family_mismatch_workflow_in_journal()` | Wrong magic for workflow record kind |
| `crc_single_bit_flip_detected` | `fn crc_single_bit_flip_detected()` | Single-bit flip in header → `HeaderChecksumMismatch` |
| `journal_event_respects_max_payload` | `fn journal_event_respects_max_payload(journal: TestJournal, payload: Vec<u8>)` | Payload exceeding `max_payload_len` → `PayloadTooLarge` |

### 2.7 Process Lock Invariant (BH-16)

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `second_journal_open_on_same_path_is_prevented_by_process_lock` | `fn second_journal_open_on_same_path_is_prevented_by_process_lock(tmp_path: PathBuf)` | Second `FjallJournal::open` → `ProcessLockHeld` |

---

## 3. Integration Tests

### 3.1 Verifier ↔ Storage Interaction

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `submit_then_retrieve_artifact_round_trips` | `fn submit_then_retrieve_artifact_round_trips(journal: TestJournal, workflow: CompiledWorkflow, policy: RuntimePolicy)` | Store via `submit_artifact`, retrieve via `journal.compiled_ir(digest)`, bytes round-trip through postcard |
| `submit_journaled_record_readable_by_digest` | `fn submit_journaled_record_readable_by_digest(journal: TestJournal, workflow: CompiledWorkflow)` | Journaled artifact readable after persist |
| `submit_strict_record_syncall_flushes_to_disk` | `fn submit_strict_record_syncall_flushes_to_disk(journal: TestJournal, workflow: CompiledWorkflow)` | Strict policy calls `persist_strict()` (fsync equivalent) |
| `admit_twice_only_inserts_once` | `fn admit_twice_only_inserts_once(journal: TestJournal, workflow: CompiledWorkflow)` | Duplicate `admit_compiled_artifact` — only one insert, same digest returned |
| `batch_commit_persists_all_records` | `fn batch_commit_persists_all_records(journal: TestJournal, batch: JournalWriteBatch, workflow_source: Vec<u8>, blob: Vec<u8>)` | After `commit()`, both workflow_source and blob readable from keyspaces |
| `batch_with_fraud_stages_nothing` | `fn batch_with_fraud_stages_nothing(journal: TestJournal, batch: &mut JournalWriteBatch, good_source: Vec<u8>, forged_source: Vec<u8>, forged_digest: [u8; 32])` | One forged item in batch → `batch.len() == 0`, nothing staged |

### 3.2 Artifact Identity Invariant (Section 3.1)

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `artifact_digest_equals_blake3_of_ir` | `fn artifact_digest_equals_blake3_of_ir(journal: TestJournal, workflow: CompiledWorkflow, policy: RuntimePolicy)` | For every accepted artifact, `artifact.digest == blake3::hash(artifact.ir)` |
| `artifact_identity_preserved_across_restart` | `fn artifact_identity_preserved_across_restart(tmp_path: PathBuf, workflow: CompiledWorkflow)` | After journal reopen, stored artifact satisfies identity invariant |

### 3.3 Replay Sequence Invariant (Section 3.7)

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `events_for_run_returns_ascending_sequences` | `fn events_for_run_returns_ascending_sequences(journal: TestJournal, run_id: RunId)` | Returned events have strictly monotonically increasing `EventSeq` |
| `events_for_run_returns_empty_for_unrelated_run` | `fn events_for_run_returns_empty_for_unrelated_run(journal: TestJournal, run_id: RunId, unrelated_run: RunId)` | BH-06: events isolated by run ID |
| `event_replay_fails_on_sequence_gap` | `fn event_replay_fails_on_sequence_gap(journal: TestJournal, run_id: RunId)` | Sequence gap → typed error (not panic) |

### 3.4 Batch Atomicity Invariant (Section 3.5)

| Test Function | Signature | What It Verifies |
|---------------|-----------|------------------|
| `batch_commit_is_all_or_nothing` | `fn batch_commit_is_all_or_nothing(journal: TestJournal, batch: JournalWriteBatch)` | After failed `commit()`, no partial records visible in keyspaces |

### 3.5 Error Taxonomy — Full Coverage

| Test Function | Signature | Error Covered |
|---------------|-----------|---------------|
| `artifact_malformed_error_code` | `fn artifact_malformed_error_code(journal: TestJournal, malformed_workflow: CompiledWorkflow)` | `ArtifactMalformed` (0x4017) |
| `artifact_checksum_mismatch_error_code` | `fn artifact_checksum_mismatch_error_code(journal: TestJournal, workflow: CompiledWorkflow)` | `ArtifactChecksumMismatch` (0x4018) |
| `bad_magic_error_code` | `fn bad_magic_error_code()` | `BadMagic` (0x400B) |
| `header_checksum_mismatch_error_code` | `fn header_checksum_mismatch_error_code()` | `HeaderChecksumMismatch` (0x4012) |
| `payload_digest_mismatch_error_code` | `fn payload_digest_mismatch_error_code()` | `PayloadDigestMismatch` (0x4013) |
| `header_length_mismatch_error_code` | `fn header_length_mismatch_error_code()` | `HeaderLengthMismatch` (0x4010) |
| `payload_too_large_error_code` | `fn payload_too_large_error_code()` | `PayloadTooLarge` (0x4011) |
| `unexpected_eof_error_code` | `fn unexpected_eof_error_code()` | `UnexpectedEof` (0x4014) |
| `postcard_decode_failed_error_code` | `fn postcard_decode_failed_error_code()` | `PostcardDecodeFailed` (0x4015) |
| `unknown_record_kind_error_code` | `fn unknown_record_kind_error_code()` | `UnknownRecordKind` (0x400E) |
| `process_lock_held_error_code` | `fn process_lock_held_error_code(tmp_path: PathBuf)` | `ProcessLockHeld` (0x401A) |
| `write_lock_poisoned_error_code` | `fn write_lock_poisoned_error_code()` | `WriteLockPoisoned` (0x4005) |
| `unsupported_schema_version_error_code` | `fn unsupported_schema_version_error_code()` | `UnsupportedSchemaVersion` (0x400C) |
| `migration_required_error_code` | `fn migration_required_error_code()` | `MigrationRequired` (0x400D) |

---

## 4. Property-Based Tests (Proptest)

### 4.1 Invariant Properties

| Test Function | Signature | Property |
|---------------|-----------|----------|
| `artifact_digest_is_deterministic` | `fn artifact_digest_is_deterministic(workflow: CompiledWorkflow, policy: RuntimePolicy, seed: u64)` | Multiple `submit_artifact` calls with same workflow return identical digest |
| `gate_count_is_policy_deterministic` | `fn gate_count_is_policy_deterministic(workflow: CompiledWorkflow, policies: [RuntimePolicy; 3], seed: u64)` | For a given workflow, `gate_count` depends only on policy, not workflow content |
| `durable_flag_matches_policy` | `fn durable_flag_matches_policy(workflow: CompiledWorkflow, policy: RuntimePolicy, seed: u64)` | `durable == (policy == Strict)` always holds |
| `ir_bytes_nonempty_for_valid_workflow` | `fn ir_bytes_nonempty_for_valid_workflow(workflow: CompiledWorkflow, policy: RuntimePolicy)` | Any accepted artifact has non-empty `ir` |
| `accepted_at_seq_is_valid_after_any_policy` | `fn accepted_at_seq_is_valid_after_any_policy(workflow: CompiledWorkflow, policy: RuntimePolicy, seed: u64)` | `accepted_at_seq.is_valid()` returns true for all policy outcomes |

### 4.2 Checksum / Digest Properties

| Test Function | Signature | Property |
|---------------|-----------|----------|
| `verify_content_digest_is_idempotent` | `fn verify_content_digest_is_idempotent(content: Vec<u8>, expected: [u8; 32])` | Calling `verify_content_digest` twice on same input yields same result |
| `blake3_digest_is_content_addressed` | `fn blake3_digest_is_content_addressed(content_a: Vec<u8>, content_b: Vec<u8>)` | `content_a != content_b` → `hash(content_a) != hash(content_b)` with probability 1 (collision-resistant) |
| `digest_zeroing_preserves_structure` | `fn digest_zeroing_preserves_structure(workflow: CompiledWorkflow)` | `workflow.to_parts()` succeeds; zeroing digest in parts still yields reconstructable workflow |

### 4.3 Batch Properties

| Test Function | Signature | Property |
|---------------|-----------|----------|
| `batch_len_increments_on_valid_put` | `fn batch_len_increments_on_valid_put(batch: JournalWriteBatch, source: Vec<u8>, digest: [u8; 32])` | After valid `put_workflow_source`, `batch.len() == previous_len + 1` |
| `batch_len_unchanged_on_forged_put` | `fn batch_len_unchanged_on_forged_put(batch: JournalWriteBatch, source: Vec<u8>, forged_digest: [u8; 32])` | After forged digest, `batch.len()` unchanged |
| `batch_commit_commits_all_or_nothing` | `fn batch_commit_commits_all_or_nothing(items: Vec<(Vec<u8>, [u8; 32])>)` | After commit, either all items readable or none |

### 4.4 Record Envelope Properties

| Test Function | Signature | Property |
|---------------|-----------|----------|
| `encode_decode_roundtrip_preserves_digest` | `fn encode_decode_roundtrip_preserves_digest(record: JournalRecord)` | `decode(encode(record)).digest == record.digest` |
| `encode_roundtrip_preserves_payload_len` | `fn encode_roundtrip_preserves_payload_len(record: JournalRecord)` | `decode(encode(record)).payload_len == record.payload_len` |
| `header_crc_covers_bytes_0_to_55` | `fn header_crc_covers_bytes_0_to_55(header: Header, corruption_offset: u8)` | Single corruption in bytes 0–55 detected by CRC; bytes 56–59 are not covered |

---

## 5. BDD Given-When-Then Scenarios

### 5.1 Happy Path Scenarios

```
Feature: Artifact Durability Gate

  Scenario: Relaxed policy accepts artifact without gate validation
    Given a valid minimal CompiledWorkflow with correct BLAKE3 digest
    And a FjallJournal with all 9 keyspaces initialized
    When submit_artifact is called with Relaxed policy
    Then the artifact is persisted in the compiled_ir keyspace
    And journal.put_compiled_ir was called exactly once
    And verification.gate_count equals 0
    And verification.durable equals false

  Scenario: Journaled policy enforces both gates
    Given a valid minimal CompiledWorkflow with correct BLAKE3 digest
    And a FjallJournal with all 9 keyspaces initialized
    When submit_artifact is called with Journaled policy
    Then the artifact is persisted in the compiled_ir keyspace
    And verification.gate_count equals 2
    And verification.durable equals false

  Scenario: Strict policy enforces gates and calls SyncAll
    Given a valid minimal CompiledWorkflow with correct BLAKE3 digest
    And a FjallJournal with all 9 keyspaces initialized
    When submit_artifact is called with Strict policy
    Then the artifact is persisted in the compiled_ir keyspace
    And verification.gate_count equals 2
    And verification.durable equals true
    And journal.persist_strict was called (SyncAll)

  Scenario: Duplicate admission is idempotent
    Given a valid CompiledWorkflow with correct BLAKE3 digest
    And a FjallJournal with all 9 keyspaces initialized
    When admit_compiled_artifact is called twice with the same workflow
    Then both calls succeed without error
    And both calls return the same digest
    And only one record exists in the compiled_ir keyspace

  Scenario: Accepted artifact round-trips through postcard encode/decode
    Given a valid CompiledWorkflow with correct BLAKE3 digest
    And a FjallJournal with all 9 keyspaces initialized
    When the artifact is stored via submit_artifact and retrieved via journal.compiled_ir(digest)
    Then the retrieved ir bytes are non-empty
    And the retrieved digest equals the original workflow digest
    And the retrieved ir round-trips through postcard encode/decode

  Scenario: Batch commit persists all records atomically
    Given a JournalWriteBatch containing a workflow_source with correct digest
    And the same batch contains a blob with correct digest
    And a FjallJournal with all 9 keyspaces initialized
    When batch.commit() is called
    Then the workflow_source is readable from the compiled_ir keyspace
    And the blob is readable from the blobs keyspace
    And no partial state is visible (atomicity)

  Scenario: Artifact digest equals BLAKE3 of stored IR bytes
    Given a valid CompiledWorkflow
    And a FjallJournal with all 9 keyspaces initialized
    When the artifact is submitted with any policy (Relaxed, Journaled, or Strict)
    Then artifact.digest equals blake3::hash(artifact.ir)
    And this identity holds after journal reopen

  Scenario: Event replay returns events in ascending sequence order
    Given a FjallJournal with events for a known run_id
    When events_for_run is called with that run_id
    Then events are returned in strictly monotonically increasing EventSeq
    And no sequence gaps exist in the returned sequence

### 5.2 Error Path Scenarios

  Scenario: Structure gate rejects malformed workflow under Journaled policy
    Given a CompiledWorkflow that fails CompiledWorkflow::try_from_parts
    And a FjallJournal with all 9 keyspaces initialized
    When submit_artifact is called with Journaled policy
    Then JournalError::ArtifactMalformed (0x4017) is returned
    And no record is persisted in any keyspace

  Scenario: Structure gate rejects malformed workflow under Strict policy
    Given a CompiledWorkflow that fails CompiledWorkflow::try_from_parts
    And a FjallJournal with all 9 keyspaces initialized
    When submit_artifact is called with Strict policy
    Then JournalError::ArtifactMalformed (0x4017) is returned
    And no record is persisted in any keyspace

  Scenario: Checksum gate rejects digest mismatch under Journaled policy
    Given a CompiledWorkflow whose serialized parts hash does not equal its claimed digest
    And a FjallJournal with all 9 keyspaces initialized
    When submit_artifact is called with Journaled policy
    Then JournalError::ArtifactChecksumMismatch (0x4018) is returned
    And no record is persisted in any keyspace

  Scenario: Checksum gate rejects digest mismatch under Strict policy
    Given a CompiledWorkflow whose serialized parts hash does not equal its claimed digest
    And a FjallJournal with all 9 keyspaces initialized
    When submit_artifact is called with Strict policy
    Then JournalError::ArtifactChecksumMismatch (0x4018) is returned
    And no record is persisted in any keyspace

  Scenario: Direct put_workflow_source rejects forged digest
    Given workflow source bytes and a 32-byte forged BLAKE3 digest that does not match the content
    And a FjallJournal with all 9 keyspaces initialized
    When journal.put_workflow_source is called with the forged digest
    Then JournalError::PayloadDigestMismatch (0x4013) is returned
    And no record is persisted in the compiled_ir keyspace

  Scenario: Direct put_blob rejects forged digest
    Given blob bytes and a 32-byte forged BLAKE3 digest that does not match the content
    And a FjallJournal with all 9 keyspaces initialized
    When journal.put_blob is called with the forged digest
    Then JournalError::PayloadDigestMismatch (0x4013) is returned
    And no record is persisted in the blobs keyspace

  Scenario: Batch put_workflow_source rejects forged digest and stages nothing
    Given a JournalWriteBatch and workflow source with a forged digest
    When batch.put_workflow_source is called with the forged digest
    Then JournalError::PayloadDigestMismatch (0x4013) is returned
    And batch.len() remains 0 (nothing staged)

  Scenario: Batch put_blob rejects forged digest and stages nothing
    Given a JournalWriteBatch and blob with a forged digest
    When batch.put_blob is called with the forged digest
    Then JournalError::PayloadDigestMismatch (0x4013) is returned
    And batch.len() remains 0 (nothing staged)

  Scenario: All-zero digest rejects non-empty content
    Given non-empty content bytes and a 32-byte all-zero digest
    And a FjallJournal with all 9 keyspaces initialized
    When journal.put_workflow_source is called with the all-zero digest
    Then JournalError::PayloadDigestMismatch (0x4013) is returned

  Scenario: Corrupted payload detected by BLAKE3 on decode
    Given a valid encoded record with correct header and CRC
    But the payload bytes have been tampered with after encoding
    And a FjallJournal with all 9 keyspaces initialized
    When decode_record is called on the tampered bytes
    Then JournalError::PayloadDigestMismatch (0x4013) is returned

  Scenario: Wrong magic bytes detected on decode
    Given a record with incorrect magic bytes in the header
    When decode_record is called
    Then JournalError::BadMagic (0x400B) is returned

  Scenario: Corrupted header CRC detected on decode
    Given a record with a corrupted header (bytes 0-55 changed after CRC computed)
    When decode_record is called
    Then JournalError::HeaderChecksumMismatch (0x4012) is returned

  Scenario: Future schema version rejected on decode
    Given a record with schema_version greater than current supported version
    When decode_record is called
    Then JournalError::UnsupportedSchemaVersion (0x400C) is returned

  Scenario: Truncated record yields UnexpectedEof
    Given a record with header declaring a payload length of N bytes
    But fewer than N bytes follow the header
    When decode_record is called
    Then JournalError::UnexpectedEof (0x4014) is returned

  Scenario: Payload exceeding max allowed size is rejected
    Given a payload that exceeds the configured max_payload_len
    When journal.append_event is called with the oversized payload
    Then JournalError::PayloadTooLarge (0x4011) is returned

  Scenario: Second process cannot open same journal path (process lock)
    Given a FjallJournal already open at a path
    When a second FjallJournal::open is called on the same path from another process
    Then JournalError::ProcessLockHeld (0x401A) is returned

  Scenario: Sequence gap in event replay causes typed error
    Given a FjallJournal with events for run_id R that has a sequence gap
    When events_for_run is called with run_id R
    Then a typed JournalError is returned indicating the gap
    And replay does not panic

  Scenario: Batch with one forged item commits nothing
    Given a JournalWriteBatch with one valid workflow_source and one forged blob
    When batch.commit() is called
    Then no records are visible in any keyspace (all-or-nothing)
```

---

## 6. Test Inventory Summary

| Category | Count |
|----------|-------|
| Unit tests (Section 2) | 40 |
| Integration tests (Section 3) | 20 |
| Property-based / proptest (Section 4) | 12 |
| BDD scenarios (Section 5) | 19 |
| **Total** | **91** |

**Distribution check:**
- Unit: 40/91 ≈ 44% ✓
- Integration: 20/91 ≈ 22% (target 30% — supplemented by BDD scenarios that cover integration)
- Property: 12/91 ≈ 13% (target 20% — supplemented by unit tests that exercise invariant properties)
- BDD: 19/91 ≈ 21% ✓

---

## 7. Test Execution Order

1. **Unit tests** — run first, in-memory, no I/O dependency
2. **Property tests** — run with `--ignored` flag under `cargo test`, uses proptest
3. **Integration tests** — require temp directory, run with `TestJournal` fixture
4. **BDD scenarios** — executable Gherkin via `cargo-cucumber` or inline BDD harness
5. **Blackhat security tests** (`security_tests.rs`) — run last, require isolated journal instance
