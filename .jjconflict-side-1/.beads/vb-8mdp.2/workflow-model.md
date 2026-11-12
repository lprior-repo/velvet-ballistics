# Workflow Model: VB Storage Read — Budget-Before-Decode

## High-Level Storage Read Workflow

```
Caller (journal/snapshots/blobs/events/headers)
    │
    ▼
decode_optional(keyspace, key, magic, max_bytes)
    │
    ├── Fjall Keyspace.get(key) ─────────────────────────────────┐
    │   Returns Option<&[u8]>  (borrowed, zero-copy)              │
    │   If None → return Ok(None)                                 │
    │   If Some → continue with borrowed bytes                   │
    │                                                            │
    ▼                                                            │
decode_record(bytes, magic, max_bytes)                            │
    │                                                            │
    ▼                                                            │
decode_record_payload(bytes, magic, max_bytes)                   │
    │                                                            │
    ├── decode_record_header(bytes, magic, max_bytes) ──────────  │
    │   │                                                        │
    │   ├── header = bytes.get(..60)? → UnexpectedEof            │
    │   ├── decoded = decode_record_header_unchecked_len(header)│
    │   ├── decoded.magic != magic → BadMagic                    │
    │   ├── validate_schema_version(decoded.schema_version)?     │
    │   ├── validate_known_kind(decoded.record_kind)?            │
    │   ├── validate_kind_family(magic, kind)?                  │
    │   ├── decoded.header_len != 60 → HeaderLengthMismatch     │
    │   ├── decoded.payload_len > max → PayloadTooLarge ⭐      │  ← BUDGET GATE (line 48)
    │   └── crc32c(header[..56]) != decoded.checksum → ...      │
    │       HeaderChecksumMismatch                               │
    │   Returns Ok(RecordHeader) with validated payload_len     │
    │                                                            │
    ├── payload_start = 60                                      │
    ├── payload_end = 60 + header.payload_len (overflow safe)  │
    ├── payload = bytes.get(payload_start..payload_end)?        │
    │   → UnexpectedEof if insufficient bytes                     │
    │                                                            │
    ├── verify_digest_match(payload, header.payload_digest)?     │
    │   → PayloadDigestMismatch if BLAKE3 fails                 │
    │                                                            │
    └── return (RecordEnvelope, payload_slice)                    │
    │                                                            │
    ▼                                                            │
postcard::from_bytes(payload)                                    │
    → PostcardDecodeFailed if deserialize fails                  │
    │                                                            │
    ▼                                                            │
return Ok(record)                                                │
    │
    ▼
return Ok(Some(record))
```

## State Machine: Budget Gate

```
[Raw bytes from Fjall]
         │
         ▼
   EOF check (needs 60 bytes)
         │
    fail → UnexpectedEof
         │ ok
         ▼
   Magic check
         │
    fail → BadMagic
         │ ok
         ▼
   Schema version check
         │
    fail → UnsupportedSchemaVersion | MigrationRequired
         │ ok
         ▼
   Known kind check
         │
    fail → UnknownRecordKind
         │ ok
         ▼
   Kind-family check
         │
    fail → RecordKindFamilyMismatch
         │ ok
         ▼
   Header length check (==60)
         │
    fail → HeaderLengthMismatch
         │ ok
         ▼
   ★ BUDGET CHECK (line 48) ★
   payload_len > max_payload_len?
         │
    yes → PayloadTooLarge { len, max }
         │ no
         ▼
   CRC32C check
         │
    fail → HeaderChecksumMismatch
         │ ok
         ▼
   [RecordHeader valid; payload_len ≤ max_payload_len]
         │
         ▼
   Payload slice + BLAKE3 verify
         │
    fail → PayloadDigestMismatch | UnexpectedEof
         │ ok
         ▼
   Postcard deserialize
         │
    fail → PostcardDecodeFailed
         │ ok
         ▼
   [Typed record returned]
```

## Typed Read Methods Using decode_optional

### Journal Events
```rust
pub fn events_for_run(&self, run: RunId) → Vec<JournalEvent>
  // Scans events keyspace with run_id prefix
  // decode_optional(&self.events, key, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
```

### Snapshot Read
```rust
pub fn snapshot(&self, run: RunId, seq: EventSeq) → Option<RunSnapshot>
  // keyspace: self.run_snapshot
  // magic: MAGIC_SNAPSHOT
  // max_bytes: MAX_SNAPSHOT_BYTES (67_108_864)
  // decode_optional(&self.run_snapshot, key.as_slice(), MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)
```

### Blob Read
```rust
pub fn blob(&self, digest: [u8; 32]) → Option<BlobRecord>
  // keyspace: self.blob
  // magic: MAGIC_BLOB
  // max_bytes: MAX_BLOB_BYTES (67_108_864)
  // decode_optional(&self.blob, key.as_slice(), MAGIC_BLOB, MAX_BLOB_BYTES)
```

### Workflow Source Read
```rust
pub fn workflow_source(&self, digest: WorkflowDigest) → Option<WorkflowSourceRecord>
  // keyspace: self.workflow_source
  // magic: MAGIC_WORKFLOW_SOURCE
  // max_bytes: MAX_WORKFLOW_SOURCE_BYTES (1_048_576)
  // decode_optional(&self.workflow_source, key, MAGIC_WORKFLOW_SOURCE, MAX_WORKFLOW_SOURCE_BYTES)
```

### Compiled IR Read
```rust
pub fn compiled_ir(&self, digest: WorkflowDigest) → Option<CompiledIrRecord>
  // keyspace: self.compiled_ir
  // magic: MAGIC_COMPILED_ARTIFACT
  // max_bytes: MAX_COMPILED_IR_BYTES (16_777_216)
  // decode_optional(&self.compiled_ir, key, MAGIC_COMPILED_ARTIFACT, MAX_COMPILED_IR_BYTES)
```

### Run Header Read
```rust
pub fn run_header(&self, run: RunId) → Option<RunHeaderRecord>
  // keyspace: self.run_header
  // magic: MAGIC_INDEX_RECORD
  // max_bytes: MAX_RUN_HEADER_BYTES (65_536)
  // decode_optional(&self.run_header, key, MAGIC_INDEX_RECORD, MAX_RUN_HEADER_BYTES)
```

## Recovery Read Path

```
recover_snapshot_plus_tail(journal, run)
    │
    ├── journal.snapshot(run, seq)  ──→ decode_optional(run_snapshot, key, VBSN, MAX_SNAPSHOT_BYTES)
    │                                     │
    │                                     └── decode_record → budget check at line 48
    │
    └── journal.events_for_run(run) ──→ decode_optional(run_event, key, VBJE, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
                                          │
                                          └── decode_journal_event → budget check + is_valid()
```

## Error Terminal States

| State | Meaning |
|-------|---------|
| `Ok(None)` | Key not present in keyspace |
| `Ok(Some(T))` | Successfully decoded and deserialized |
| `Err(JournalError)` | Any decode/envelope/codec error; recovery propagates |

## Key Invariants

1. **Zero-copy read**: Fjall returns `&[u8]`; no heap allocation before budget gate
2. **Budget gate ordering**: Line 48 `PayloadTooLarge` check precedes any payload access
3. **Bounded slice**: Payload slice length equals validated `header.payload_len`
4. **Overflow safe**: `payload_end = 60 + payload_len` uses `checked_add`
5. **Digest chain**: BLAKE3 of payload verified before deserialization
6. **Magic family**: Each keyspace maps to exactly one magic; cross-family reads fail with `BadMagic`