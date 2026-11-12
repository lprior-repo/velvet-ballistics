# Contract: VB Storage Budget-Before-Decode Invariant

## Core Theorem (Fowler/Wlaschin Style)

```
┌──────────────────────────────────────────────────────────────────────┐
│  BUDGET-BEFORE-DECODE INVARIANT                                       │
│                                                                       │
│  For all calls to decode_record_header(h, magic, max):                │
│                                                                       │
│    PRE:  h is any &[u8] (possibly hostile, corrupt, or short)        │
│          max is u32 > 0 (type-specific constant)                        │
│                                                                       │
│    POST: If returned Ok(header):                                       │
│            header.payload_len ≤ max                                    │
│                                                                       │
│          If returned Err(PayloadTooLarge { len, max }):                │
│            len > max                                                   │
│                                                                       │
│    PROOF OBLIGATION:                                                  │
│      No allocation of > max bytes occurs before this check.          │
└──────────────────────────────────────────────────────────────────────┘
```

## Type-Level Contract: decode_record_header

```text
decode_record_header: (bytes: &[u8], magic: u32, max: u32) → Result<RecordHeader, JournalError>

REQUIRES:
  1. bytes.len() ≥ 0 (any length, including 0)
  2. magic ∈ {VBJE, VBSN, VBBL, VBSR, VBIR, VBIX}
  3. max ∈ {MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            MAX_SNAPSHOT_BYTES,
            MAX_BLOB_BYTES,
            MAX_WORKFLOW_SOURCE_BYTES,
            MAX_COMPILED_IR_BYTES,
            MAX_RUN_HEADER_BYTES}

ENSURES (Success):
  1. result.magic == magic
  2. result.schema_version == 1
  3. result.record_kind ∈ RecordKind
  4. result.header_len == 60
  5. result.payload_len ≤ max  ← BUDGET INVARIANT
  6. blake3(bytes[60..60+result.payload_len]) == result.payload_digest
  7. crc32c(bytes[..56]) == result.header_checksum

ENSURES (Error):
  1. Returns BadMagic          ⟺ bytes contains wrong magic
  2. Returns PayloadTooLarge   ⟺ bytes declares payload_len > max
  3. Returns UnexpectedEof     ⟺ bytes.len() < 60 OR payload slice OOB
  4. Returns HeaderChecksumMismatch ⟺ CRC32C mismatch
  5. Returns UnknownRecordKind ⟺ record_kind not in RecordKind
  6. Returns UnsupportedSchemaVersion ⟺ schema_version > 1
  7. Returns MigrationRequired ⟺ schema_version < 1

EXCEPTIONS:
  None (all errors are explicit JournalError variants)

NO UNWRAP / PANIC / EXPECT anywhere in the call chain.
```

## Type-Level Contract: decode_record_payload

```text
decode_record_payload: (bytes: &[u8], magic: u32, max: u32) → Result<(RecordEnvelope, &[u8]), JournalError>

REQUIRES:
  1. Same as decode_record_header
  2. bytes.len() ≥ 60 + header.payload_len (after header validation)

ENSURES (Success):
  1. result.1.magic == magic
  2. result.1.schema_version == 1
  3. result.1.record_kind ∈ RecordKind
  4. result.1.sequence == header.sequence
  5. result.2.len() == header.payload_len
  6. blake3(result.2) == header.payload_digest

ENSURES (Error):
  1. All errors from decode_record_header propagate
  2. Returns UnexpectedEof ⟺ bytes.len() < 60 + payload_len
  3. Returns PayloadDigestMismatch ⟺ BLAKE3 mismatch

CALLS decode_record_header FIRST (budget gate runs before any payload access).
```

## Type-Level Contract: decode_optional

```text
decode_optional: (&Keyspace, key: &[u8], magic: u32, max: u32) → Result<Option<T>, JournalError>
  where T: DeserializeOwned

REQUIRES:
  1. keyspace is an open Fjall keyspace
  2. key is a valid key for keyspace type
  3. magic and max are matched per keyspace type

ENSURES (None case):
  keyspace.get(key) returns None
  → returns Ok(None)
  (no decode attempted)

ENSURES (Some case):
  let value = keyspace.get(key)?  // &[u8] borrowed, no allocation
  let (envelope, payload) = decode_record_payload(value, magic, max)?
  let record = postcard::from_bytes(payload)?
  → returns Ok(Some(record))

NO ALLOCATION BEFORE BUDGET GATE.
```

## Railway Error Contract: decode_journal_event

```text
decode_journal_event: (bytes: &[u8], magic: u32, max: u32) → Result<(RecordEnvelope, JournalEvent), JournalError>

RAILWAY:
  decode_record::<JournalEvent>(bytes, magic, max)
      → Ok((envelope, event)) if postcard succeeds
      → Err(PostcardDecodeFailed) if postcard fails
  THEN
  JournalEvent::is_valid(event)?
      → Ok((envelope, event)) if valid
      → Err(InvalidEvent) if run_id=0 or seq=u64::MAX or attempt=0
```

## Workflow Contract: FjallJournal.snapshot

```text
FjallJournal.snapshot(run, seq) → Result<Option<RunSnapshot>, JournalError>

STEPS:
  1. key = run_snapshot_key(run, seq)?
  2. decode_optional(&self.run_snapshot, key.as_slice(), MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)?
     → Ok(None) if not found
     → Ok(Some(RunSnapshot)) if found and valid
     → Err(JournalError) on any decode failure

ENSURES:
  1. Budget gate: payload_len ≤ MAX_SNAPSHOT_BYTES enforced
  2. Magic gate: magic == MAGIC_SNAPSHOT enforced
  3. Digest gate: BLAKE3 digest verified
  4. Deserialize gate: postcard must succeed

INVARIANT:
  Snapshot bytes stored by put_snapshot() are always readable by snapshot()
  IF put_snapshot succeeds AND snapshot is called with same (run, seq).
```

## Global Invariant: No Allocation Before Budget Gate

```text
GLOBAL INVARIANT for vb_storage::codec::

  For ALL executions of decode_record_header(h, magic, max):
    ¬(exists allocation A such that size(A) > max AND A happens-before budget_check)

  where budget_check = line 48: if decoded.payload_len > max_payload_len

PROOF STRATEGY:
  1. Kani: harness that calls decode_record_header with arbitrary bytes,
     proves Err(PayloadTooLarge) is returned for len > max before any Vec creation
  2. Rust type system: decode_record_header takes &[u8] (no &mut, no owned),
     cannot create owned allocation inside function
  3. decode_record_payload creates Vec ONLY after decode_record_header returns Ok
  4. postcard::from_bytes creates Vec from bounded slice (payload_len ≤ max)
```

## Record Type Max Constants

| Record Type | Magic | max_bytes | Constant |
|-------------|-------|-----------|----------|
| JournalEvent | VBJE | 1_048_576 | MAX_JOURNAL_EVENT_PAYLOAD_BYTES |
| RunSnapshot | VBSN | 67_108_864 | MAX_SNAPSHOT_BYTES |
| BlobRecord | VBBL | 67_108_864 | MAX_BLOB_BYTES |
| WorkflowSourceRecord | VBSR | 1_048_576 | MAX_WORKFLOW_SOURCE_BYTES |
| CompiledIrRecord | VBIR | 16_777_216 | MAX_COMPILED_IR_BYTES |
| RunHeaderRecord | VBIX | 65_536 | MAX_RUN_HEADER_BYTES |

## Illegal States (Made Unrepresentable)

1. **Pre-budget allocation**: The `decode_record_header` function signature is `&[u8]` → it cannot create a `Vec`. Allocation happens in `decode_record_payload` and `decode_record` ONLY after budget gate passes.

2. **Unbounded payload read**: `decode_record_payload` uses `bytes.get(60..payload_end)` which returns `Option<&[u8]>`. The `?` operator converts OOB to `UnexpectedEof`. There is no unchecked slice access.

3. **Magic bypass**: `decode_record_header` returns `BadMagic` before any other validation that could succeed with wrong magic.

4. **Overflow silently wrapped**: `payload_end = payload_start.checked_add(payload_len)` — overflow is impossible. Wraparound would return `UnexpectedEof`.

5. **Zero-max bypass**: `if decoded.payload_len > max_payload_len` — with max=0, only zero-length payloads pass. No allocation possible.

## Side Effects

`decode_record_header`, `decode_record_payload`, `decode_record`, `decode_optional` are **pure** functions:
- No mutation of any variable
- No I/O (read path is caller responsible for Fjall get)
- No time or randomness
- No unsafe code (`#![forbid(unsafe_code)]` in vb_storage)

The only effects are:
- Returning `Result<RecordHeader, JournalError>` or similar
- Allocating a `Vec` for the deserialized type (postcard) — AFTER budget gate