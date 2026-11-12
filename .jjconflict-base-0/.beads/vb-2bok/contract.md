# Contract: vb-2bok — Durability Gate for Accepted Artifacts

## Bead Overview

- **Bead ID:** vb-2bok
- **Title:** verifier/storage: Durability gate for accepted artifacts
- **Domain:** `vb_storage` + `vb_validate` boundary — artifact admission and persistence
- **Governing norm:** MASTER.md Section 18 (Fjall Persistence) and Section 19 (Action ABI)

---

## 1. What This Contract Is About

"Accepted artifacts" are compiled workflow IR records (`CompiledIrRecord`) that have passed verification and are stored durably in the Fjall `compiled_ir` keyspace. The **durability gate** is the boundary that decides whether a compiled artifact is admitted to storage and with what durability guarantee.

Three policy tiers control this gate:

| Policy | Structure Gate | Checksum Gate | Persisted | SyncAll |
|--------|---------------|---------------|-----------|---------|
| `Relaxed` | skipped | skipped | yes | no |
| `Journaled` | enforced | enforced | yes | no |
| `Strict` | enforced | enforced | yes | **yes** |

---

## 2. EARS Preconditions and Postconditions

### 2.1 `submit_artifact(journal, workflow, policy) -> AcceptedArtifact`

**Precondition (Universal):** `journal` is a validly opened `FjallJournal` with all 9 keyspaces initialized.

**Precondition (Relaxed):**
- When `policy == Relaxed`: no gate validation is performed.
- The workflow's serialized bytes are stored directly in `compiled_ir` keyspace.

**Precondition (Journaled | Strict):**
- `workflow.to_parts()` must succeed (workflow is reconstructable).
- `CompiledWorkflow::try_from_parts(parts.clone())` must return `Ok`.
- Checksum validation: BLAKE3 hash of `parts` with digest zeroed must equal `workflow.digest()`.

**Postcondition (Relaxed):**
- Returns `AcceptedArtifact { verification: VerificationProof { gate_count: 0, durable: false }, ... }`.
- `journal.put_compiled_ir(record)` was called exactly once.

**Postcondition (Journaled):**
- Returns `AcceptedArtifact { verification: VerificationProof { gate_count: 2, durable: false }, ... }`.
- Both gates passed: structure valid, checksum matches.
- `journal.put_compiled_ir(record)` was called exactly once.

**Postcondition (Strict):**
- Returns `AcceptedArtifact { verification: VerificationProof { gate_count: 2, durable: true }, ... }`.
- Same gates as Journaled plus `journal.persist_strict()` succeeded (SyncAll).

**Postcondition (All):**
- Returned `artifact.digest == workflow.digest()`.
- `artifact.ir` is non-empty postcard-encoded bytes.
- `artifact.accepted_at_seq` is a valid `EventSeq`.

---

### 2.2 `admit_compiled_artifact(journal, workflow) -> WorkflowDigest`

**Precondition (Universal):** `journal` is open and valid. `workflow` is non-null.

**Precondition:** `CompiledWorkflow::try_from_parts(workflow.to_parts())` succeeds (structure gate).

**Precondition:** BLAKE3(digest-zeroed serialized parts) == `workflow.digest()` (checksum gate).

**Postcondition:** `journal.put_compiled_ir(record)` was called where `record.digest == workflow.digest()`.

**Postcondition:** Returns `workflow.digest()`.

**Postcondition:** On duplicate admission, returns same digest without error (idempotent).

---

### 2.3 `verify_content_digest(content, expected) -> Result<(), JournalError>`

**Precondition (Universal):** `content` is a byte slice. `expected` is a 32-byte BLAKE3 digest.

**Postcondition (Success):** `blake3::hash(content) == expected`.

**Postcondition (Failure):** Returns `JournalError::PayloadDigestMismatch` if hashes differ. Never panics.

---

## 3. Invariants

### 3.1 Artifact Identity Invariant
For any accepted artifact, `artifact.digest == BLAKE3(artifact.ir)` must hold. The digest key and stored content are tightly coupled — changing one without the other is impossible via the public API.

### 3.2 Gate Count Invariant
`VerificationProof.gate_count` is a deterministic function of `RuntimePolicy`:
- `Relaxed` → 0
- `Journaled` → 2
- `Strict` → 2

The `durable` flag is `true` only for `Strict`.

### 3.3 Digest Forgery Prevention Invariant
For both direct write path (`put_workflow_source`, `put_blob`) and batch path (`JournalWriteBatch::put_workflow_source`, `JournalWriteBatch::put_blob`), `verify_content_digest` is called before any keyspace insertion. A forged digest (content that does not hash to the claimed digest) can never be persisted.

### 3.4 Record Envelope Invariants
All stored records satisfy:
- 60-byte header with fixed layout (magic, schema_version, kind, header_len=60, payload_len, sequence, blake3 digest, crc32c checksum).
- Magic bytes distinguish families: `0x56425352` (VBSR), `0x56424952` (VBIR), `0x56424A45` (VBJE), `0x5642534E` (VBSN), `0x5642424C` (VBBL), `0x56424958` (VBIX).
- Header CRC protects bytes 0–55. Payload BLAKE3 protects the payload bytes.
- `payload_len <= max_payload_len` enforced before Postcard decode.

### 3.5 Batch Atomicity Invariant
`JournalWriteBatch::commit()` either persists all staged operations or none. There is no partial commit visible to subsequent readers.

### 3.6 Process Lock Invariant
Only one process can hold the Fjall database lock at a time. Second open on same path fails with `JournalError::ProcessLockHeld`.

### 3.7 Replay Sequence Invariant
For any run, `events_for_run(run)` returns events in strictly monotonically increasing `EventSeq`. A sequence gap or wrong-run event causes replay to fail with typed error.

---

## 4. Error Taxonomy

### 4.1 Artifact Malformation (Gate 1 Failure)
| Error | Code | Circumstance |
|-------|------|-------------|
| `ArtifactMalformed` | `0x4017` | `try_from_parts` fails on cloned parts |

### 4.2 Checksum Mismatch (Gate 2 Failure)
| Error | Code | Circumstance |
|-------|------|-------------|
| `ArtifactChecksumMismatch` | `0x4018` | Recomputed BLAKE3 != claimed digest |

### 4.3 Record Integrity Errors
| Error | Code | Trigger |
|-------|------|---------|
| `BadMagic` | `0x400B` | Magic byte mismatch on decode |
| `HeaderChecksumMismatch` | `0x4012` | CRC32C header corruption |
| `PayloadDigestMismatch` | `0x4013` | BLAKE3 payload corruption |
| `HeaderLengthMismatch` | `0x4010` | header_len != 60 |
| `PayloadTooLarge` | `0x4011` | payload_len > configured max |

### 4.4 Deserialization Errors
| Error | Code | Trigger |
|-------|------|---------|
| `UnexpectedEof` | `0x4014` | Truncated record bytes |
| `PostcardDecodeFailed` | `0x4015` | Postcard deserialize error |
| `UnknownRecordKind` | `0x400E` | kind ID not in allowlist |

### 4.5 Concurrency / Lock Errors
| Error | Code | Trigger |
|-------|------|---------|
| `ProcessLockHeld` | `0x401A` | Second journal open on same path |
| `WriteLockPoisoned` | `0x4005` | Mutex holder panicked |

### 4.6 Schema Migration Errors
| Error | Code | Trigger |
|-------|------|---------|
| `UnsupportedSchemaVersion` | `0x400C` | Version > current |
| `MigrationRequired` | `0x400D` | Version < current, migration path exists |

---

## 5. Explicit File Reads (Before Any Write)

Before persisting any accepted artifact, the following reads occur in the happy path:

### 5.1 `submit_artifact` (Journaled/Strict)
1. **`workflow.to_parts()`** — serializes the workflow to extract parts for gate validation.
2. **`CompiledWorkflow::try_from_parts(parts.clone())`** — Gate 1: validates structure by reconstructing workflow from parts.
3. **`blake3::hash(serialized_parts_with_zeroed_digest)`** — Gate 2: computes checksum against claimed digest.

### 5.2 `admit_compiled_artifact`
Same as 5.1 above, plus:
4. **`journal.compiled_ir.exists(key)`** — implicit in Fjall insert; duplicate detection is handled by key uniqueness.

### 5.3 Batch Write Path (e.g., `JournalWriteBatch::put_workflow_source`)
1. **`verify_content_digest(&record.source, &record.digest.as_bytes())`** — reads content bytes to compute hash before staging.

### 5.4 `verify_content_digest`
1. **`blake3::hash(content)`** — reads content bytes to compute actual digest.

---

## 6. Security Properties (Blackhat Derived)

| BH ID | Property | Test |
|-------|----------|------|
| BH-01 | Direct `put_workflow_source` rejects forged digest | `forged_workflow_source_digest_rejected` |
| BH-01 | Direct `put_blob` rejects forged digest | `forged_blob_digest_rejected` |
| BH-02 | Batch `put_workflow_source` rejects forged digest | `batch_forged_workflow_source_digest_rejected` |
| BH-02 | Batch `put_blob` rejects forged digest | `batch_forged_blob_digest_rejected` |
| BH-03 | Zeroed bytes rejected before Postcard decode | `decode_rejects_all_zero_bytes` |
| BH-03 | Corrupt payload detected by BLAKE3 | `decode_rejects_valid_header_with_corrupt_payload` |
| BH-04 | Sequence overflow at u64::MAX rejected | `event_seq_overflow_rejected` |
| BH-05 | Truncated record yields `UnexpectedEof` | `decode_rejects_header_only_when_payload_declared` |
| BH-06 | Different runs are isolated (keys include run ID) | `events_for_run_returns_empty_for_unrelated_run` |
| BH-07 | Future schema version rejected | `decode_rejects_future_schema_version_in_full_record` |
| BH-08 | Kind-family mismatch detected | `encode_rejects_kind_family_mismatch_workflow_in_journal` |
| BH-09 | CRC single-bit flip detected | `crc_single_bit_flip_detected` |
| BH-14 | All-zero digest rejects non-empty content | `all_zero_digest_rejects_nonempty_content` |
| BH-15 | Payload size limits enforced | `journal_event_respects_max_payload` |
| BH-16 | Process lock prevents dual writers | `second_journal_open_on_same_path_is_prevented_by_process_lock` |

---

## 7. Acceptance Tests

### 7.1 Happy Path Tests

```
Given a valid minimal CompiledWorkflow with correct BLAKE3 digest
When submit_artifact is called with Relaxed policy
Then artifact is persisted, verification.gate_count == 0, verification.durable == false

Given a valid minimal CompiledWorkflow with correct BLAKE3 digest
When submit_artifact is called with Journaled policy
Then artifact is persisted, verification.gate_count == 2, verification.durable == false

Given a valid minimal CompiledWorkflow with correct BLAKE3 digest
When submit_artifact is called with Strict policy
Then artifact is persisted, verification.gate_count == 2, verification.durable == true
And journal.persist_strict() was called (SyncAll)

Given a valid CompiledWorkflow
When admit_compiled_artifact is called twice
Then both calls succeed and return the same digest (idempotent)

Given a valid CompiledWorkflow
When the artifact is stored and read back via journal.compiled_ir(digest)
Then the returned ir bytes round-trip through postcard encode/decode

Given a batch containing workflow_source and blob with correct digests
When batch.commit() succeeds
Then all records are readable from their respective keyspaces
```

### 7.2 Error Path Tests

```
Given a workflow with structure that fails CompiledWorkflow::try_from_parts
When submit_artifact is called with Journaled policy
Then JournalError::ArtifactMalformed is returned
And no record is persisted

Given a CompiledWorkflow whose parts hash does not equal its claimed digest
When submit_artifact is called with Journaled policy
Then JournalError::ArtifactChecksumMismatch is returned
And no record is persisted

Given workflow source with wrong digest (forged)
When journal.put_workflow_source is called
Then JournalError::PayloadDigestMismatch is returned
And record is not persisted

Given blob with wrong digest (forged)
When journal.put_blob is called
Then JournalError::PayloadDigestMismatch is returned
And record is not persisted

Given a batch with a forged workflow source digest
When batch.put_workflow_source is called
Then JournalError::PayloadDigestMismatch is returned
And batch.len() remains 0 (nothing staged)

Given a batch with a forged blob digest
When batch.put_blob is called
Then JournalError::PayloadDigestMismatch is returned
And batch.len() remains 0

Given a corrupted record with tampered payload
When decode_record is called
Then JournalError::PayloadDigestMismatch is returned

Given a record with wrong magic bytes
When decode_record is called
Then JournalError::BadMagic is returned

Given a record with corrupted header CRC
When decode_record is called
Then JournalError::HeaderChecksumMismatch is returned

Given a record with future schema version
When decode_record is called
Then JournalError::UnsupportedSchemaVersion is returned

Given a truncated record (EOF before payload end)
When decode_record is called
Then JournalError::UnexpectedEof is returned

Given a second process attempting to open the same journal path
When FjallJournal::open is called
Then JournalError::ProcessLockHeld is returned

Given an event replay with a sequence gap
When events_for_run is called
Then JournalError::SequenceGap is returned
```

---

## 8. Key Source Files Referenced

| File | Purpose |
|------|---------|
| `crates/vb_storage/src/admission.rs` | `submit_artifact`, `admit_compiled_artifact`, `VerificationProof`, `VerificationWarning` |
| `crates/vb_storage/src/journal.rs` | `FjallJournal`, `verify_content_digest`, `append_*` methods |
| `crates/vb_storage/src/batch.rs` | `JournalWriteBatch`, digest verification on batch writes |
| `crates/vb_storage/src/codec.rs` | `encode_record`, `decode_record`, `verify_digest_match`, header validation |
| `crates/vb_storage/src/error.rs` | Full `JournalError` taxonomy with diagnostic codes |
| `crates/vb_storage/src/security_tests.rs` | BH-01 through BH-17 blackhat security tests |
| `crates/vb_storage/src/artifacts.rs` | `artifact_exists`, `list_artifacts`, `remove_artifact` |
| `crates/vb_storage/src/constants.rs` | Magic bytes, max payload sizes, schema version |
| `crates/vb_storage/src/keys.rs` | Key construction functions for all keyspaces |
| `velvet-ballistics-MASTER.md` | Sections 18 (Fjall Persistence), 19 (Action ABI), 16 (Validation Errors) |
