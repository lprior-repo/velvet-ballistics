# Domain Model: VB Storage Budget-Before-Decode

## Ubiquitous Language

| Term | Definition |
|------|------------|
| **Envelope** | 60-byte fixed header + variable payload; header encodes magic, schema version, record kind, payload length, sequence, BLAKE3 digest of payload, and CRC32C of header prefix |
| **RecordHeader** | Decoded header fields; `payload_len: u32` is the declared payload byte length |
| **max_payload_len** | Caller-supplied budget ceiling; budget-before-decode invariant: `payload_len ≤ max_payload_len` must hold before any allocation |
| **decode_record_header** | Partial decode: reads 60-byte header from byte slice, checks bounds, validates magic/version/kind/family, then **enforces budget check at line 48** |
| **decode_record_payload** | Full decode: calls `decode_record_header`, slices payload from bytes, verifies BLAKE3 digest |
| **decode_record** | Full decode + postcard deserialization |
| **decode_journal_event** | decode_record + `JournalEvent::is_valid()` semantic check |
| **decode_optional** | Journal read helper; calls `keyspace.get(key)` then `decode_record`; does **not** allocate before budget check |
| **RecordEnvelope** | Metadata extracted from header: magic, schema_version, record_kind, sequence |
| **RecordKind** | Wire identifier (u16) for record family: WorkflowSource=1, CompiledIr=2, RunHeader=3, Snapshot=30, Blob=40, plus 20+ event kinds |
| **MAGIC_JOURNAL_EVENT** | `0x5642_4A45` (ASCII `VBJE`) — journal event family marker |
| **MAGIC_SNAPSHOT** | `0x5642_534E` (ASCII `VBSN`) — snapshot family marker |
| **MAGIC_BLOB** | `0x5642_424C` (ASCII `VBBL`) — blob family marker |
| **MAGIC_WORKFLOW_SOURCE** | `0x5642_5352` (ASCII `VBSR`) |
| **MAGIC_COMPILED_ARTIFACT** | `0x5642_4952` (ASCII `VBIR`) |
| **FjallJournal** | 9-keyspace Fjall-backed append journal; keyspaces: workflow_source, compiled_ir, run_header, events, run_snapshot, blob, index_status, index_workflow, index_action |
| **KeyspaceProfile** | Hot (bloom filters, no KV sep), Cold (KV sep threshold 4096), Blob (KV sep threshold 1024) |

## Value Objects

### RecordEnvelope
```rust
pub struct RecordEnvelope {
    pub magic: u32,
    pub schema_version: u16,
    pub record_kind: u16,
    pub sequence: u64,
}
```
Immutable metadata from header; carries magic + kind for family validation.

### RecordHeader
```rust
pub struct RecordHeader {
    pub magic: u32,
    pub schema_version: u16,
    pub record_kind: u16,
    pub header_len: u32,        // must equal 60
    pub payload_len: u32,       // declared payload bytes
    pub sequence: u64,
    pub payload_digest: [u8; 32], // BLAKE3 of payload bytes
    pub header_checksum: u32,   // CRC32C of first 56 bytes
}
```
Parsed from 60-byte wire header; `payload_len` is **untrusted** until budget check at line 48.

### StorageLimits
```rust
pub struct StorageLimits {
    pub max_journal_event_payload_bytes: u32, // 1_048_576
}
```
Caller-defined budget ceiling per record type.

## Fjall Keyspace Map

| Keyspace | Profile | Record Type | Magic | max_bytes |
|----------|---------|-------------|-------|-----------|
| `workflow_source` | Cold | WorkflowSourceRecord | VBSR | 1_048_576 |
| `compiled_ir` | Cold | CompiledIrRecord | VBIR | 16_777_216 |
| `run_header` | Hot | RunHeaderRecord | VBIX | 65_536 |
| `run_event` | Hot | JournalEvent | VBJE | 1_048_576 |
| `run_snapshot` | Cold | RunSnapshot | VBSN | 67_108_864 |
| `blob` | Blob | BlobRecord | VBBL | 67_108_864 |
| `index_status` | Hot | Index record | VBIX | — |
| `index_workflow` | Hot | Index record | VBIX | — |
| `index_action` | Hot | Index record | VBIX | — |

## Decode Functions (Budget-Before-Decode)

### decode_record_header(bytes, expected_magic, max_payload_len) → Result<RecordHeader, JournalError>
1. Bounds-check: `bytes.get(..60)` → `UnexpectedEof` if short
2. Read header fields (unchecked len)
3. Magic check: `decoded.magic == expected_magic` → `BadMagic` if mismatch
4. Schema version check → `UnsupportedSchemaVersion` / `MigrationRequired`
5. Kind check → `UnknownRecordKind`
6. Kind-family check → `RecordKindFamilyMismatch`
7. Header length check: `decoded.header_len == 60` → `HeaderLengthMismatch`
8. **Budget check (line 48)**: `decoded.payload_len > max_payload_len` → `PayloadTooLarge` ⭐
9. CRC32C check → `HeaderChecksumMismatch`
10. Returns `RecordHeader` with validated `payload_len`

**Critical invariant**: Steps 1–8 allocate **nothing**. Step 8 is the budget gate.

### decode_record_payload(bytes, expected_magic, max_payload_len) → Result<(RecordEnvelope, &[u8]), JournalError>
1. Calls `decode_record_header` (budget check at line 48)
2. Computes `payload_start = header.header_len` (60)
3. Computes `payload_end = payload_start + payload_len` with overflow check
4. Slices `bytes[payload_start..payload_end]` → `UnexpectedEof` if insufficient bytes
5. Verifies BLAKE3 digest of payload → `PayloadDigestMismatch`
6. Returns `(RecordEnvelope, payload_slice)`

### decode_record(bytes, expected_magic, max_payload_len) → Result<(RecordEnvelope, T), JournalError>
1. Calls `decode_record_payload`
2. Calls `postcard::from_bytes(payload)` → `PostcardDecodeFailed` if invalid

### decode_optional(journal, keyspace, key, magic, max_bytes) → Result<Option<T>, JournalError>
```rust
pub(crate) fn decode_optional<T: DeserializeOwned>(
    &self, keyspace: &fjall::Keyspace, key: &[u8],
    magic: u32, max_bytes: u32,
) -> Result<Option<T>, JournalError> {
    let Some(value) = keyspace.get(key)? else {
        return Ok(None);
    };
    let (_, record) = decode_record(value.as_ref(), magic, max_bytes)?;
    Ok(Some(record))
}
```
**No allocation before budget check**: Fjall returns `&[u8]` reference; decode_record_header is called on that slice.

## Workflow Read Patterns

### Journal Event Read
```
FjallJournal.events.get(key) 
  → decode_record(bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
    → decode_record_payload
      → decode_record_header (budget check at line 48)
      → slice payload
      → verify_digest
    → postcard::from_bytes
```

### Snapshot Read
```
FjallJournal.snapshot(run, seq)
  → decode_optional(&self.run_snapshot, key, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)
    → FjallJournal.decode_optional
      → keyspace.get(key) → Option<&[u8]>
      → decode_record(bytes, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)
```

### Blob Read
```
FjallJournal.blob(digest)
  → decode_optional(&self.blob, key, MAGIC_BLOB, MAX_BLOB_BYTES)
```

## Forbidden States

1. **Allocation-before-budget-check**: Vec allocation must not occur before `payload_len ≤ max_payload_len` is confirmed
2. **Unbounded payload read**: Payload slice must not exceed `max_payload_len`
3. **Magic bypass**: Wrong magic must produce `BadMagic` before any other operation
4. **Schema bypass**: Unknown schema version must produce `UnsupportedSchemaVersion` or `MigrationRequired`
5. **Kind bypass**: Unknown record kind must produce `UnknownRecordKind`
6. **Family bypass**: Record kind must belong to magic family or produce `RecordKindFamilyMismatch`
7. **Header length bypass**: Header length other than 60 must produce `HeaderLengthMismatch`
8. **Checksum bypass**: CRC32C mismatch must produce `HeaderChecksumMismatch`
9. **Digest bypass**: BLAKE3 digest mismatch must produce `PayloadDigestMismatch`
10. **Overflow bypass**: `payload_start + payload_len` overflow must produce `UnexpectedEof`