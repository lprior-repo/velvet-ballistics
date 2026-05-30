# Architectural Drift Report: `vb_storage/src/binary.rs`

**File:** `crates/vb_storage/src/binary.rs`
**Line Count:** 388 (VIOLATION: exceeds 300-line limit by 88 lines)
**Status:** REFACTOR REQUIRED

---

## Executive Summary

`binary.rs` is a 388-line utility module providing low-level byte-level read/write helpers for fixed-width record header fields. It violates the **<300 line rule** by 88 lines and exhibits **widespread primitive obsession** — raw `usize` offsets and bare `&[u8]` slices scatter magic layout constants across call sites instead of encoding the binary record layout in the type system.

---

## Violation 1: Line Count (388 > 300)

The file must be split into at least two modules. The production code (lines 1–75) is ~75 lines; the test module (lines 77–388) is **311 lines** — nearly the entire budget consumed by tests alone.

### Required Split

| Module | Responsibility | Target Lines |
|--------|---------------|--------------|
| `binary.rs` | Core read/write primitives | ≤100 |
| `binary/tests.rs` OR `binary/tests_roundtrip.rs` | All roundtrip + edge-case tests | ≤200 |
| `record_layout.rs` (NEW) | RecordHeader domain type wrapping offset arithmetic | ≤150 |

---

## Violation 2: Primitive Obsession — Raw Offsets

### Problem

Every function signature uses `offset: usize` as a raw integer parameter:

```rust
pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, JournalError>
pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, JournalError>
pub(crate) fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, JournalError>
pub(crate) fn write_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), JournalError>
// ... etc
```

The **magic layout constants** `CRC_OFFSET = 56` and `DIGEST_BYTES = 32` leak out of the type system entirely. Call sites like `kani_postcard_envelope_wire.rs` hardcode `24` and `CRC_OFFSET` directly in slice operations:

```rust
// kani_postcard_envelope_wire.rs:294
header[24..24 + DIGEST_BYTES].copy_from_slice(correct_digest.as_bytes());
// kani_postcard_envelope_wire.rs:55-56
header[CRC_OFFSET..CRC_OFFSET.saturating_add(4)].copy_from_slice(&crc.to_le_bytes());
```

This is **parse, then validate** (at runtime, with fallible operations) rather than **make illegal states unrepresentable**.

### Scott Wlaschin DDD Violations

1. **Primitive Obsession**: `offset: usize` is a primitive; should be `HeaderFieldOffset<N>` (a newtype on `usize` with phantom `N` for width).
2. **Constants not typed**: `CRC_OFFSET` and `DIGEST_BYTES` are raw `usize` values used across 386 matches in the codebase. They should be embedded in a `RecordHeaderLayout` trait or `HeaderLayout` associated-type struct.
3. **No domain type for the binary record**: The file exposes 7 standalone functions that manipulate "record headers" but there is no `RecordHeader` type to represent the domain concept. Callers manually compute offsets, lengths, and boundaries — errors are silently possible.

### Evidence of Tight Coupling

Search results show `CRC_OFFSET` and `DIGEST_BYTES` appear in **386 locations** across the codebase. Every call site that writes a digest or CRC is doing manual slice arithmetic instead of delegating to a typed `RecordHeader` API.

---

## Violation 3: `write_digest` Contains a Magic Number

```rust
pub(crate) fn write_digest(
    bytes: &mut [u8],
    digest: &[u8; DIGEST_BYTES],
) -> Result<(), JournalError> {
    let target = bytes
        .get_mut(24..CRC_OFFSET)  // <-- magic number 24 hardcoded
        .ok_or(JournalError::UnexpectedEof)?;
    target.copy_from_slice(digest);
    Ok(())
}
```

The `24` is the **start of the digest field** in the record header. This is **not derived from any type or constant** — it's just a number. The actual header layout is:

| Field | Offset | Width |
|-------|--------|-------|
| (reserved) | 0 | 24 |
| digest | 24 | 32 |
| crc32c | 56 | 4 |

The magic `24` should be `const DIGEST_START: usize = 24;` — and more importantly, the entire header layout should be a single `RecordHeaderLayout` const struct, not scattered constants.

---

## Violation 4: No Parse, Don't Validate — All Runtime Bounds Checking

All six primitives (`read_u16/32/64`, `write_u16/32/64`) use the same error-prone pattern:

```rust
let end = offset.checked_add(WIDTH).ok_or(JournalError::UnexpectedEof)?;
let slice = bytes.get(offset..end).ok_or(JournalError::UnexpectedEof)?;
```

This is **validate after parse** — the function first attempts the read, then reports `UnexpectedEof` if it fails. For a binary record system, the correct approach is to **ensure at compile time** (via the type system or const generics) that buffers are large enough for the expected layout.

Compare to a proper `RecordHeader<Buf>` newtype that knows its own layout and exposes typed accessors:

```rust
impl RecordHeader<60> {
    pub fn digest(&self) -> &[u8; 32] { /* ... */ }
    pub fn crc32(&self) -> u32 { /* ... */ }
}
```

---

## Violation 5: No State Representation

The file describes a **binary record envelope** — a fixed-width header with typed fields (magic, version, digest, crc). This is a perfect candidate for a Scott Wlaschin **typed state** model where the header is a single immutable value object that parses or fails (rather than a bag of 7 standalone functions).

Currently there is no `RecordHeader` type. The functions are all `pub(crate)` but entirely disconnected — they don't share a type that represents "a parsed record header."

---

## Refactoring Prescription

### Step 1: Extract `record_layout.rs` (NEW)

Create a new module defining:

```rust
/// Compile-time record header layout constants.
pub const RECORD_HEADER_LAYOUT: HeaderLayout = HeaderLayout {
    total_bytes: 60,
    digest_offset: 24,
    digest_bytes: 32,
    crc_offset: 56,
};

pub struct HeaderLayout {
    pub total_bytes: usize,
    pub digest_offset: usize,
    pub digest_bytes: usize,
    pub crc_offset: usize,
}

/// Newtype wrapping a 60-byte record header.
#[repr(transparent)]
pub struct RecordHeader<T: AsRef<[u8]>>(T);

impl RecordHeader<[u8; 60]> {
    /// Parse a raw 60-byte header slice into a RecordHeader.
    pub fn from_bytes(bytes: &[u8; 60]) -> Result<Self, JournalError> { ... }
    pub fn digest(&self) -> &[u8; 32] { ... }
    pub fn crc32(&self) -> u32 { ... }
    // etc.
}
```

### Step 2: Reduce `binary.rs` to ~75 lines

Move all `#[test]` blocks to `binary/tests_roundtrip.rs`. Reduce each read/write pair to a single generic function if possible:

```rust
pub(crate) fn read_uN<T: ByteOrder + Width>(bytes: &[u8], offset: usize) -> Result<T, JournalError>
pub(crate) fn write_uN<T: ByteOrder + Width>(bytes: &mut [u8], offset: usize, value: T) -> Result<(), JournalError>
```

Or keep them as-is but extract `write_digest` to use `HeaderLayout::DIGEST_OFFSET` instead of bare `24`.

### Step 3: Remove Magic Numbers

Replace `24` in `write_digest` with a named constant derived from `HeaderLayout`.

---

## Summary Table

| Issue | Severity | Scott Wlaschin Principle Violated |
|-------|----------|----------------------------------|
| 388 lines (> 300) | CRITICAL | Structuring — file too large |
| `offset: usize` everywhere | HIGH | Primitive obsession — no type-safe offsets |
| Magic number `24` in `write_digest` | HIGH | Primitive obsession — no `HeaderLayout` type |
| 386 scatter sites for `CRC_OFFSET`/`DIGEST_BYTES` | HIGH | Primitive obsession — constants not encapsulated |
| All functions are validate-after-parse | MEDIUM | Parse, don't validate |
| No `RecordHeader` domain type | MEDIUM | Types represent domain concepts |

---

## Recommendation

**REFACTOR.** Split tests into a separate module. Create `record_layout.rs` with a typed `RecordHeader` domain type. Replace all standalone offset arithmetic with methods on `RecordHeader`. This will eliminate the primitive obsession at the binary layout level and make the 386 call sites across `vb_storage` more type-safe.

**Priority**: HIGH — the 388-line count is a hard violation, and the primitive obsession in offset handling is a systemic issue touching every record read/write in the storage crate.
