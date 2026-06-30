# VB Storage Budget-Before-Decode Codebase Map

## Bead
- **ID**: vb-8mdp.2
- **Title**: Prove VB storage budget-before-decode for journal and snapshot reads
- **Target Crate**: `vb_storage`

---

## 1. Fjall Journal Read Path

### Primary File
- `crates/vb_storage/src/journal/core.rs`

### Keyspace Layout
```rust
// FjallJournal struct (lines 51-67)
pub struct FjallJournal {
    pub(crate) database: fjall::Database,
    pub(crate) workflow_source: fjall::Keyspace,
    pub(crate) compiled_ir: fjall::Keyspace,
    pub(crate) run_header: fjall::Keyspace,
    pub(crate) events: fjall::Keyspace,         // KEYSPACE_RUN_EVENT
    pub(crate) run_snapshot: fjall::Keyspace,   // KEYSPACE_RUN_SNAPSHOT
    pub(crate) blob: fjall::Keyspace,
    // ...
}
```

### Keyspace Constants
- `crates/vb_storage/src/constants.rs`:
  - `KEYSPACE_RUN_EVENT = "run_event"` (PREFIX `0x11`)
  - `KEYSPACE_RUN_SNAPSHOT = "run_snapshot"` (PREFIX `0x12`)
  - `KEYSPACE_WORKFLOW_SOURCE = "workflow_source"` (PREFIX `0x01`)
  - `KEYSPACE_COMPILED_IR = "compiled_ir"` (PREFIX `0x02`)
  - `KEYSPACE_BLOB = "blob"` (PREFIX `0x20`)

---

## 2. Journal Read Path and Payload Length Handling

### Entry Point: `decode_optional` (internal.rs, lines 12-25)
```rust
pub(crate) fn decode_optional<T: DeserializeOwned>(
    &self,
    keyspace: &fjall::Keyspace,
    key: &[u8],
    magic: u32,
    max_bytes: u32,
) -> Result<Option<T>, JournalError> {
    let Some(value) = keyspace.get(key)? else {
        return Ok(None);
    };
    let (_, record) = decode_record(value.as_ref(), magic, max_bytes)?;
    Ok(Some(record))
}
```

### Called By
- `journal/source.rs`: `workflow_source()`, `compiled_ir()`
- `blobs.rs`: `blob()`
- `snapshots.rs`: `snapshot()`
- `headers.rs`: `run_header()`

### Replay Path: `events_for_run_from` (journal/replay.rs, lines 74-105)
```rust
pub(crate) fn events_for_run_from(
    &self,
    run: vb_core::RunId,
    start_seq: EventSeq,
    first_event: EventSeq,
    limit: EventReplayLimit,
) -> Result<Vec<JournalEvent>, JournalError> {
    let snap = self.database.snapshot();
    for item in snap.range(&self.events, start_key..) {
        let (_, value) = item.into_inner_if(...)?;
        let Some(value) = value else { break; };
        let (_, event) = decode_record(
            value.as_ref(),
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // ...
    }
}
```

### Magic Constants
- `MAGIC_JOURNAL_EVENT = 0x5642_4A45` ("VBJE")
- `MAGIC_SNAPSHOT = 0x5642_534E` ("VBSN")
- `MAGIC_WORKFLOW_SOURCE = 0x5642_5352` ("VBSR")
- `MAGIC_COMPILED_ARTIFACT = 0x5642_4952` ("VBIR")
- `MAGIC_BLOB = 0x5642_424C` ("VBBL")
- `MAGIC_INDEX_RECORD = 0x5642_4958` ("VBIX")

---

## 3. Snapshot Read Path and Budget Checking

### Entry Point: `snapshot()` (snapshots.rs, lines 33-45)
```rust
pub fn snapshot(
    &self,
    run: vb_core::RunId,
    seq: EventSeq,
) -> Result<Option<RunSnapshot>, JournalError> {
    let key = run_snapshot_key(run, seq)?;
    self.decode_optional(
        &self.run_snapshot,
        key.as_slice(),
        MAGIC_SNAPSHOT,
        MAX_SNAPSHOT_BYTES,
    )
}
```

### Write Path: `put_snapshot()` (snapshots.rs, lines 18-30)
```rust
pub fn put_snapshot(&self, snapshot: &RunSnapshot) -> Result<(), JournalError> {
    let key = run_snapshot_key(snapshot.run, snapshot.seq)?;
    let value = encode_record(
        MAGIC_SNAPSHOT,
        crate::records::RecordKind::Snapshot,
        snapshot.seq.get(),
        snapshot,
        MAX_SNAPSHOT_BYTES,
    )?;
    self.run_snapshot.insert(key.to_vec(), value)?;
    Ok(())
}
```

### Max Sizes (constants.rs, lines 78-88)
```rust
pub const MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32 = 1_048_576;   // 1 MiB
pub const MAX_SNAPSHOT_BYTES: u32 = 67_108_864;               // 64 MiB
pub const MAX_BLOB_BYTES: u32 = 67_108_864;                   // 64 MiB
pub const MAX_WORKFLOW_SOURCE_BYTES: u32 = 1_048_576;         // 1 MiB
pub const MAX_COMPILED_IR_BYTES: u32 = 16_777_216;            // 16 MiB
pub const MAX_RUN_HEADER_BYTES: u32 = 65_536;                 // 64 KiB
```

### Higher-level: `load_snapshot()` (recovery/replay/core.rs, lines 232-257)
```rust
pub fn load_snapshot(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
) -> RecoveryResult<RunSnapshot> {
    let snapshot = match journal.snapshot(run, seq) {
        Ok(Some(snapshot)) => snapshot,
        Ok(None) | Err(crate::JournalError::PostcardDecodeFailed) => { /* ... */ }
    };
    let applied_seq = get_applied_sequence(journal, run)?;
    verify_snapshot_payload_digest(&snapshot)?;
    Ok(snapshot)
}
```

---

## 4. VB Storage Envelope Codec (vb-3t44 Reference)

The vb-3t44 bead defines the shared envelope codec contract. The fixed-wire codec tests live in:
- `crates/vb_storage/src/kani_postcard_envelope_wire.rs` - Envelope wire format proofs
- `crates/vb_storage/src/kani_codec.rs` - `decode_record_header` never panics on hostile input
- `crates/vb_storage/src/kani_record_magic.rs` - BadMagic detection
- `crates/vb_storage/src/kani_record_schema.rs` - Schema version validation
- `crates/vb_storage/src/kani_record_kind.rs` - UnknownRecordKind detection
- `crates/vb_storage/src/kani_record_crc.rs` - HeaderChecksumMismatch detection
- `crates/vb_storage/src/kani_record_payload_len.rs` - PayloadTooLarge budget enforcement

**This bead (vb-8mdp.2) MUST NOT duplicate those fixed-wire tests.** This bead focuses on integration-level tests of storage read paths.

---

## 5. Budget/Reserve Logic Before Postcard Decode

### Budget Check Location: `decode_record_header()` (codec/header.rs, lines 26-58)
```rust
pub fn decode_record_header(
    header: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<RecordHeader, JournalError> {
    let header = header.get(..RECORD_HEADER_BYTES).ok_or(JournalError::UnexpectedEof)?;
    let decoded = decode_record_header_unchecked_len(header)?;
    // ... magic check ...
    validate_schema_version(decoded.schema_version)?;
    validate_known_kind(decoded.record_kind)?;
    validate_kind_family(decoded.magic, decoded.record_kind)?;
    if decoded.header_len != RECORD_HEADER_LEN {
        return Err(JournalError::HeaderLengthMismatch { ... });
    }
    if decoded.payload_len > max_payload_len {   // <-- BUDGET CHECK
        return Err(JournalError::PayloadTooLarge { len: decoded.payload_len, max: max_payload_len });
    }
    if crc32c::crc32c(header_prefix_for_crc(header)?) != decoded.header_checksum {
        return Err(JournalError::HeaderChecksumMismatch);
    }
    Ok(decoded)
}
```

### Full Decode: `decode_record_payload()` (codec/payload.rs, lines 56-82)
```rust
pub(crate) fn decode_record_payload(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, &[u8]), JournalError> {
    let header = decode_record_header(bytes, expected_magic, max_payload_len)?;
    let payload_start = usize::try_from(header.header_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_len_usize = usize::try_from(header.payload_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_end = payload_start.checked_add(payload_len_usize).ok_or(JournalError::UnexpectedEof)?;
    let payload = bytes.get(payload_start..payload_end).ok_or(JournalError::UnexpectedEof)?;
    verify_digest_match(payload, header.payload_digest)?;
    Ok((envelope, payload))
}
```

### Postcard Decode (codec/mod.rs, line 42)
```rust
pub fn decode_record<T: DeserializeOwned>(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, T), JournalError> {
    let (envelope, payload) = self::payload::decode_record_payload(...)?;
    let value = postcard::from_bytes(payload).map_err(|_| JournalError::PostcardDecodeFailed)?;
    Ok((envelope, value))
}
```

### Budget Check Ordering (no-reserve path)
The header-only budget check (`payload_len > max_payload_len`) happens BEFORE:
1. Any Vec allocation for the payload slice
2. Postcard deserialization
3. Digest verification

**Flow**: `decode_record_header` → `PayloadTooLarge` error (if exceeded) → `decode_record_payload` slices only the budgeted range → `postcard::from_bytes`

---

## 6. Typed Storage Error Types

### Location: `crates/vb_storage/src/error/mod.rs`

#### Budget/Decode Errors (directly relevant)
```rust
// Payload length exceeded configured maximum
#[error("record payload too large: {len} > {max}")]
PayloadTooLarge { len: u32, max: u32 },

// Header CRC32C did not match
#[error("record header checksum mismatch")]
HeaderChecksumMismatch,

// Payload BLAKE3 digest did not match
#[error("record payload digest mismatch")]
PayloadDigestMismatch,

// Record ended before declared header/payload length
#[error("unexpected end of record")]
UnexpectedEof,

// Postcard payload decode failed
#[error("postcard payload decode failed")]
PostcardDecodeFailed,

// Record magic did not match expected family
#[error("bad record magic: {found:#010x}")]
BadMagic { found: u32 },

// Record schema version not supported
#[error("unsupported record schema version: {version}")]
UnsupportedSchemaVersion { version: u16 },

// Record schema requires explicit migration
#[error("record schema migration required from {from} to {to}")]
MigrationRequired { from: u16, to: u16 },

// Unknown record kind
#[error("unknown record kind: {kind}")]
UnknownRecordKind { kind: u16 },

// Record kind not valid for magic family
#[error("record kind {kind} does not belong to magic {magic:#010x}")]
RecordKindFamilyMismatch { magic: u32, kind: u16 },

// Header length not contract value
#[error("record header length mismatch: {found}")]
HeaderLengthMismatch { found: u32 },

// JournalEvent semantically invalid
#[error("journal event is structurally encoded but semantically invalid")]
InvalidEvent,
```

#### Snapshot-specific Errors
```rust
// Snapshot seq mismatch
#[error("snapshot seq {snapshot_seq:?} does not match applied seq {applied_seq:?}")]
SnapshotSequenceMismatch { snapshot_seq: EventSeq, applied_seq: EventSeq },

// Snapshot min_seq exceeds snapshot seq
#[error("snapshot min_seq {min_seq:?} exceeds snapshot seq {seq:?}")]
SnapshotMinSequenceInvalid { min_seq: EventSeq, seq: EventSeq },

// Snapshot payload digest mismatch
#[error("snapshot payload digest mismatch")]
SnapshotPayloadDigestMismatch,
```

---

## 7. Journal and Snapshot Read Functions

### Journal Read Functions

| Function | File | Signature |
|----------|------|-----------|
| `events_for_run` | `journal/replay.rs:53` | `pub fn events_for_run(&self, run: RunId) -> Result<Vec<JournalEvent>, JournalError>` |
| `events_for_run_bounded` | `journal/replay.rs:58` | `pub fn events_for_run_bounded(&self, run, limit) -> Result<Vec<JournalEvent>, JournalError>` |
| `events_for_run_from` | `journal/replay.rs:74` | `pub(crate) fn events_for_run_from(&self, run, start_seq, first_event, limit) -> Result<Vec<JournalEvent>, JournalError>` |
| `decode_optional` | `journal/internal.rs:13` | `pub(crate) fn decode_optional<T>(&self, keyspace, key, magic, max_bytes) -> Result<Option<T>, JournalError>` |

### Snapshot Read Functions

| Function | File | Signature |
|----------|------|-----------|
| `snapshot` | `snapshots.rs:33` | `pub fn snapshot(&self, run, seq) -> Result<Option<RunSnapshot>, JournalError>` |
| `load_snapshot` | `recovery/replay/core.rs:232` | `pub fn load_snapshot(journal, run, seq) -> RecoveryResult<RunSnapshot>` |
| `write_snapshot` | `lib.rs:228` | `pub fn write_snapshot(journal, snapshot) -> Result<(), JournalError>` (validates PRE-03/04/05) |

### Other Read Functions (same pattern via `decode_optional`)

| Function | File | Signature |
|----------|------|-----------|
| `workflow_source` | `journal/source.rs:33` | `pub fn workflow_source(&self, digest) -> Result<Option<WorkflowSourceRecord>, JournalError>` |
| `compiled_ir` | `journal/source.rs:61` | `pub fn compiled_ir(&self, digest) -> Result<Option<CompiledIrRecord>, JournalError>` |
| `blob` | `blobs.rs:35` | `pub fn blob(&self, digest) -> Result<Option<BlobRecord>, JournalError>` |
| `run_header` | `headers.rs:33` | `pub fn run_header(&self, run) -> Result<Option<RunHeaderRecord>, JournalError>` |

---

## 8. Target Crate(s) and Files for Test Implementation

### Primary Test Target
- **`crates/vb_storage/`** - The main storage crate

### Files for Integration Tests
| File | Purpose | Existing Tests |
|------|---------|----------------|
| `src/journal/tests.rs` | Journal CRUD roundtrip tests | Yes - 2400+ lines |
| `src/snapshot_tests.rs` | Snapshot store/retrieve tests | Yes - 200+ lines |
| `src/blob_tests.rs` | Blob store/retrieve tests | Yes |
| `src/error_tests.rs` | Error type exhaustiveness | Yes |
| `src/security_tests.rs` | Adversarial decode tests | Yes |
| `src/tests.rs` | Comprehensive integration tests | Yes - 26000+ lines |

### Key Existing Test Helpers
- `src/test_helpers.rs`: `temp_journal()`, `write_snapshot()`
- `src/proptest_storage.rs`: `fn temp_journal()` with proptestarbitrary variants

### Constants for Tests
- `MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_576`
- `MAX_SNAPSHOT_BYTES = 67_108_864`
- `MAX_BLOB_BYTES = 67_108_864`
- `RECORD_HEADER_BYTES = 60`

---

## 9. Relevant APIs, Types, and Test Patterns

### Key Types

```rust
// types.rs
pub struct RecordEnvelope { magic, schema_version, record_kind, sequence }
pub struct RecordHeader { magic, schema_version, record_kind, header_len, payload_len, sequence, payload_digest, header_checksum }
pub struct EventSeq(u64);  // Monotonic per-run sequence

// records.rs
pub enum RecordKind { WorkflowSource=1, CompiledIr=2, RunHeader=3, Snapshot=10, Blob=20, ... }

// recovery/types.rs
pub struct RunSnapshot { run, seq, workflow, slots, taint, payload_digest, min_seq }
```

### RecordKind IDs (records.rs)
```rust
WorkflowSource = 1
CompiledIr = 2
RunHeader = 3
RunEvent = 10..=27  (journal events)
Snapshot = 10
Blob = 20
IndexRecord = 50
```

### Test Pattern: Budget-Exceeding Input
```rust
// From security_tests.rs / existing decode tests
fn adversarial_decode_kind_family_mismatch_blob_in_snapshot() {
    // Corrupt magic while preserving CRC
    // Call snapshot() / events_for_run() / blob() / etc
    // Verify: Err(JournalError::BadMagic) returned
}
```

### Test Pattern: PayloadTooLarge at Boundary
```rust
// From kani_record_payload_len.rs (Kani-level)
// Integration test equivalent:
fn snapshot_rejects_payload_exceeding_max() {
    // Encode snapshot with payload_len > MAX_SNAPSHOT_BYTES
    // Write directly to Fjall keyspace (bypass normal put_snapshot)
    // journal.snapshot(run, seq)
    // Expected: Err(JournalError::PayloadTooLarge { len: _, max: MAX_SNAPSHOT_BYTES })
}
```

### Key Test Infrastructure
- `tempfile::TempDir` for isolated Fjall DB paths
- `crate::write_snapshot()` for validated snapshot writes
- `decode_record()` / `decode_record_header()` for direct codec testing
- `crate::constants::RECORD_HEADER_BYTES`, `MAGIC_SNAPSHOT`, etc.

---

## 10. Budget Enforcement Summary

### Where Budget is Enforced

| Stage | Function | Check |
|-------|----------|-------|
| 1. Header parse | `decode_record_header()` | `payload_len > max_payload_len` → `PayloadTooLarge` |
| 2. Slice extraction | `decode_record_payload()` | `checked_add` overflow → `UnexpectedEof`; `bytes.get(..payload_end)` → `UnexpectedEof` |
| 3. Digest verify | `verify_digest_match()` | Blake3 hash mismatch → `PayloadDigestMismatch` |
| 4. Postcard decode | `decode_record()` | `postcard::from_bytes` fail → `PostcardDecodeFailed` |
| 5. Semantic validate | `decode_journal_event()` | `!event.is_valid()` → `InvalidEvent` |

### Max Payload Limits by Record Type
| Record Type | Max Bytes | Constant |
|-------------|-----------|----------|
| Journal Event | 1,048,576 | `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` |
| Snapshot | 67,108,864 | `MAX_SNAPSHOT_BYTES` |
| Blob | 67,108,864 | `MAX_BLOB_BYTES` |
| Workflow Source | 1,048,576 | `MAX_WORKFLOW_SOURCE_BYTES` |
| Compiled IR | 16,777,216 | `MAX_COMPILED_IR_BYTES` |
| Run Header | 65,536 | `MAX_RUN_HEADER_BYTES` |

### What is NOT Allocated Before Budget Check
- No Vec allocation for payload data
- No deserialization buffer
- Only the 60-byte fixed header is read first

---

## Evidence of What Was Found

- **Fjall journal**: `crates/vb_storage/src/journal/core.rs` + `internal.rs` + `replay.rs`
- **Fjall snapshot**: `crates/vb_storage/src/snapshots.rs`
- **Budget check**: `crates/vb_storage/src/codec/header.rs:decode_record_header()` (line 48)
- **Envelope codec**: `crates/vb_storage/src/codec/` (mod.rs, header.rs, payload.rs, validation.rs)
- **Error types**: `crates/vb_storage/src/error/mod.rs`
- **Constants**: `crates/vb_storage/src/constants.rs`
- **Recovery types**: `crates/vb_storage/src/recovery/types.rs` (RunSnapshot, RecoveryError)
- **Test patterns**: `crates/vb_storage/src/snapshot_tests.rs`, `journal/tests.rs`, `security_tests.rs`
