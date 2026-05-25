# Hazard Analysis: VB Storage Budget-Before-Decode

## H1: Budget Bypass — Early Allocation

### Description
An attacker or corrupt record causes `decode_record_header` to return `Ok` with a large `payload_len`, but a Vec is allocated BEFORE the budget check (line 48), causing memory exhaustion.

### Affected Code Path
```rust
// decode_record_header: allocation BEFORE line 48 (CURRENTLY NOT THE CASE)
pub fn decode_record_header(...) {
    let header = header.get(..RECORD_HEADER_BYTES).ok_or(UnexpectedEof)?;
    let decoded = decode_record_header_unchecked_len(header)?;
    // ... validation checks ...
    if decoded.payload_len > max_payload_len {  // line 48
        return Err(PayloadTooLarge { ... });
    }
    // NO allocation before line 48 in current code
}
```

### Current State: NOT VULNERABLE
The current `decode_record_header` does NOT allocate before line 48. It only:
1. Slices 60 bytes from input (borrowed)
2. Reads fields via `read_u32`/`read_u16`/`read_u64` (pure reads)
3. Validates magic, schema, kind, family
4. Checks budget at line 48

### Residual Risk
If a future refactor adds a cache, pre-allocation, or intermediate buffer before line 48, the budget gate is defeated.

### Mitigation
Kani proofs verify that `decode_record_header` returns `Err(PayloadTooLarge)` before any branch that could allocate > `max_payload_len` bytes.

---

## H2: Corrupt Record Injection via Magic Confusion

### Description
A record written to keyspace A (e.g., `blob`) is read using the key from keyspace B (e.g., `workflow_source`). Wrong magic produces `BadMagic`, but if the key construction is flawed, an attacker could read arbitrary bytes within a keyspace.

### Affected Code Path
```rust
// Key construction must use correct prefix per keyspace
let key = workflow_source_key(digest.as_bytes())?;  // PREFIX_WORKFLOW_SOURCE (0x01)
keyspace.get(key)?  // Fjall returns bytes from THIS keyspace only
```

### Current State: NOT VULNERABLE (keyspace isolation)
Fjall keyspaces are isolated; keys are namespaced. Reading from `workflow_source` keyspace with a `workflow_source` key cannot access `blob` keyspace data.

### Residual Risk
- Key construction bugs: If `workflow_source_key` uses wrong prefix, Fjall would return `None` (key not found) rather than corrupt data from another keyspace.
- Wrong key type: If caller passes `blob` key to `workflow_source` read, `keyspace.get()` returns `None`.

### Mitigation
- Fixed-size keys with type-safe `keys` module
- Key prefix distinctness proven via `restate_fjall_keyspace_manifest_tests`
- Magic check in `decode_record_header` catches any remaining key confusion

---

## H3: Integer Overflow in Payload End Calculation

### Description
`payload_end = payload_start + payload_len` overflows u64/u32, wrapping to a small value. This could cause:
- `bytes.get(payload_start..payload_end)` to succeed with a small slice
- `payload_len` in header is correct but actual bytes are insufficient
- Digest check fails on truncated payload

### Affected Code Path
```rust
// decode_record_payload (line 66-68)
let payload_end = payload_start
    .checked_add(payload_len_usize)  // overflow check
    .ok_or(JournalError::UnexpectedEof)?;
let payload = bytes
    .get(payload_start..payload_end)  // only reached if overflow safe
    .ok_or(JournalError::UnexpectedEof)?;
```

### Current State: PROTECTED
`checked_add` is used; overflow returns `UnexpectedEof` before slice access.

### Residual Risk
None from overflow; residual risk is that `payload_end` fits in usize but `bytes` is shorter, which is caught by the bounds check `.get()`.

---

## H4: Postcard Deserialization Without Budget Gate

### Description
`decode_record` calls `postcard::from_bytes(payload)` AFTER `decode_record_payload`. The budget check at line 48 validates `payload_len ≤ max_payload_len`, and `payload` slice is exactly `payload_len` bytes. Postcard then allocates based on the content.

### Affected Code Path
```rust
pub fn decode_record<T: DeserializeOwned>(...) -> Result<(RecordEnvelope, T), JournalError> {
    let (envelope, payload) =
        self::payload::decode_record_payload(bytes, expected_magic, max_payload_len)?;
    let value = postcard::from_bytes(payload)  // allocation here, after budget gate
        .map_err(|_| JournalError::PostcardDecodeFailed)?;
    Ok((envelope, value))
}
```

### Current State: PROTECTED
- Budget gate (line 48) runs BEFORE payload slice
- Payload slice length equals validated `payload_len`
- Postcard receives bounded slice

### Residual Risk
- Postcard itself may internally allocate more than `payload_len` for complex types (e.g., Vec fields inside the deserialized struct). This is a postcard behavior, not a vb_storage bug.
- Mitigation: `max_payload_len` is the per-type constant; Postcard can only allocate up to the slice size it receives.

---

## H5: BLAKE3 Digest Bypass

### Description
A corrupt record has valid magic/version/kind and passes budget check, but payload bytes don't match `payload_digest`. If digest verification is skipped or buggy, corrupt payload is returned.

### Affected Code Path
```rust
// decode_record_payload (line 72)
verify_digest_match(payload, header.payload_digest)?;
```

### Current State: PROTECTED
`verify_digest_match` is called immediately after slicing, before returning.

### Residual Risk
If `verify_digest_match` is moved or made optional, corrupt payloads could be returned.

---

## H6: CRC32C Header Checksum Bypass

### Description
A corrupt header has valid magic/version/kind and passes budget check, but header bytes are corrupted. Without CRC32C check, corrupt header could produce wrong `payload_len` or `header_len`.

### Affected Code Path
```rust
// decode_record_header (line 54-56)
if crc32c::crc32c(header_prefix_for_crc(header)?) != decoded.header_checksum {
    return Err(JournalError::HeaderChecksumMismatch);
}
```

### Current State: PROTECTED
CRC32C check is at line 54, AFTER budget check at line 48.

### Ordering Concern
Budget check (line 48) precedes CRC32C check (line 54). This is correct: budget must be enforced even if header is corrupt, to prevent allocation bombs.

---

## H7: Hostile Input — Max Payload = 0

### Description
`decode_record_header` called with `max_payload_len = 0`. Any record with `payload_len > 0` is rejected with `PayloadTooLarge`. This is correct behavior.

### Affected Code Path
```rust
if decoded.payload_len > max_payload_len {  // line 48
    return Err(PayloadTooLarge { ... });
}
```

### Current State: DEFINED BEHAVIOR
When `max_payload_len = 0`, only zero-length payloads are accepted. No allocation risk.

---

## H8: Schema Version Confusion

### Description
A record with schema version > CURRENT_SCHEMA_VERSION is written (future version). Reading it should fail with `UnsupportedSchemaVersion`, not succeed silently.

### Affected Code Path
```rust
// decode_record_header line 40
validate_schema_version(decoded.schema_version)?;
// In validation module:
pub fn validate_schema_version(version: u16) -> Result<(), JournalError> {
    if version > CURRENT_SCHEMA_VERSION {
        return Err(JournalError::UnsupportedSchemaVersion { version });
    }
    // old schema: MigrationRequired
    if version < CURRENT_SCHEMA_VERSION {
        return Err(JournalError::MigrationRequired { from: version, to: CURRENT_SCHEMA_VERSION });
    }
    Ok(())
}
```

### Current State: PROTECTED

---

## H9: Recovery Replay Divergence via Corrupt Snapshot

### Description
A corrupt snapshot passes budget check but contains invalid state. During `recover_snapshot_plus_tail`, corrupt snapshot produces wrong `RecoveryFrameSeed`, leading to replay divergence.

### Affected Code Path
```rust
// recover_snapshot_plus_tail
let snapshot = journal.snapshot(run, seq)?;  // budget check inside
// ... hydrate from snapshot ...
```

### Current State: PROTECTED
Budget check in `decode_record_header` applies to snapshot reads. BLAKE3 digest check catches corrupt payload. `postcard::from_bytes` catches deserialize errors.

### Residual Risk
Semantic validity: A snapshot could deserialize successfully (correct postcard bytes, correct length, correct digest) but contain values that cause replay divergence. `RunSnapshot` fields (`slots`, `taint` as `Vec<u8>`) are opaque bytes to storage; runtime validation happens in `hydrate_run_frame`.

---

## H10: Fjall KV Separation + Budget Interaction

### Description
`KeyspaceProfile::Cold` and `Blob` enable KV separation (value stored separately from key). Large blob reads go through Fjall's value retrieval which may allocate beyond `max_bytes`.

### Affected Code Path
```rust
// decode_optional
let Some(value) = keyspace.get(key)? else { return Ok(None) };
// value is &[u8] from Fjall; Fjall handles KV separation internally
let (_, record) = decode_record(value.as_ref(), magic, max_bytes)?;
```

### Current State: NOT A RISK
`decode_record_header` budget check is applied to the RETRIEVED value bytes. Fjall's KV separation doesn't bypass the codec budget check. The `max_bytes` is enforced by vb_storage, not Fjall.

---

## Hazard Summary Table

| Hazard | Type | Severity | Status | Mitigation |
|--------|------|----------|--------|-----------|
| H1: Budget bypass (early alloc) | Runtime | Critical | NOT VULNERABLE | Kani proof of no-alloc before line 48 |
| H2: Magic confusion | Security | High | NOT VULNERABLE | Keyspace isolation + magic check |
| H3: Overflow in payload_end | Runtime | High | PROTECTED | checked_add |
| H4: Postcard over-alloc | Runtime | Medium | PROTECTED | Bounded slice before postcard |
| H5: Digest bypass | Integrity | High | PROTECTED | verify_digest_match called |
| H6: CRC32C bypass | Integrity | High | PROTECTED | CRC check at line 54 |
| H7: max_payload=0 | Defined | Low | CORRECT | No allocation possible |
| H8: Schema confusion | Compatibility | Medium | PROTECTED | validate_schema_version |
| H9: Corrupt snapshot divergence | Recovery | High | PROTECTED | Digest + postcard + runtime validation |
| H10: KV separation bypass | Performance | None | NOT A RISK | Budget enforced after retrieval |

## Proof Obligations

1. **Kani**: Prove `decode_record_header` returns `Err(PayloadTooLarge)` before any path that could allocate `> max_payload_len` bytes
2. **Kani**: Prove `decode_record_payload` never slices `bytes[60..]` beyond `max_payload_len`
3. **Kani**: Prove `checked_add` overflow check in `decode_record_payload` prevents wraparound
4. **Verus**: Prove `decode_record_header` is a total function on `&[u8]` (no panic)
5. **TLA+**: Prove keyspace prefix distinctness prevents cross-keyspace reads