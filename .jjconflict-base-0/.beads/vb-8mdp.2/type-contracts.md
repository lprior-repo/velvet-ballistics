# Type Contracts: VB Storage Budget-Before-Decode

## Contract: decode_record_header

```text
decode_record_header(header: &[u8], expected_magic: u32, max_payload_len: u32) → Result<RecordHeader, JournalError>
```

### Preconditions
- `header.len() >= 60` (envelope header is fixed 60 bytes)
- `expected_magic` is a known magic constant (VBJE, VBSN, VBBL, VBSR, VBIR, VBIX)
- `max_payload_len` is a positive u32 bound

### Postconditions (Success)
- Returns `RecordHeader` where:
  - `header.magic == expected_magic`
  - `header.schema_version == 1` (CURRENT_SCHEMA_VERSION)
  - `header.record_kind` is a known variant of `RecordKind`
  - `header.header_len == 60`
  - `header.payload_len <= max_payload_len` ⭐ **BUDGET INVARIANT**
  - `header.header_checksum == crc32c(header[..56])`
  - `header.payload_digest == blake3(payload_bytes)`

### Postconditions (Error)
| Error Variant | Condition |
|--------------|-----------|
| `UnexpectedEof` | `header.len() < 60` or slice operations fail |
| `BadMagic { found }` | `found != expected_magic` |
| `UnsupportedSchemaVersion { version }` | `version > CURRENT_SCHEMA_VERSION` |
| `MigrationRequired { from, to }` | `from < CURRENT_SCHEMA_VERSION` |
| `UnknownRecordKind { kind }` | `kind` not in `RecordKind` |
| `RecordKindFamilyMismatch { magic, kind }` | `kind` not valid for `magic` family |
| `HeaderLengthMismatch { found }` | `found != 60` |
| `PayloadTooLarge { len, max }` | `len > max_payload_len` |
| `HeaderChecksumMismatch` | CRC32C mismatch |

### Budget Enforcement Point
Line 48 in `codec/header.rs`:
```rust
if decoded.payload_len > max_payload_len {
    return Err(JournalError::PayloadTooLarge { ... });
}
```
This is the **sole budget gate** before the caller may attempt payload slicing or postcard deserialization.

## Contract: decode_record_payload

```text
decode_record_payload(bytes: &[u8], expected_magic: u32, max_payload_len: u32)
  → Result<(RecordEnvelope, &[u8]), JournalError>
```

### Preconditions
- Same as `decode_record_header`
- `bytes.len() >= 60 + payload_len` (header + declared payload)

### Postconditions (Success)
- Returns `(RecordEnvelope, payload_slice)` where:
  - `payload_slice.len() == header.payload_len`
  - `blake3(payload_slice) == header.payload_digest`
  - Envelope carries validated magic/kind/version from header

### Postconditions (Error)
- All errors from `decode_record_header` propagate
- `UnexpectedEof` if `bytes.len() < 60 + payload_len`
- `PayloadDigestMismatch` if BLAKE3 digest fails

### Budget Chain
```
decode_record_payload → decode_record_header (line 48 budget check)
                      → payload_len is now validated
                      → slice bytes[60..60+payload_len]
                      → verify BLAKE3 digest
```
No allocation happens between budget check and payload slice.

## Contract: decode_optional

```text
decode_optional<T: DeserializeOwned>(
    &self,
    keyspace: &fjall::Keyspace,
    key: &[u8],
    magic: u32,
    max_bytes: u32,
) → Result<Option<T>, JournalError>
```

### Preconditions
- `keyspace` is a valid open Fjall keyspace
- `key` is a valid key for the keyspace type
- `magic` is a known magic constant
- `max_bytes` is the per-type maximum (e.g., `MAX_SNAPSHOT_BYTES`)

### Postconditions (None case)
- Returns `Ok(None)` if `keyspace.get(key)` returns `None`
- No decode attempted

### Postconditions (Some case)
- `value: &[u8]` returned by Fjall is borrowed (no allocation)
- `decode_record(value, magic, max_bytes)` is called
- Budget check at `decode_record_header` line 48 governs payload_len
- Returns `Ok(Some(record))` on success

### Type-Level Budget Binding

| Record Type | Magic | max_bytes constant | Decode fn |
|-------------|-------|-------------------|----------|
| JournalEvent | VBJE | MAX_JOURNAL_EVENT_PAYLOAD_BYTES (1 MiB) | decode_journal_event |
| RunSnapshot | VBSN | MAX_SNAPSHOT_BYTES (64 MiB) | decode_record |
| BlobRecord | VBBL | MAX_BLOB_BYTES (64 MiB) | decode_record |
| WorkflowSourceRecord | VBSR | MAX_WORKFLOW_SOURCE_BYTES (1 MiB) | decode_record |
| CompiledIrRecord | VBIR | MAX_COMPILED_IR_BYTES (16 MiB) | decode_record |
| RunHeaderRecord | VBIX | MAX_RUN_HEADER_BYTES (64 KiB) | decode_record |

## Type-Level Constraints

### RecordHeader.payload_len
- Type: `u32`
- Constraint: `payload_len <= max_payload_len` (budget argument)
- Constraint: `payload_len` is the **declared** length; actual bytes must be available before use
- Constraint: After `decode_record_header` returns `Ok`, caller may assume `payload_len <= max_payload_len`

### max_payload_len
- Type: `u32`
- Constraint: Must be the type-specific constant (not caller-controlled arbitrary value)
- Constraint: Each magic family has a fixed max (e.g., VBJE ≤ 1_048_576)

### RecordEnvelope
- Derived from header; carries magic + kind for downstream family validation
- Does NOT contain payload bytes (just metadata)

### decode_journal_event semantic check
```rust
let (envelope, event) = decode_record::<JournalEvent>(...)?;
if !event.is_valid() {
    return Err(JournalError::InvalidEvent);
}
```
Extra guard: rejects `run_id=0`, `seq=u64::MAX`, `attempt=0` that pass postcard but fail semantic check.

## Phantom Allocation Hazards

### Forbidden: Pre-budget Vec allocation
```rust
// BAD — allocates before budget check
let value = keyspace.get(key)?; // Fjall returns &[u8] — this is fine
let payload_len = decode_record_header(value, magic, max)?.payload_len;
let mut buf = Vec::with_capacity(payload_len as usize); // ALLOCATION before check
```

### Correct: Borrowed slice, check, then use
```rust
// GOOD — no allocation before budget gate
let value = keyspace.get(key)?; // &[u8] borrowed from Fjall
let header = decode_record_header(value, magic, max)?; // line 48 budget check
// Now payload_len is validated; safe to slice
let payload = &value[60..60 + header.payload_len as usize];
```

### decode_optional pattern is safe
```rust
let Some(value) = keyspace.get(key)? else { return Ok(None) };
// value is &[u8] — borrowed, no allocation
let (_, record) = decode_record(value, magic, max_bytes)?; // budget gate inside
```

## max_bytes Enforcement Hierarchy

1. **Encode time**: `payload_len_u32(payload.len(), max)` checks `len > max → PayloadTooLarge`
2. **Wire time**: `decode_record_header` checks `decoded.payload_len > max_payload_len` at line 48
3. **Decode time**: `decode_record_payload` slices `bytes[60..60+payload_len]` with bounds check
4. **Deserialize time**: `postcard::from_bytes` operates on bounded slice

This chain ensures a corrupt or adversarial record can never cause allocation beyond `max_bytes`.