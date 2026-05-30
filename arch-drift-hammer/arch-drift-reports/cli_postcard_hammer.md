# Architectural Drift Report: `cli_postcard.rs`

**File**: `crates/vb_cli/src/cli_postcard.rs`  
**Total Lines**: 539 (VIOLATION: 239 lines over 300-line limit)  
**Workspace**: `arch-drift-hammer`  
**Date**: 2026-05-29  
**Enforcer**: architectural-drift agent  

---

## Executive Summary

This file is a **PRIMARY DRIFT TARGET**. At 539 lines, it violates the foundational architectural rule requiring all source files to stay under 300 lines. Beyond the line count violation, the file exhibits **severe primitive obsession** throughout its type definitions, with 8+ primitive `u8`/`u16`/`u32` arrays and scalars that should be replaced with domain-typed NewTypes.

**STATUS: MANDATORY REFACTOR REQUIRED**

---

## Violation #1: Line Count (CRITICAL)

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 539 | 300 | **OVER BY 239** |
| Module code | ~200 | 300 | OK |
| Test code | ~200 | - | INLINE |
| Constants | 35 | - | INLINE |

**The file MUST be split.** Suggested decomposition:

1. `cli_postcard_types.rs` - Value objects, NewTypes, error types (~100 lines)
2. `cli_postcard_codec.rs` - Encoding/decoding logic (~150 lines)
3. `cli_postcard_validation.rs` - Validation functions (~80 lines)
4. `cli_postcard.rs` - Re-exports and module glue (~40 lines)
5. `cli_postcard_test_vectors.rs` - Test helpers (~80 lines, or move to integration tests)

---

## Violation #2: Primitive Obsession (SEVERE)

### A. Magic Bytes - `[u8; 4]`

**Current** (line 69):
```rust
pub(crate) magic: [u8; 4],
```

**Problem**: Raw byte array with no domain semantics. Callers can construct arbitrary `[u8; 4]` and call it "magic."

**Required NewType**:
```rust
pub(crate) struct MagicBytes([u8; 4]);
impl MagicBytes {
    pub(crate) const CLI: Self = MagicBytes([0x56, 0x43, 0x4C, 0x41]);
    pub(crate) fn matches(&self, other: &[u8; 4]) -> bool { self.0 == *other }
}
```

### B. Schema Version - `u16`

**Current** (line 71):
```rust
pub(crate) schema_version: u16,
```

**Problem**: Any `u16` can be passed. No guarantee it represents a valid schema version.

**Required NewType**:
```rust
pub(crate) struct SchemaVersion(u16);
impl SchemaVersion {
    pub(crate) const CURRENT: Self = SchemaVersion(1);
    pub(crate) fn is_compatible(&self) -> bool { self.0 != 0 && self.0 <= Self::CURRENT.0 }
}
```

### C. Kind - `u16`

**Current** (line 73):
```rust
pub(crate) kind: u16,
```

**Problem**: Protocol "kind" is a raw `u16`. Could be confused with other numeric identifiers.

**Required NewType**:
```rust
pub(crate) struct PostcardKind(u16);
impl PostcardKind {
    pub(crate) const CLI_POSTCARD: Self = PostcardKind(2);
}
```

### D. Header Length - `u32`

**Current** (line 75):
```rust
pub(crate) header_len: u32,
```

**Problem**: Raw `u32` for a structurally fixed value. Validation happens AFTER construction.

**Required NewType**:
```rust
pub(crate) struct HeaderLen(u32);
impl HeaderLen {
    pub(crate) const EXPECTED: Self = HeaderLen(52);
    pub(crate) fn is_valid(&self) -> bool { self.0 == Self::EXPECTED.0 }
}
```

### E. Payload Length - `u32`

**Current** (line 77):
```rust
pub(crate) payload_len: u32,
```

**Problem**: Raw `u32` without bounds checking at construction. The `MAX_PAYLOAD` constant is checked separately in `validate()`.

**Required NewType**:
```rust
pub(crate) struct PayloadLen(u32);
impl PayloadLen {
    pub(crate) const MAX: Self = PayloadLen(64 * 1024);
    pub(crate) fn is_within_bounds(&self) -> bool { self.0 <= Self::MAX.0 }
    pub(crate) fn as_usize(&self) -> Option<usize> { u32::try_from(self.0).ok() }
}
```

### F. Payload Digest - `[u8; 32]`

**Current** (line 79):
```rust
pub(crate) payload_digest: [u8; 32],
```

**Problem**: Raw BLAKE3 output bytes. No type safety to distinguish from other 32-byte hashes.

**Required NewType**:
```rust
pub(crate) struct PayloadDigest([u8; 32]);
impl PayloadDigest {
    pub(crate) fn compute(payload: &[u8]) -> Self {
        let digest = blake3::hash(payload);
        let mut out = [0u8; 32];
        out.copy_from_slice(digest.as_bytes());
        PayloadDigest(out)
    }
}
```

### G. Header CRC - `u32`

**Current** (line 81):
```rust
pub(crate) header_crc: u32,
```

**Problem**: Raw `u32` for CRC. No distinction from other CRCs or checksums.

**Required NewType**:
```rust
pub(crate) struct HeaderCrc(u32);
impl HeaderCrc {
    pub(crate) fn compute(header_bytes: &[u8]) -> Self {
        HeaderCrc(crc32fast::hash(header_bytes))
    }
    pub(crate) fn matches(&self, header_bytes: &[u8]) -> bool {
        self.0 == crc32fast::hash(header_bytes)
    }
}
```

### H. JSON UTF-8 Bytes - `Vec<u8>`

**Current** (line 48):
```rust
pub(crate) json_utf8: Vec<u8>,
```

**Problem**: Raw `Vec<u8>` for JSON content. Could contain invalid UTF-8 or non-JSON data.

**Required NewType**:
```rust
pub(crate) struct JsonUtf8Bytes(Vec<u8>);
impl JsonUtf8Bytes {
    pub(crate) fn new(bytes: Vec<u8>) -> Result<Self, PostcardError> {
        // Validate UTF-8
        std::str::from_utf8(&bytes).map_err(|_| PostcardError::JsonPayloadDecodeFailed)?;
        Ok(JsonUtf8Bytes(bytes))
    }
}
```

---

## Violation #3: Validation Scattered Across Multiple Functions

**Problem**: Validation logic is fragmented:

| Function | Lines | Responsibility |
|----------|-------|----------------|
| `PostcardHeader::validate()` | 93-104 | Magic, header_len, payload_len bounds |
| `validate_cli_payload()` | 191-202 | schema_version, kind, content_type |
| `validate_version_and_kind()` | 243-254 | Version bounds, kind check |
| `validate_header_crc()` | 227-241 | CRC validation |
| `decode_postcard()` | 269-300 | Orchestrates all validation |

**DDD Principle Violation**: Validation is "spread across the system" rather than being cohesive domain operations.

**Correct Pattern**: Single `PostcardHeader::validate()` should be the canonical validation method, accepting/rejecting the entire domain object in one place.

---

## Violation #4: PostcardError Enum Size

**Current**: 12 variants (lines 145-169)

```rust
pub(crate) enum PostcardError {
    InvalidMagic,           // Line 148
    InvalidHeaderLength,    // Line 150
    PayloadTooLarge,        // Line 152
    VersionTooOld,          // Line 154
    VersionTooNew,          // Line 156
    WrongKind,              // Line 158
    DigestMismatch,         // Line 160
    CrcMismatch,            // Line 162
    PayloadMetadataMismatch,// Line 164
    JsonPayloadDecodeFailed,// Line 166
    DecodeFailed,           // Line 168
}
```

**Problem**: 12 variants suggests this enum conflates multiple error categories:
- Header validation errors (4 variants)
- Payload validation errors (3 variants)
- Cryptographic errors (2 variants)
- Decoding errors (3 variants)

**DDD Principle**: Large error enums often indicate missing type refinement or multiple bounded contexts poorly combined.

**Suggested Split**:
```rust
// Header errors
pub(crate) enum HeaderError { InvalidMagic, InvalidHeaderLength, CrcMismatch }

// Payload errors
pub(crate) enum PayloadError { TooLarge, MetadataMismatch, DigestMismatch }

// Version errors  
pub(crate) enum VersionError { TooOld, TooNew, WrongKind }

// Codec errors
pub(crate) enum CodecError { JsonDecodeFailed, DecodeFailed }

// Top-level
pub(crate) enum PostcardError {
    Header(HeaderError),
    Payload(PayloadError),
    Version(VersionError),
    Codec(CodecError),
}
```

---

## Violation #5: `read_array` Helper at File Scope

**Current** (lines 138-142):
```rust
fn read_array<const N: usize>(data: &[u8], start: usize) -> Result<[u8; N], PostcardError> {
    let end = start.checked_add(N).ok_or(PostcardError::DecodeFailed)?;
    let bytes = data.get(start..end).ok_or(PostcardError::DecodeFailed)?;
    <[u8; N]>::try_from(bytes).map_err(|_| PostcardError::DecodeFailed)
}
```

**Problem**: This is a **partial function** - it panics on arithmetic overflow (`checked_add`) rather than returning an error that propagates cleanly. The `ok_or` pattern is correct but the function should be a method on a `ByteSlice` domain type.

**Required Refactor**: Move into a `ByteSlice` value object with bounds-checked reads.

---

## Violation #6: Inline Test Code (~200 lines)

**Problem**: 200 lines of tests are embedded in the production module, contributing to the line count violation.

**Required Action**: Move tests to `cli_postcard_test_vectors.rs` or `crates/workspace_tests/vb_cli_postcard_tests.rs`.

---

## Required Refactoring Actions

### Priority 1: Split the File
Create the following modules:
- [ ] `cli_postcard_types.rs` - NewTypes and error types
- [ ] `cli_postcard_codec.rs` - Encode/decode logic
- [ ] `cli_postcard_validation.rs` - Validation logic  
- [ ] `cli_postcard.rs` - Module glue (target: <50 lines)

### Priority 2: Introduce NewTypes
- [ ] `MagicBytes([u8; 4])`
- [ ] `SchemaVersion(u16)`
- [ ] `PostcardKind(u16)`
- [ ] `HeaderLen(u32)`
- [ ] `PayloadLen(u32)`
- [ ] `PayloadDigest([u8; 32])`
- [ ] `HeaderCrc(u32)`
- [ ] `JsonUtf8Bytes(Vec<u8>)`

### Priority 3: Consolidate Validation
- [ ] Merge `validate_version_and_kind()` into `PostcardHeader::validate()`
- [ ] Move `validate_cli_payload()` to `CliPostcardPayload::validate()`
- [ ] Move `validate_header_crc()` to `HeaderCrc::verify()`

### Priority 4: Extract Tests
- [ ] Move inline tests to `cli_postcard_test_vectors.rs`

---

## Architectural Health Score

| Dimension | Score | Notes |
|-----------|-------|-------|
| Line Count | 0/10 | 539 vs 300 limit |
| Primitive Obsession | 1/10 | 8+ raw primitives |
| Validation Cohesion | 3/10 | Scattered across 5 functions |
| Error Enum Design | 4/10 | 12 variants, should be nested |
| Test Isolation | 2/10 | ~200 lines inline |

**Overall: 2/10 - CRITICAL REFACTOR REQUIRED**

---

## Evidence

- File path: `/home/lewis/src/velvet-ballistics/crates/vb_cli/src/cli_postcard.rs`
- Line count: `wc -l` confirms 539 lines
- Contains 12 error variants, 8+ raw primitive fields, 200+ lines of inline tests
- Multiple validation functions that should be consolidated into domain objects

---

## Sign-off

**Enforcer**: architectural-drift  
**Status**: UNACCEPTABLE - MANDATORY REFACTOR  
**Next Action**: Agent must split file and introduce NewTypes before this code can be considered architecturally compliant.
