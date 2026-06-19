# Type Contracts — Storage Envelope & Digest Verification Family

**Beads**: `vb-mrwe.1`, `vb-mrwe.2`, `vb-mrwe.3`, `vb-mrwe.5`
**Status**: Grounded in HEAD. Every signature below cites a concrete source location.

## TC-1. `decode_record_payload` — trailing bytes rejection (vb-mrwe.1)

```rust
// crates/vb_storage/src/codec/payload.rs:76-95
pub(crate) fn decode_record_payload(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, &[u8]), JournalError> {
    let header = decode_record_header(bytes, expected_magic, max_payload_len)?;
    let payload_start =
        usize::try_from(header.header_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_len_usize =
        usize::try_from(header.payload_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_end = payload_start
        .checked_add(payload_len_usize)
        .ok_or(JournalError::UnexpectedEof)?;
    let payload = bytes
        .get(payload_start..payload_end)
        .ok_or(JournalError::UnexpectedEof)?;
    verify_digest_match(payload, header.payload_digest)?;     // TC-2
    reject_trailing_bytes(payload_end, bytes.len())?;          // TC-1 invariant
    Ok((envelope_from_header(&header), payload))
}
```

**Invariants**:
- `payload_end == bytes.len()` is required to return `Ok`.
- `payload_end > bytes.len()` returns `Err(JournalError::UnexpectedEof)` (insufficient bytes for declared payload).
- `payload_end < bytes.len()` returns `Err(JournalError::UnexpectedTrailingBytes { declared_end: payload_end, actual_len: bytes.len() })`.

**Error variant field semantics** (do not change without rebinding proof):
- `declared_end` = exclusive offset where payload bytes end (= `RECORD_HEADER_BYTES + payload_len`).
- `actual_len` = total input length including trailing bytes.

## TC-2. `verify_digest_match` — payload digest mismatch (vb-mrwe.2, envelope level)

```rust
// crates/vb_storage/src/codec/payload.rs:14-23
pub fn verify_digest_match(
    payload: &[u8],
    expected_digest: [u8; DIGEST_BYTES],
) -> Result<(), JournalError> {
    if blake3::hash(payload).as_bytes() == &expected_digest {
        Ok(())
    } else {
        Err(JournalError::PayloadDigestMismatch)
    }
}
```

## TC-3. `verify_content_digest` — admission-time digest mismatch (vb-mrwe.2, put path)

```rust
// crates/vb_storage/src/journal/admission.rs:5-12
pub(crate) fn verify_content_digest(
    content: &[u8],
    expected: &[u8],
) -> Result<(), JournalError> {
    let computed = blake3::hash(content);
    if computed.as_bytes() == expected {
        Ok(())
    } else {
        Err(JournalError::PayloadDigestMismatch)
    }
}
```

**Note**: This is the same error variant as TC-2 (`PayloadDigestMismatch`). The put path uses the same variant intentionally — the operational fact (digest disagrees with content) is the same regardless of which side initiated the comparison. Per-bead attribution is preserved via call-site context (journal tests assert on it).

## TC-4. `put_workflow_source` — workflow source put path

```rust
// crates/vb_storage/src/journal/source.rs:22-34
pub fn put_workflow_source(
    &self,
    record: &WorkflowSourceRecord,
) -> Result<(), JournalError> {
    verify_content_digest(&record.source, &record.digest.as_bytes())?; // TC-3
    let key = workflow_source_key(record.digest.as_bytes())?;
    let value = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        record,
        MAX_WORKFLOW_SOURCE_BYTES,
    )?;
    self.workflow_source.insert(key.to_vec(), value)?;
    Ok(())
}
```

**Invariant**: `blake3(record.source) == record.digest` is checked BEFORE any insertion. A forged digest cannot persist.

## TC-5. `put_compiled_ir` — compiled IR put path with metadata hash defense

```rust
// crates/vb_storage/src/journal/source.rs:64-72
pub(crate) fn put_compiled_ir(
    &self,
    record: &CompiledIrRecord,
) -> Result<(), JournalError> {
    validate_compiled_ir_record(record)?;
    let h_pending = Self::compute_pending_metadata_hash(record)?;
    let key = compiled_ir_key(record.digest.as_bytes())?;
    let existing = self.load_existing_compiled_ir(&key)?;
    self.validate_metadata_hash_is_consistent(
        record.digest, h_pending, existing.as_ref()
    )?;
    let record_to_store = self.build_stored_record(record, h_pending);
    self.insert_compiled_ir_record(&key, &record_to_store)
}
```

**Invariants**:
- `validate_compiled_ir_record(record)` MUST reject structurally invalid envelopes BEFORE the metadata-hash check.
- `compute_pending_metadata_hash` MUST cover the artifact envelope fields that should not change after admission: `source_digest`, `policy_digest`, inner `ir`, `verification` flags, `accepted_at_seq`, `required_capabilities`.
- `validate_metadata_hash_is_consistent` MUST return `Err(JournalError::MetadataMutation { digest })` when `h_pending != h_existing`. The check is symmetric — both "no metadata_hash yet" and "stored metadata_hash disagrees" cases go through `compute_artifact_metadata_hash(existing)` to derive a comparable value.

## TC-6. `DigestCheck` — strict hierarchy

```rust
// crates/vb_storage/src/recovery/types/digest.rs:9-52
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DigestCheck {
    WorkflowSourceOnly,
    WorkflowAndIr,
    Full,
}

impl DigestCheck {
    pub const fn hierarchy_rank(self) -> u8 {
        match self {
            Self::WorkflowSourceOnly => 1,
            Self::WorkflowAndIr => 2,
            Self::Full => 3,
        }
    }
    pub const fn checks_workflow_source(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowSourceOnly.hierarchy_rank()
    }
    pub const fn checks_compiled_ir(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowAndIr.hierarchy_rank()
    }
    pub const fn checks_full(self) -> bool {
        self.hierarchy_rank() >= Self::Full.hierarchy_rank()
    }
    pub const fn is_strictly_weaker_than(self, other: Self) -> bool {
        self.hierarchy_rank() < other.hierarchy_rank()
    }
}
```

**Invariants**:
- `rank(WorkflowSourceOnly) < rank(WorkflowAndIr) < rank(Full)` is a numeric fact. Adding a new variant requires a strictly larger rank.
- `checks_*` predicates are RANK-DERIVED, not independent flags. Disabling `checks_full()` requires inserting a rank between 2 and 3 or restructuring the enum.

## TC-7. `DigestCheckConfig` — Full-level inputs (vb-mrwe.3)

```rust
// crates/vb_storage/src/recovery/types/digest.rs:54-62
#[derive(Debug, Clone, Copy)]
pub struct DigestCheckConfig<'a> {
    pub action_abi_entries: Option<&'a [(ActionId, WorkflowDigest, WorkflowDigest)]>,
    pub policy_entries: Option<&'a [(StepIdx, WorkflowDigest, WorkflowDigest)]>,
}
```

**Semantics**:
- `action_abi_entries`: slice of `(ActionId, expected, found)`. `None` is invalid for `Full`; an empty slice is valid only when the caller has no action ABIs to verify.
- `policy_entries`: slice of `(StepIdx, expected, found)`. Same `None`-vs-empty distinction.
- Lifetime `'a` ties the config slices to the caller; the config is `Copy` but does not own the data.

## TC-8. `verify_digests` — Full-level orchestration (vb-mrwe.3)

```rust
// crates/vb_storage/src/recovery/recover.rs:94-155
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
    config: Option<DigestCheckConfig<'_>>,
) -> RecoveryResult<()> {
    check_workflow_and_ir(journal, run, workflow_digest, ir_digest, found_ir_digest, level)?;
    check_full_level(config, level)?;
    Ok(())
}

fn check_full_level(
    config: Option<DigestCheckConfig<'_>>,
    level: DigestCheck,
) -> RecoveryResult<()> {
    if !matches!(level, DigestCheck::Full) {
        return Ok(());
    }
    let Some(cfg) = config else {
        return Err(RecoveryError::FullDigestCheckConfigMissing);
    };
    let Some(action_entries) = cfg.action_abi_entries else {
        return Err(RecoveryError::FullDigestCheckConfigMissing);
    };
    let Some(policy_entries) = cfg.policy_entries else {
        return Err(RecoveryError::FullDigestCheckConfigMissing);
    };
    check_action_abi_digests(action_entries)?;
    check_policy_digests(policy_entries)?;
    Ok(())
}
```

**Invariants**:
- For `level < Full`, `config` is unused and may be `None`.
- For `level == Full`:
  - `config` MUST be `Some`.
  - `cfg.action_abi_entries` MUST be `Some`.
  - `cfg.policy_entries` MUST be `Some`.
  - Any of the three absent → `Err(FullDigestCheckConfigMissing)`.
- `check_action_abi_digests` and `check_policy_digests` MUST be no-op when their input slice is empty (caller has no entries to verify).

## TC-9. `RecordKind::StepSucceeded = 29` (vb-mrwe.5)

```rust
// crates/vb_storage/src/records/kinds.rs:22
/// Step succeeded event.
StepSucceeded = 29,
```

**Invariants**:
- Wire ID 29 is reserved and MUST NOT collide with any other `RecordKind` variant.
- `JournalEvent::StepSucceeded { run, seq, step, output }.record_kind()` MUST return `RecordKind::StepSucceeded`.
- `JournalEvent::SlotWrittenEvent { .. }.record_kind()` MUST return `RecordKind::SlotWritten` (ID 12). The two MUST be distinguishable by `record_kind()`.
- All ID values are listed in `kinds.rs::id()` (the second match arm in `pub const fn id`). Removing or renaming a variant does NOT free its ID.

## TC-10. `ValidatedJournalRecord` — semantic validation typestate

```rust
// crates/vb_storage/src/codec/record.rs:78-86
pub fn decode_validated_journal_record(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<ValidatedJournalRecord, JournalError> {
    let (envelope, event) =
        decode_record::<JournalEvent>(bytes, expected_magic, max_payload_len)?;
    ValidatedJournalRecord::try_new(envelope, event)
}
```

The `ValidatedJournalRecord` typestate makes "journal event whose structural decode succeeded but semantic invariants (e.g. `run_id != 0`, `seq` non-zero, `attempt != 0`) failed" unrepresentable. This is the boundary that catches "structurally valid but semantically illegal" events before they enter the replay pipeline.

## Forbidden Construct Patterns (Holzman-aligned)

The following patterns MUST NOT appear in any code that touches these contracts:

- `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `dbg!()`.
- Unchecked indexing (`bytes[i]`), unchecked slicing (`&bytes[..n]` without bounds proof), unchecked arithmetic (`a + b` without `checked_add`).
- `unsafe` blocks (forbidden at the crate level via `#![forbid(unsafe_code)]`).
- `Option<bool>`-shaped behavior flags, stringly record-kind IDs, `Option`-shaped lifecycle state.

Every existing `bytes.get(start..end).ok_or(...)` and `checked_add().ok_or(...)` in the cited files is the **required** form, not an optional one.
