# Error Taxonomy: VB Storage Budget-Before-Decode

## JournalError Variants Relevant to Budget-Before-Decode

### Codec Layer Errors

#### BadMagic
```rust
#[error("bad record magic: {found:#010x}")]
BadMagic { found: u32 }
```
- **Cause**: Wire magic does not match expected family magic
- **Detection**: `decode_record_header` line 35–38, before any payload access
- **Severity**: Reject; indicates wrong record type at key or corrupted key
- **No allocation before**: Verified at step 3 of decode (before budget gate)

#### PayloadTooLarge
```rust
#[error("record payload too large: {len} > {max}")]
PayloadTooLarge { len: u32, max: u32 }
```
- **Cause**: `payload_len` field in header exceeds caller-supplied `max_payload_len`
- **Detection**: `decode_record_header` line 48 ⭐ **BUDGET GATE**
- **Severity**: Reject; prevents allocation bomb
- **Invariant**: This is the **sole budget enforcement point** before payload access
- **No allocation before**: Budget check at line 48 is step 8; no Vec created before this

#### UnexpectedEof
```rust
#[error("unexpected end of record")]
UnexpectedEof
```
- **Cause**: Insufficient bytes for header (needs 60) or payload slice
- **Detection**: `bytes.get(..60)` at line 32; `payload_end.checked_add()` at line 66; `bytes.get(start..end)` at line 69
- **Severity**: Reject; indicates truncated or corrupt record

#### HeaderChecksumMismatch
```rust
#[error("record header checksum mismatch")]
HeaderChecksumMismatch
```
- **Cause**: CRC32C of first 56 bytes doesn't match declared `header_checksum`
- **Detection**: `decode_record_header` line 54
- **Severity**: Reject; header corrupted

#### HeaderLengthMismatch
```rust
#[error("record header length mismatch: {found}")]
HeaderLengthMismatch { found: u32 }
```
- **Cause**: `header_len` field is not 60
- **Detection**: `decode_record_header` line 43–47
- **Severity**: Reject; wrong wire format version

#### UnknownRecordKind
```rust
#[error("unknown record kind: {kind}")]
UnknownRecordKind { kind: u16 }
```
- **Cause**: `record_kind` field not in `RecordKind` enum
- **Detection**: `decode_record_header` line 41

#### RecordKindFamilyMismatch
```rust
#[error("record kind {kind} does not belong to magic {magic:#010x}")]
RecordKindFamilyMismatch { magic: u32, kind: u16 }
```
- **Cause**: Record kind not valid for the magic family
- **Detection**: `decode_record_header` line 42

#### UnsupportedSchemaVersion
```rust
#[error("unsupported record schema version: {version}")]
UnsupportedSchemaVersion { version: u16 }
```
- **Cause**: Schema version > CURRENT_SCHEMA_VERSION (1)
- **Detection**: `validate_schema_version` in `decode_record_header`

#### MigrationRequired
```rust
#[error("record schema migration required from {from} to {to}")]
MigrationRequired { from: u16, to: u16 }
```
- **Cause**: Schema version < CURRENT_SCHEMA_VERSION
- **Detection**: `validate_schema_version` in `decode_record_header`

#### PayloadDigestMismatch
```rust
#[error("record payload digest mismatch")]
PayloadDigestMismatch
```
- **Cause**: BLAKE3 of payload bytes doesn't match `header.payload_digest`
- **Detection**: `verify_digest_match` in `decode_record_payload` (called after budget gate)
- **Severity**: Reject; payload corrupted

#### PostcardDecodeFailed
```rust
#[error("postcard payload decode failed")]
PostcardDecodeFailed
```
- **Cause**: Postcard deserialize error (malformed or wrong type)
- **Detection**: `postcard::from_bytes` in `decode_record`
- **Severity**: Reject; wire type doesn't match target type

### Journal-Specific Errors

#### DuplicateEvent
```rust
#[error("duplicate journal event for run {run:?} seq {seq:?}")]
DuplicateEvent { run: RunId, seq: EventSeq }
```
- **Cause**: Event key already exists during append
- **Detection**: `keyspace.contains_key(key)` before insert

#### SequenceGap
```rust
#[error("journal replay sequence gap: expected {expected:?}, actual {actual:?}")]
SequenceGap { expected: EventSeq, actual: EventSeq }
```
- **Cause**: Non-contiguous event sequence during replay
- **Detection**: `validate_replayed_event` in codec module

#### WrongRun
```rust
#[error("journal replay returned run {actual:?}, expected {expected:?}")]
WrongRun { expected: RunId, actual: RunId }
```
- **Cause**: Replayed event belongs to different run

#### SequenceOverflow
```rust
#[error("journal event sequence overflow")]
SequenceOverflow
```
- **Cause**: `next_seq` addition overflows u64

### Recovery Errors

#### CorruptSnapshot
```rust
#[error("snapshot corrupt for run {run:?} at seq {seq:?}")]
CorruptSnapshot { run: RunId, seq: EventSeq }
```
- **Cause**: Snapshot decode fails (any JournalError from snapshot read)
- **Severity**: Recovery falls back to full journal replay

#### NoRecoveryData
```rust
#[error("no recovery data found for run {run:?}")]
NoRecoveryData { run: RunId }
```
- **Cause**: Neither snapshot nor events found for run

#### NonIdempotentActionBlocked
```rust
#[error("non-idempotent action {action:?} at step {step:?} cannot be re-executed during recovery")]
NonIdempotentActionBlocked { action: ActionId, step: StepIdx }
```
- **Cause**: Action replay tracker blocks re-execution

### Fjall Layer Errors

#### Fjall
```rust
#[error("fjall journal operation failed: {0}")]
Fjall(#[from] fjall::Error)
```
- **Cause**: Underlying Fjall database error (I/O, corruption, etc.)
- **Severity**: Propagated from keyspace operations

## Error Hierarchy by Decode Phase

```
[Raw bytes from Fjall]
  │
  ├─ UnexpectedEof ────────────── Header EOF (line 32-33)
  │
  ├─ BadMagic ─────────────────── Magic mismatch (line 35-38)
  │
  ├─ UnsupportedSchemaVersion ─── Future schema (validate_schema_version)
  ├─ MigrationRequired ────────── Old schema (validate_schema_version)
  │
  ├─ UnknownRecordKind ────────── Unknown kind (line 41)
  ├─ RecordKindFamilyMismatch ─── Kind/magic mismatch (line 42)
  │
  ├─ HeaderLengthMismatch ─────── Wrong header_len (line 43-47)
  │
  ├─ PayloadTooLarge ⭐ ────────── Budget exceeded (line 48-53)
  │
  ├─ HeaderChecksumMismatch ────── CRC32C fail (line 54-56)
  │
  │  [After budget gate: payload_len is trusted]
  │
  ├─ UnexpectedEof ─────────────── Payload EOF (line 69-71)
  ├─ PayloadDigestMismatch ─────── BLAKE3 fail (line 72)
  │
  └─ PostcardDecodeFailed ──────── Deserialize fail (decode_record line 42)
```

## Error Recovery Semantics

| Error | Recovery Action |
|-------|----------------|
| `BadMagic` | Hard fail; wrong key or serious corruption |
| `PayloadTooLarge` | Hard fail; indicates write-time violation or attack |
| `UnexpectedEof` | Hard fail; truncated record |
| `HeaderChecksumMismatch` | Hard fail; header corrupted |
| `PayloadDigestMismatch` | Hard fail; payload corrupted |
| `PostcardDecodeFailed` | Hard fail; wire type error |
| `DuplicateEvent` | Deduplicated; not an error for reads |
| `SequenceGap` | Hard fail; journal corruption |
| `CorruptSnapshot` | Fall back to full journal replay |
| `NoRecoveryData` | Return error to caller |