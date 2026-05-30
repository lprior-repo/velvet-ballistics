# ARCHITECTURAL DRIFT REPORT: `frame_types.rs`

**File:** `crates/vb_ipc/src/frame_types.rs`
**Status:** ATROCITY — 301 lines (1 line over limit)
**Reviewer:** arch-drift-hammer
**Date:** 2026-05-29

---

## EXECUTIVE SUMMARY

This file barely clears the 300-line threshold but embodies everything wrong with primitive-obsessed IPC design. Every wire-format primitive is left untyped. The word "frame" appears 47 times but there is not a single value object to show for it. The `encode`/`decode` methods are not testable in isolation because they are coupled to concrete byte layouts instead of abstracted behind a `WireCodec` trait.

---

## 1. RESPONSIBILITY MAP

| Symbol | Type | Responsibility | Lines |
|--------|------|----------------|-------|
| `IpcFrameHeader` | struct | Binary IPC header (24 bytes: magic + version + command + flags + reserved + correlation + payload_len) | 14–121 |
| `IpcFrameHeader::encode` | method | Serializes header to `[u8; IPC_HEADER_LEN]` little-endian wire format | 39–64 |
| `IpcFrameHeader::decode` | method | Deserializes + validates header from `[u8; IPC_HEADER_LEN]` | 67–120 |
| `IpcFrame` | struct | Complete IPC frame = header + bounded payload | 124–128 |
| `IpcFrame::new` | method | Validates header/payload length agreement, wraps payload in `BoundedPayload` | 132–150 |
| `IpcFrame::header` | accessor | Returns cloned header | 154–156 |
| `IpcFrame::payload` | accessor | Returns `&BoundedPayload` | 160–162 |
| `decode_frame` | free function | Top-level convenience: decode header then build frame | 166–176 |
| `tests` | module | 10 unit tests covering decode rejection paths and accessor behavior | 178–301 |

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 `flags: u16` — Untyped Bitfield

```rust
pub struct IpcFrameHeader {
    pub flags: u16,   // ← raw u16, no type safety
```

**Problem:** `flags` is a 16-bit bitmask but has no `Flags` wrapper. Any `u16` value is accepted as valid flags. There is no way to enforce read-only vs. mutable flags, no way to name individual bits, and no compile-time guarantee that flag combinations are valid.

**DDD Fix:** Introduce a `IpcFrameFlags` newtype with named constants:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcFrameFlags(u16);

impl IpcFrameFlags {
    pub const NONE: Self = Self(0);
    pub const COMPRESSED: Self = Self(1 << 0);
    pub const PRIORITY: Self = Self(1 << 1);
    // etc.
}
```

### 2.2 `correlation: u64` — Raw Correlation ID

```rust
pub correlation: u64,   // ← raw u64, no CorrelationId wrapper
```

**Problem:** Correlation IDs are a domain concept (request-reply matching) but are stored as an undifferentiated `u64`. No type distinguishes a correlation ID from a generic sequence number elsewhere in the codebase.

**DDD Fix:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CorrelationId(u64);
```

### 2.3 `payload_len: u32` — Raw Byte Count

```rust
pub payload_len: u32,   // ← raw u32, should be PayloadLen
```

**Problem:** `u32` can represent any value 0..=4GiB. The domain concept "postcard payload byte length" is a `u32` with additional constraints (bounded by `MaxPayloadBytes`). This is validated at decode time, but the `IpcFrameHeader` struct itself allows constructing arbitrary `u32` values.

**DDD Fix:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadLen(u32);

impl PayloadLen {
    pub fn new(raw: u32) -> Self;
    pub fn get(self) -> u32;
}
```

### 2.4 Reserved Field — Magic Number Discipline

```rust
let reserved = cursor.read_u16::<LittleEndian>()...;
if reserved != 0 {
    return Err(IpcError::ReservedNonZero { actual: reserved });
}
```

**Problem:** The reserved field is a `u16` with hardcoded semantic that it "must be zero." This constraint is enforced at runtime but is not expressed in the type system. A `Reserved<u16, 0>` phantom marker type would make the constraint compile-time-checkable.

### 2.5 Magic and Version — Untyped Wire Constants

```rust
cursor.write_u32::<LittleEndian>(IPC_MAGIC)...;
cursor.write_u16::<LittleEndian>(IPC_VERSION)...;
```

**Problem:** `IPC_MAGIC` and `IPC_VERSION` are `const u32`/`const u16` but should be typed `WireMagic` and `WireVersion` newtypes to prevent mixing them up in byteorder calls.

---

## 3. ANEMIC DOMAIN MODEL — ENCODE/DECODE COUPLING

### 3.1 Wire Format Bleeds Into Domain

`IpcFrameHeader::encode` and `IpcFrameHeader::decode` are concrete methods that write/read raw bytes. They mix wire format concerns (little-endian byte order, fixed layout) with the domain struct. This makes it impossible to:
- Test wire format independently of the domain struct
- Swap wire formats (e.g., big-endian for cross-platform)
- Mock the codec in tests

### 3.2 No `WireCodec` Trait

A proper DDD design would define:
```rust
pub trait WireCodec<T> {
    fn encode(t: T) -> Result<Vec<u8>, IpcError>;
    fn decode(bytes: &[u8]) -> Result<T, IpcError>;
}
```

This file has zero trait definitions and zero abstraction over encoding.

### 3.3 `decode_frame` Free Function

```rust
pub fn decode_frame(...) -> Result<IpcFrame, IpcError> { ... }
```

This standalone function (not an impl block method) owns the composition of `IpcFrameHeader::decode` + `IpcFrame::new`. In DDD terms, this is a **application service** leaking into the domain layer.

---

## 4. TEST MODULE OBSERVATIONS

The 123-line test module (lines 178–301) is 40% of the file. It is well-structured but does not test the newtypes, flags, or correlation ID behavior — only the header encode/decode roundtrip. This is a symptom of the anemic model: there is no domain behavior to test beyond byte shuffling.

---

## 5. SUMMARY SCORECARD

| Criterion | Status | Notes |
|-----------|--------|-------|
| Line count | 🔴 FAIL | 301 / 300 (+1) |
| Primitive obsession | 🔴 FAIL | 3+ untyped wire primitives (`flags`, `correlation`, `payload_len`) |
| Value objects | 🔴 FAIL | Zero newtypes for domain concepts |
| WireCodec abstraction | 🔴 FAIL | Encode/decode hardcoded to concrete types |
| Reserved field typing | 🔴 FAIL | Raw `u16` with runtime-only constraint |
| Reserved field marker type | 🔴 FAIL | No `Reserved<T, N>` phantom type |
| Flags type | 🔴 FAIL | No `IpcFrameFlags` bitflags struct |
| CorrelationId type | 🔴 FAIL | No `CorrelationId(u64)` newtype |
| PayloadLen type | 🔴 FAIL | No `PayloadLen(u32)` newtype |
| Decode as impl method | ⚠️ DEBATABLE | `decode_frame` free function is questionable DDD |

---

## 6. PRESCRIPTION

### Phase 1: Extract Value Objects (do not touch encode/decode yet)
- Create `CorrelationId(u64)` newtype
- Create `PayloadLen(u32)` newtype  
- Create `IpcFrameFlags(u16)` with `bitflags!` or manual consts
- Replace raw fields in `IpcFrameHeader`

### Phase 2: WireCodec Trait
- Define `WireCodec<IpcFrameHeader>` trait
- Move `encode`/`decode` to a `IpcFrameHeaderWire` impl of the trait
- This enables injectable/mockable codecs for testing

### Phase 3: Reserved Marker Type
- Introduce `struct Reserved<T, const N: u64>(T)` phantom type
- Replace reserved `u16` field

### Phase 4: Line Count
- After extraction, push the new value object files into `frame_types/` module directory
- Original file should drop to ~180 lines

---

## 7. VERIFICATION COMMANDS

After refactoring:
```bash
cd /home/lewis/src/velvet-ballistics
wc -l crates/vb_ipc/src/frame_types.rs          # must be ≤ 300
cargo check -p vb_ipc                           # must compile
cargo test -p vb_ipc                            # must pass
cargo clippy -p vb_ipc -- -D warnings           # zero warnings
```
