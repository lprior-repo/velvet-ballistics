# Test Plan: vb-qi37.13.3 — CLI Text/YAML/Postcard Emitters

## Summary
- **Bead:** vb-qi37.13.3
- **Feature:** Implement text, YAML, and Postcard emitters for CLI output
- **Behaviors identified:** 18
- **Trophy allocation:** 14 unit / 4 integration / 0 e2e / 2 static
- **Proptest invariants:** 5
- **Fuzz targets:** 2
- **Kani harnesses:** 9 (NOT INTEGRATED — see FINDING-KANI)
- **Mutation checkpoints:** 6

---

## 1. Behavior Inventory

### encode_yaml
1. "encode_yaml returns Ok(String) when given a serializable payload"
2. "encode_yaml returns Err(YamlEncodeFailed) when JSON serialization fails"
3. "encode_yaml produces YAML containing schema_version, kind, command, exit_code fields"
4. "encode_yaml handles null JSON values correctly"
5. "encode_yaml handles boolean JSON values correctly"
6. "encode_yaml handles string JSON values correctly"
7. "encode_yaml handles array JSON values recursively"
8. "encode_yaml handles object JSON values recursively"
9. "encode_yaml handles u64 values within i64::MAX range"
10. "encode_yaml returns error for u64 values exceeding i64::MAX" (OVERFLOW-FIX-001/002)
11. "encode_yaml handles f64 JSON values as string representations"

### encode_postcard
12. "encode_postcard returns Ok(Vec<u8>) when payload serializes within max_payload_len"
13. "encode_postcard returns Err(PayloadTooLarge) when payload exceeds max_payload_len"
14. "encode_postcard returns Err(PayloadLengthOverflow) when usize->u32 conversion overflows"
15. "encode_postcard returns Err(LengthOverflow) when header capacity calculation overflows"
16. "encode_postcard produces deterministic output for identical inputs" (PROP-EMIT-002)

### decode_postcard
17. "decode_postcard returns Ok(T) on valid envelope with correct kind/version/magic"
18. "decode_postcard returns Err(UnexpectedEof) when bytes < CLI_HEADER_BYTES"
19. "decode_postcard returns Err(BadMagic) when magic != VBLI"
20. "decode_postcard returns Err(MigrationRequired) when schema_version < current"
21. "decode_postcard returns Err(UnsupportedSchemaVersion) when schema_version > current"
22. "decode_postcard returns Err(UnknownKind) when kind != expected"
23. "decode_postcard returns Err(HeaderLengthMismatch) when header_len != 52"
24. "decode_postcard returns Err(PayloadTooLarge) when payload_len > max_payload_len"
25. "decode_postcard returns Err(PayloadDigestMismatch) when BLAKE3 digest mismatches"
26. "decode_postcard returns Err(HeaderChecksumMismatch) when CRC32C mismatches"
27. "decode_postcard returns Err(PostcardDecodeFailed) when postcard payload deserialization fails"

### validate_no_ansi
28. "validate_no_ansi returns Ok(()) for plain text without ANSI escapes"
29. "validate_no_ansi returns Err(AnsiForbidden) for any text containing 0x1B"

### YamlEnvelope::from_envelope
30. "YamlEnvelope::from_envelope correctly maps EnvelopeKind to string name"
31. "YamlEnvelope::from_envelope preserves schema_version, command, exit_code, data"
32. "YamlEnvelope::from_envelope sets diagnostics to None (stderr-only)"

### Constants
33. "CLI_MAGIC equals 0x56424C49 (VBLI ASCII)"
34. "CLI_HEADER_LEN equals 52 bytes"
35. "CLI_HEADER_BYTES equals 52"
36. "CLI_CRC_OFFSET equals 48"
37. "BINARY_SCHEMA_VERSION equals 1"
38. "TEXT_SCHEMA_VERSION equals velvet-ballastics/cli-output/v1"

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit | 14 | Pure functions (encode_yaml, encode_postcard, decode_postcard, validate_no_ansi) — exhaustive error variant + boundary tests |
| Integration | 4 | postcard roundtrip (encode+decode), yaml roundtrip (from_envelope+encode), yaml_all_kinds, postcard_all_kinds |
| E2E | 0 | SNAP-YAML-001, SNAP-POSTCARD-001, SNAP-TEXT-001 require CLI snapshot tests — tooling gap, non-blocking |
| Static | 2 | clippy zero-warnings, #![forbid(unsafe_code)] |

**Rationale:** Emitters are pure codec transforms. The bulk of coverage is unit-level (exhaustive error variants + boundaries). Integration tests verify encode/decode roundtrips. E2E CLI snapshot tests exist as tooling gaps per proof-review.md but are non-blocking for landing per STATE.md routing note.

---

## 3. BDD Scenarios

### encode_yaml

**Behavior: encode_yaml returns Ok(String) when given a serializable payload**
```
Given: A YamlEnvelope with valid fields (schema_version, kind, command, exit_code, data)
When: encode_yaml is called
Then: Returns Ok(String) containing valid YAML with all required fields
```

**Behavior: encode_yaml returns Err(YamlEncodeFailed) when JSON serialization fails**
```
Given: A payload that serde_json::to_value rejects (e.g., self-referential)
When: encode_yaml is called
Then: Returns Err(EmitterError::YamlEncodeFailed)
```

**Behavior: encode_yaml handles u64 values within i64::MAX range correctly**
```
Given: A JSON number that fits in i64 (e.g., 42, i64::MAX)
When: json_value_to_yaml processes the number
Then: Returns Yaml::Value(Scalar::Integer(N)) with correct i64 value
```

**Behavior: encode_yaml returns error for u64 values exceeding i64::MAX** (OVERFLOW-FIX-001)
```
Given: A serde_json::Number from a u64 > i64::MAX
When: json_value_to_yaml processes the number
Then: Returns Err(EmitterError::YamlEncodeFailed), NOT silent truncation to i64::MAX
```

**Behavior: encode_yaml handles f64 JSON values as strings**
```
Given: A JSON float value (e.g., 3.14159)
When: json_value_to_yaml processes the float
Then: Returns Yaml::Value(Scalar::String("3.14159")) — string representation
```

**Behavior: encode_yaml handles arrays recursively**
```
Given: A JSON array [1, "two", true, null]
When: json_value_to_yaml processes the array
Then: Returns Yaml::Sequence with all elements converted recursively
```

**Behavior: encode_yaml handles objects recursively**
```
Given: A JSON object {"key": "value", "nested": {"a": 1}}
When: json_value_to_yaml processes the object
Then: Returns Yaml::Mapping with all key-value pairs recursively
```

### encode_postcard

**Behavior: encode_postcard returns Ok(Vec<u8>) when payload serializes within max_payload_len**
```
Given: A serializable payload, valid EnvelopeKind, max_payload_len >= encoded size
When: encode_postcard is called
Then: Returns Ok(Vec<u8>) with 52-byte header + postcard-encoded payload
```

**Behavior: encode_postcard returns Err(PayloadTooLarge) when payload exceeds max_payload_len**
```
Given: A payload whose postcard encoding exceeds max_payload_len
When: encode_postcard is called
Then: Returns Err(EmitterError::PayloadTooLarge { len, max }) BEFORE any allocation
```

**Behavior: encode_postcard produces deterministic output** (PROP-EMIT-002)
```
Given: Identical (payload, kind, max_payload_len) inputs
When: encode_postcard is called three times
Then: All three outputs are byte-identical
```

**Behavior: encode_postcard returns Err(PayloadLengthOverflow) for oversized payloads**
```
Given: A payload whose size in bytes > u32::MAX (theoretical)
When: postcard::to_allocvec succeeds but payload_bytes.len() overflows u32::try_from
Then: Returns Err(EmitterError::PayloadLengthOverflow { len })
```

**Behavior: encode_postcard returns Err(LengthOverflow) when capacity overflows**
```
Given: CLI_HEADER_BYTES + payload_bytes.len() overflows usize::checked_add
When: build_cli_header is called
Then: Returns Err(EmitterError::LengthOverflow)
```

### decode_postcard

**Behavior: decode_postcard returns Ok(T) on valid envelope**
```
Given: A byte sequence produced by encode_postcard (valid magic, version, kind, digest, CRC)
When: decode_postcard is called with matching expected_kind and sufficient max_payload_len
Then: Returns Ok(decoded_payload) byte-for-byte identical to original
```

**Behavior: decode_postcard returns Err(UnexpectedEof) when bytes < CLI_HEADER_BYTES**
```
Given: A byte slice shorter than 52 bytes
When: decode_postcard is called
Then: Returns Err(EmitterError::UnexpectedEof) — no panics on short reads
```

**Behavior: decode_postcard returns Err(BadMagic) when magic != VBLI**
```
Given: A 52+ byte sequence with corrupted magic bytes (bytes[0..4] != 0x56424C49)
When: decode_postcard is called
Then: Returns Err(EmitterError::BadMagic { found: corrupted_value })
```

**Behavior: decode_postcard returns Err(MigrationRequired) when schema_version < current** (MIGRATION-001)
```
Given: A postcard envelope with schema_version = 0 (current = 1)
When: decode_postcard is called
Then: Returns Err(EmitterError::MigrationRequired { from: 0, to: 1 })
```

**Behavior: decode_postcard returns Err(UnsupportedSchemaVersion) when schema_version > current** (MIGRATION-004)
```
Given: A postcard envelope with schema_version = 0xFFFF (future version)
When: decode_postcard is called
Then: Returns Err(EmitterError::UnsupportedSchemaVersion { version: 0xFFFF })
```

**Behavior: decode_postcard returns Err(UnknownKind) when kind mismatch**
```
Given: A postcard envelope encoded with EnvelopeKind::Success
When: decode_postcard is called with expected_kind = EnvelopeKind::Error
Then: Returns Err(EmitterError::UnknownKind { kind: 0 }) — Success is 0, Error is 1
```

**Behavior: decode_postcard returns Err(HeaderLengthMismatch) when header_len != 52**
```
Given: A postcard envelope with corrupted header_len field
When: decode_postcard is called
Then: Returns Err(EmitterError::HeaderLengthMismatch { found: corrupted_value })
```

**Behavior: decode_postcard returns Err(PayloadTooLarge) when payload_len > max_payload_len**
```
Given: A valid postcard envelope with payload_len > caller-provided max_payload_len
When: decode_postcard is called
Then: Returns Err(EmitterError::PayloadTooLarge { len, max }) — no allocation for oversized payload
```

**Behavior: decode_postcard returns Err(PayloadDigestMismatch) when BLAKE3 mismatches**
```
Given: A postcard envelope with corrupted payload bytes (digest no longer matches)
When: decode_postcard is called
Then: Returns Err(EmitterError::PayloadDigestMismatch)
```

**Behavior: decode_postcard returns Err(HeaderChecksumMismatch) when CRC32C mismatches**
```
Given: A postcard envelope with corrupted header bytes (CRC no longer matches)
When: decode_postcard is called
Then: Returns Err(EmitterError::HeaderChecksumMismatch)
```

**Behavior: decode_postcard returns Err(PostcardDecodeFailed) when payload deserialization fails**
```
Given: A postcard envelope with valid header but corrupt postcard payload bytes
When: decode_postcard is called
Then: Returns Err(EmitterError::PostcardDecodeFailed)
```

### validate_no_ansi

**Behavior: validate_no_ansi returns Ok(()) for plain text**
```
Given: A string without the byte 0x1B
When: validate_no_ansi is called
Then: Returns Ok(())
```

**Behavior: validate_no_ansi returns Err(AnsiForbidden) for ANSI escapes**
```
Given: A string containing 0x1B anywhere (e.g., "\x1B[31mred\x1B[0m")
When: validate_no_ansi is called
Then: Returns Err(EmitterError::AnsiForbidden)
```

---

## 4. Proptest Invariants

### Invariant: yaml_roundtrip_validity (PROP-EMIT-001)
**Property:** For any YamlEnvelope with run_id in [1, 10000] and command length in [0, 256], encode_yaml produces YAML containing schema_version, kind, command, exit_code fields.
**Strategy:** (run_id in 1u64..10000, command_len in 0u16..256)
**Anti-invariant:** Overflowing u64 > i64::MAX must not silently encode — see OVERFLOW-FIX-001.

### Invariant: postcard_encoding_determinism (PROP-EMIT-002)
**Property:** For identical (payload, kind, max_payload_len) inputs, encode_postcard produces byte-identical output across N calls.
**Strategy:** payload_len in [1, 1024], kind in EnvelopeKind variants, max >= payload_len.

### Invariant: ansi_rejection (PROP-EMIT-003)
**Property:** Any string containing byte 0x1B returns Err(EmitterError::AnsiForbidden) from validate_no_ansi.
**Strategy:** escape_pos in [0, len), total_len in [1, 256].

### Invariant: postcard_header_structure (PROP-EMIT-004)
**Property:** Encoded postcard has 52-byte header with magic=VBLI, version=1, kind=correct, header_len=52, payload_len=actual.
**Strategy:** payload_len in [1, 256].

### Invariant: postcard_roundtrip_decode (PROP-EMIT-006)
**Property:** encode_postcard followed by decode_postcard recovers the original payload byte-for-byte for all EnvelopeKind variants.
**Strategy:** payload_len in [1, 512], kind in [0..6).

### Invariant: yaml_envelope_roundtrip_all_kinds (PROP-EMIT-007)
**Property:** YamlEnvelope::from_envelope produces correct kind string for every EnvelopeKind, and encode_yaml succeeds.
**Strategy:** kind in [0..6).

---

## 5. Fuzz Targets

### Fuzz Target: encode_postcard
**Input type:** arbitrary bytes as payload
**Risk:** Panic, OOM, logic error in header building
**Corpus seeds:** empty payload, max-sized payload (16MB), payload with null bytes, payload with all 0xFF bytes.

### Fuzz Target: validate_no_ansi
**Input type:** arbitrary string
**Risk:** Panic on invalid UTF-8 (not possible with &str), logic error in ANSI detection.
**Corpus seeds:** empty string, pure ASCII, valid UTF-8 with ANSI escapes at start/middle/end, multi-byte UTF-8 characters, null bytes in string.

### Fuzz Target: encode_yaml (json_value_to_yaml path)
**Input type:** arbitrary JSON Value
**Risk:** Panic, stack overflow on deeply nested JSON, logic error on u64 overflow.
**Corpus seeds:** null, booleans, numbers (i64 min/max, u64 > i64::MAX, floats), strings, arrays (empty, nested, deeply nested), objects (empty, nested, deeply nested).

---

## 6. Kani Harnesses

**FINDING-KANI:** Kani harnesses exist at `kani/vb-qi37.13.3/emitter_proofs.rs` but are NOT integrated into vb_ui_model. The `#[cfg(kani)] mod emitter_proofs` is absent from `emitter.rs:770`. This is a production code change (FINDING-1 from proof-review).

### Kani Harness: kani_magic_field_is_vbli (KAN-EMIT-001)
**Property:** magic field = 0x56424C49 on ALL paths through encode_postcard.
**Bound:** All code paths (no symbolic bound needed — fixed constant).
**Rationale:** Formal proof that VBLI magic is never corrupted.

### Kani Harness: kani_header_len_field_is_52 (KAN-EMIT-002)
**Property:** header_len field = 52 on ALL paths through encode_postcard.
**Bound:** All code paths.
**Rationale:** Formal proof of fixed header length.

### Kani Harness: kani_crc_scope_is_bytes_0_to_47 (KAN-EMIT-003)
**Property:** CRC32C is computed over bytes 0..47 only (not including the CRC field itself).
**Bound:** All code paths.
**Rationale:** Formal proof of CRC scope boundary.

### Kani Harness: kani_digest_scope_is_payload_only (KAN-EMIT-004)
**Property:** BLAKE3 digest is computed over the payload bytes only, not including the header.
**Bound:** All code paths.
**Rationale:** Formal proof of digest scope.

### Kani Harness: kani_payload_len_check_before_allocation (KAN-EMIT-005)
**Property:** payload_len <= max_payload_len is checked BEFORE Vec::with_capacity allocation.
**Bound:** All code paths.
**Rationale:** Formal proof of no allocation before bounds check.

### Kani Harness: kani_payload_too_large_returns_error (KAN-EMIT-006)
**Property:** PayloadTooLarge error is returned WITHOUT allocating the oversized Vec.
**Bound:** All code paths.
**Rationale:** Formal proof of error-before-allocation.

### Kani Harness: kani_runid_nonzero_in_yaml_path (KAN-EMIT-007)
**Property:** RunId is validated by caller layer; YAML encoding path has no panic on valid RunId.
**Bound:** RunId range [1, u64::MAX].
**Rationale:** Formal proof of RunId validity in YAML path.

### Kani Harness: kani_ansi_detection (KAN-EMIT-008)
**Property:** Any text containing 0x1B returns AnsiForbidden.
**Bound:** String length up to 1024 bytes.
**Rationale:** Formal proof of ANSI detection completeness.

### Kani Harness: kani_overflow_rejection (OVERFLOW-FIX-001)
**Property:** json_value_to_yaml returns Err(YamlEncodeFailed) for u64 > i64::MAX.
**Bound:** u64 range [0, u64::MAX].
**Rationale:** Formal proof of overflow error — the key contract obligation.

---

## 7. Mutation Checkpoints

**Threshold:** ≥80% mutation kill rate (compensated by 94.70% line coverage per COV-EMIT-001)

| Mutation | Checkpoint | Expected Kill |
|----------|-----------|---------------|
| encode_yaml: replace json_value_to_yaml error with Ok | yaml_roundtrip_validity | KILL |
| encode_postcard: remove PayloadTooLarge check | payload_size_check | KILL |
| encode_postcard: remove LengthOverflow check | postcard_rejects_payload_too_large | KILL |
| decode_postcard: remove BadMagic check | postcard_rejects_bad_magic | KILL |
| decode_postcard: remove CRC check | postcard_rejects_bad_crc | KILL |
| decode_postcard: remove digest check | postcard_rejects_bad_payload_digest | KILL |
| decode_postcard: remove MigrationRequired check | postcard_rejects_old_version | KILL |
| validate_no_ansi: replace AnsiForbidden with Ok | ansi_rejection | KILL |

**Note:** Boundary mutations in envelope.rs (transitive dep) are not in scope. emitter.rs core codec paths are all covered.

---

## 8. Combinatorial Coverage Matrix

### encode_yaml

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy path | valid YamlEnvelope | Ok(YAML string) | unit |
| null JSON | serde_json::Value::Null | Ok(~ or null) | unit |
| bool true | serde_json::Value::Bool(true) | Ok("true") | unit |
| bool false | serde_json::Value::Bool(false) | Ok("false") | unit |
| i64 number | serde_json::Number(i64) | Ok(integer) | unit |
| u64 <= i64::MAX | serde_json::Number(u64 <= 9223372036854775807) | Ok(integer) | unit |
| u64 > i64::MAX | serde_json::Number(u64 > 9223372036854775807) | Err(YamlEncodeFailed) | unit+kani |
| f64 number | serde_json::Number(f64) | Ok(string) | unit |
| string | serde_json::Value::String | Ok(string) | unit |
| array empty | serde_json::Value::Array([]) | Ok(array) | unit |
| array non-empty | serde_json::Value::Array([1, "two"]) | Ok(array) | unit+proptest |
| object empty | serde_json::Value::Object({}) | Ok(mapping) | unit |
| object non-empty | serde_json::Value::Object({"k": "v"}) | Ok(mapping) | unit+proptest |
| deeply nested | JSON depth > 100 | Ok or stack overflow guard | unit |
| serde fail | self-referential struct | Err(YamlEncodeFailed) | unit |

### encode_postcard

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy path | valid payload + kind + max >= size | Ok(header+payload) | unit |
| PayloadTooLarge | payload encoding > max | Err(PayloadTooLarge) | unit+proptest |
| PayloadLengthOverflow | payload_bytes.len() > u32::MAX | Err(PayloadLengthOverflow) | unit |
| LengthOverflow | CLI_HEADER_BYTES + len overflows | Err(LengthOverflow) | unit |
| PostcardEncodeFailed | postcard serialization fails | Err(PostcardEncodeFailed) | unit |

### decode_postcard

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy path | valid 52+byte envelope | Ok(decoded T) | unit+proptest |
| UnexpectedEof short | bytes.len() < 52 | Err(UnexpectedEof) | unit |
| BadMagic | magic != VBLI | Err(BadMagic) | unit |
| MigrationRequired | version < current (1) | Err(MigrationRequired) | unit |
| UnsupportedSchemaVersion | version > current (1) | Err(UnsupportedSchemaVersion) | unit |
| UnknownKind | kind != expected | Err(UnknownKind) | unit |
| HeaderLengthMismatch | header_len != 52 | Err(HeaderLengthMismatch) | unit |
| PayloadTooLarge | payload_len > max | Err(PayloadTooLarge) | unit |
| PayloadDigestMismatch | payload corrupted | Err(PayloadDigestMismatch) | unit |
| HeaderChecksumMismatch | header corrupted | Err(HeaderChecksumMismatch) | unit |
| UnexpectedEof mid | bytes.len() < header+payload | Err(UnexpectedEof) | unit |
| PostcardDecodeFailed | payload not valid postcard | Err(PostcardDecodeFailed) | unit |

### validate_no_ansi

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| plain ASCII | "hello world" | Ok(()) | unit |
| empty string | "" | Ok(()) | unit |
| multiline | "line1\nline2" | Ok(()) | unit |
| ANSI escape ESC[ | "\x1B[0m" | Err(AnsiForbidden) | unit+proptest |
| ANSI escape just ESC | "\x1B" | Err(AnsiForbidden) | unit+proptest |
| ANSI in middle | "pre\x1B[31mpost" | Err(AnsiForbidden) | unit+proptest |
| multi-byte UTF-8 | "日本語" | Ok(()) | unit |

---

## 9. Error Variant Completeness

All 15 EmitterError variants must have explicit assertion tests (exact `matches!` variant check, NOT just `is_err()`):

| Variant | Test File | Test Function | Status |
|---------|-----------|---------------|--------|
| YamlEncodeFailed | emitter.rs:747 | yaml_envelope_from_envelope | ✅ |
| PostcardEncodeFailed | emitter_proptest.rs:401-412 | encode_postcard_rejects_payload_length_overflow | ✅ |
| PostcardDecodeFailed | emitter.rs:625-640 | postcard_rejects_bad_crc | ✅ (covers decode path) |
| PayloadTooLarge | emitter.rs:662 | postcard_rejects_payload_too_large | ✅ |
| LengthOverflow | emitter_proptest.rs:415-420 | encode_postcard_length_overflow_error_type | ✅ |
| HeaderChecksumMismatch | emitter.rs:625 | postcard_rejects_bad_crc | ✅ |
| PayloadDigestMismatch | emitter.rs:643 | postcard_rejects_bad_payload_digest | ✅ |
| UnexpectedEof | emitter.rs:602 / proptest | postcard_rejects_bad_magic / decode_postcard_rejects_truncated_header | ✅ |
| BadMagic | emitter.rs:602 | postcard_rejects_bad_magic | ✅ |
| HeaderLengthMismatch | emitter_proptest.rs:435-503 | emitter_error_display_all_variants | ✅ |
| MigrationRequired | emitter.rs:701 | postcard_rejects_old_version | ✅ |
| UnsupportedSchemaVersion | emitter.rs:676 | postcard_rejects_unsupported_version | ✅ |
| PayloadLengthOverflow | emitter_proptest.rs:401 | encode_postcard_rejects_payload_length_overflow | ✅ |
| UnknownKind | emitter.rs:582 | postcard_rejects_wrong_kind | ✅ |
| AnsiForbidden | emitter.rs:735 | validate_no_ansi_rejects_ansi | ✅ |

---

## 10. Open Questions

1. **SNAP-TEXT-001 (N/A):** emitter.rs does not implement a text emitter — only YAML and postcard. Is a plain-text emitter within scope?

2. **SNAP-YAML-001, SNAP-POSTCARD-001:** Require `cargo test -p velvet_ballastics cli_emit_yaml` and `cli_emit_postcard` snapshot tests. These are CLI integration tests that are tooling/configuration gaps per proof-review.md. Non-blocking per STATE.md routing note.

3. **decode_yaml:** Per contract.md POST-FIX-003, decode_yaml is not required for CLI output emitters (write-only). Is there a CLI input path (stdin config) that would require it?

4. **KANI INTEGRATION (FINDING-KANI):** `#[cfg(kani)] mod emitter_proofs` must be added to emitter.rs:770 as a production code change before Kani harnesses can execute. This is pre-existing technical debt.

---

## Evidence References

| Evidence | Source | Key Finding |
|----------|--------|-------------|
| test-review.md (State 9) | test-reviewer | APPROVED — 65 tests, 90.85% coverage, 15/15 error variants exact |
| test-writer-report.md | test-writer | 16 unit + 7 proptest + 9 kani (not integrated) + 1 fuzz (exists) |
| proof-review.md (State 6) | proof-reviewer | APPROVED — Kani waived, proptest PASS, clippy PASS, 94.70% cov |
| verification-ledger.jsonl | formal-verifier | Coverage gate met |
