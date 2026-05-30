# ARCH-DRIFT REPORT: `kani_postcard_envelope_wire.rs`

**File**: `crates/vb_storage/src/kani_postcard_envelope_wire.rs`
**Lines**: 337 (**VIOLATION**: exceeds `<300` limit by 37 lines)
**Date**: 2026-05-29
**Enforcer**: architectural-drift

---

## Summary

This Kani proof harness file verifies VB-STORAGE-POSTCARD-ENVELOPE-001 (decode order
enforcement for storage record envelopes). It contains **5 proof harnesses** that all
suffer from severe **primitive obsession** and copy-paste duplication. The file
proves decode ordering but does so using raw byte manipulation with no domain
abstractions.

---

## Critical Violations

### 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 337 | <300 | **FAIL** |

---

## Primitive Obsession Violations (Scott Wlaschin)

### A. Magic Bytes — No `Magic` Newtype

**Current (PRIMITIVE)**:
```rust
let expected_magic: u32 = kani::any();
let wrong_magic: u32 = kani::any();
kani::assume(wrong_magic != expected_magic);
header[0..4].copy_from_slice(&wrong_magic.to_le_bytes());
```

**Required (VALUE OBJECT)**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Magic(u32);

impl Magic {
    const EXPECTED: Magic = Magic(0x4a_xx_-xx_xx); // actual expected value
}

// In proof: let wrong_magic = Magic::arbitrary(exclude: expected_magic);
```

**Violation**: Magic is a domain concept with validity constraints (4-byte big-endian
magic at offset 0). It should be a typed `ValueObject`, not a raw `u32`.

---

### B. Schema Version — No `SchemaVersion` Type

**Current (PRIMITIVE)**:
```rust
header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
```

**Required**: Should be `SchemaVersion(u16)` with validation.

---

### C. RecordKind — No Typed Kind

**Current (PRIMITIVE)**:
```rust
header[6..8].copy_from_slice(&RecordKind::RunAccepted.id().to_le_bytes());
```

`RecordKind::id()` returns raw `u16`. No newtype.

---

### D. Header Length Field — No `HeaderLen` Type

**Current (PRIMITIVE)**:
```rust
header[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
```

`RECORD_HEADER_LEN` is a raw `u32` constant (60). No domain type wrapping the
constant as a singleton type.

---

### E. Payload Length — No `PayloadLen` Type

**Current (PRIMITIVE)**:
```rust
let oversized_payload_len: u32 = kani::any();
kani::assume(oversized_payload_len > max_payload);
header[12..16].copy_from_slice(&oversized_payload_len.to_le_bytes());
```

**Required**: `PayloadLen(u32)` with `MaxPayload(u32)` as a associated constraint
type, not two raw `u32` values.

---

### F. Timestamp — No `EventSeq` Type

**Current (PRIMITIVE)**:
```rust
header[16..24].copy_from_slice(&0u64.to_le_bytes());
```

Raw `u64` with no domain type.

---

### G. Digest — No `PayloadDigest` Type

**Current (PRIMITIVE)**:
```rust
for i in 0..DIGEST_BYTES {
    header[24 + i] = kani::any();
}
header[24..24 + DIGEST_BYTES].copy_from_slice(correct_digest.as_bytes());
```

`[u8; 32]` is a raw byte array. Should be `PayloadDigest([u8; 32])`.

---

### H. CRC — No `HeaderCrc` Type

**Current (PRIMITIVE)**:
```rust
let crc = crc32c::crc32c(&header[..CRC_OFFSET]);
header[CRC_OFFSET..CRC_OFFSET.saturating_add(4)].copy_from_slice(&crc.to_le_bytes());
```

Raw `u32`. Should be `HeaderCrc(u32)`.

---

## Byte Layout Hardcoding

All five harnesses repeat the same magic byte offsets:

| Field | Offset | Size | Hardcoded In |
|-------|--------|------|-------------|
| Magic | 0 | 4 | All 5 proofs |
| SchemaVer | 4 | 2 | All 5 proofs |
| RecordKind | 6 | 2 | All 5 proofs |
| HeaderLen | 8 | 4 | All 5 proofs |
| PayloadLen | 12 | 4 | All 5 proofs |
| Timestamp | 16 | 8 | All 5 proofs |
| Digest | 24 | 32 | All 5 proofs |
| CRC | 56 | 4 | All 5 proofs |

**DRY Violation**: These offsets appear ~40 times across the 5 harnesses. No single
source of truth for the header layout.

---

## Boilerplate Duplication

Each harness independently reproduces:

```rust
// SET MAGIC
header[0..4].copy_from_slice(&valid_magic.to_le_bytes());
// SET SCHEMA VERSION
header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
// SET RECORD KIND
header[6..8].copy_from_slice(&RecordKind::RunAccepted.id().to_le_bytes());
// SET HEADER LEN
header[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
// SET PAYLOAD LEN
header[12..16].copy_from_slice(&payload_len.to_le_bytes());
// SET TIMESTAMP
header[16..24].copy_from_slice(&0u64.to_le_bytes());
// SET DIGEST
for i in 0..DIGEST_BYTES {
    header[24 + i] = kani::any();
}
// SET CRC
let crc = crc32c::crc32c(&header[..CRC_OFFSET]);
header[CRC_OFFSET..CRC_OFFSET.saturating_add(4)].copy_from_slice(&crc.to_le_bytes());
```

This 12-line block appears **5 times** (60 lines of pure duplication).

---

## Missing Domain Abstractions

### 1. No `RecordHeader` Newtype

The file treats `[u8; 60]` as an opaque byte array. A `RecordHeader` newtype wrapper
with typed accessors would eliminate all offset constants:

```rust
struct RecordHeader([u8; 60]);

impl RecordHeader {
    fn magic(&self) -> Magic { Magic(u32::from_le_bytes(...)) }
    fn schema_version(&self) -> SchemaVersion { ... }
    fn payload_len(&self) -> PayloadLen { ... }
    fn digest(&self) -> PayloadDigest { ... }
    fn crc(&self) -> HeaderCrc { ... }
}
```

### 2. No `HeaderBuilder` for Test/Proof Construction

Each proof manually assembles headers. A `HeaderBuilder` would:

```rust
struct HeaderBuilder {
    magic: u32,
    schema_version: u16,
    kind: u16,
    header_len: u32,
    payload_len: u32,
    timestamp: u64,
    digest: [u8; 32],
}

impl HeaderBuilder {
    fn build_with_valid_crc(self) -> [u8; 60] { ... }
}
```

### 3. No `kani::Arbitrary` for Domain Types

The proofs use `kani::any()` on raw `u32`, `u64`, and `[u8; N]` arrays.
Implementing `kani::Arbitrary` for `Magic`, `PayloadLen`, `PayloadDigest` etc.
would:

- Ensure generated values satisfy domain constraints
- Eliminate `kani::assume()` guards like `kani::assume(wrong_magic != expected_magic)`
- Make the proof harness self-documenting

---

## Kani-Specific Issues

### 1. Hardcoded Unwind Bounds

```rust
#[kani::unwind(4)]
```

`4` is a magic number with no justification. Unwind bounds should be derived from
the maximum loop/recursion depth in `decode_record_header`, not hardcoded.

### 2. No `kani::Arbitrary` for Core Structures

The harness generates header bytes via:
```rust
let header: [u8; RECORD_HEADER_BYTES] = kani::any();
```

This creates arbitrary bytes that must then be manually shaped. Compare to the
required form where `kani::any::<Magic>()` returns a `Magic` that is already
validated.

### 3. CRC Corruption is Manual

```rust
let bad_crc = good_crc.wrapping_add(1);
```

This single-bit flip is fine for CRC corruption, but the approach is ad-hoc.
A `Corrupt` trait or `CrcCorruptor` would be more systematic.

---

## Recommendations

### Immediate (Refactor to Pass)

1. **Extract a `HeaderBuilder`** utility module with a builder pattern for proof
   harness construction. This eliminates ~60 lines of duplication.

2. **Implement `kani::Arbitrary`** for `Magic`, `PayloadLen`, `PayloadDigest`,
   and `HeaderCrc`. This eliminates manual byte manipulation in proofs.

3. **Move to a typed `RecordHeader` struct** that encapsulates the 60-byte layout
   with accessor methods. Proofs use typed fields, not raw byte offsets.

4. **Justify unwind bounds** with a constant derived from code analysis, or use
   `#[kani::unwind(N)]` where N is documented as `max_steps_in_decode`.

### Short-Term (DDD Compliance)

5. **Create value objects**: `Magic(u32)`, `SchemaVersion(u16)`,
   `RecordKind(u16)`, `PayloadLen(u32)`, `EventSeq(u64)`,
   `PayloadDigest([u8; 32])`, `HeaderCrc(u32)`.

6. **Replace offset constants** with `RecordHeader` accessor methods.

7. **Add a `DecodeOrder` enum** that explicitly models the steps:
   ```rust
   enum DecodeStep { Magic, SchemaVersion, Kind, HeaderLen, PayloadLen, Crc, Digest, Postcard }
   ```
   This makes the ordering invariant visible in the type system.

---

## Verdict

| Check | Status | Details |
|-------|--------|---------|
| Line count <300 | **FAIL** | 337 lines (37 over) |
| No primitive obsession | **FAIL** | 8 primitive types used directly |
| DRY header layout | **FAIL** | ~60 lines duplicated across 5 proofs |
| Typed domain model | **FAIL** | No `Magic`, `PayloadLen`, etc. newtypes |
| Kani best practices | **PARTIAL** | Unclear unwind justification |
| Value objects for byte fields | **FAIL** | Raw `[u8; N]` arrays throughout |

**ARCHITECTURAL DRIFT**: YES — This file is a proof-of-concept harness built with
zero domain modeling. It proves correctness of decode ordering but cannot
express that ordering as a type-level invariant. The decode order is proven but
not encoded.

---

## Effort Estimate

| Task | Lines Saved | Priority |
|------|-------------|----------|
| Extract `HeaderBuilder` | ~60 | MUST |
| `kani::Arbitrary` impls | ~20 | MUST |
| Typed `RecordHeader` | ~15 | MUST |
| Unwind documentation | 0 (additive) | SHOULD |
| Value object newtypes | ~40 (eliminates offset magic) | SHOULD |

**Total reduction potential**: ~135 lines → file could be ~200 lines.
