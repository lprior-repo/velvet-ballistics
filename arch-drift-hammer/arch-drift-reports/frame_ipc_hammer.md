# Architectural Drift Report: `vb_ipc/src/frame.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_ipc/src/frame.rs`  
**Total Lines:** 1540  
**Classification:** PROTOCOL ENCODING/DECODING LAYER  
**Date:** 2026-05-29  
**Drift Agent:** architectural-drift

---

## 1. File Overview

This file implements IPC frame encoding, decoding, validation, and I/O utilities for the vb_ipc crate. It operates at the wire-format level, handling byte-level frame construction and parsing.

### Frame Layout (Wire Format)

```
Offset  Size  Field
------  ----  -----
0       4     MAGIC (VBLT in LE)
4       2     VERSION (1)
6       2     COMMAND (u16)
8       2     FLAGS (u16)
10      2     RESERVED (must be 0)
12      8     CORRELATION (u64)
20      4     PAYLOAD_LEN (u32)
-------------------
24      N     PAYLOAD (postcard bytes)
```

Total header size: **24 bytes (IPC_HEADER_LEN)**

---

## 2. Frame Types and Operations

### 2.1 Types

| Type | Location | Primitive Fields |
|------|----------|------------------|
| `IpcFrameHeader` | frame_types.rs:15 | `command`, `flags: u16`, `correlation: u64`, `payload_len: u32` |
| `IpcFrame` | frame_types.rs:125 | `header: IpcFrameHeader`, `payload: BoundedPayload` |
| `BoundedPayload` | bounded.rs:48 | `Bytes` |

### 2.2 Public Functions in frame.rs

| Function | Signature | Primitive Params |
|----------|-----------|-----------------|
| `encode_frame` | `(command, flags: u16, correlation: u64, payload: &[u8])` | flags, correlation |
| `decode_frame_header` | `(&[u8; 24])` | — |
| `decode_frame_payload` | `(header, payload: &[u8])` | — |
| `validate_frame_magic` | `(bytes: &[u8])` | — |
| `validate_frame_bounds` | `(header, max_payload)` | — |
| `read_frame_header` | `<R: Read>(reader)` | — |
| `read_frame_header_bounded` | `<R: Read>(reader, max_payload)` | — |
| `read_frame_payload` | `<R: Read>(reader, header)` | — |
| `read_frame_payload_bounded` | `<R: Read>(reader, header, max_payload)` | — |
| `write_frame` | `<W: Write>(writer, command, flags: u16, correlation: u64, payload: &[u8])` | flags, correlation |

---

## 3. Primitive Obsession Violations

### VIOLATION 1: `flags: u16` — No Semantic Type

**Locations:**
- `frame.rs:13` — `encode_frame(..., flags: u16, ...)`
- `frame.rs:139` — `write_frame(..., flags: u16, ...)`
- `frame_types.rs:19` — `IpcFrameHeader { flags: u16, ... }`
- `frame_types.rs:29` — `IpcFrameHeader::new(..., flags: u16, ...)`

**Problem:** `u16` carries no semantic meaning. Flags are bitfields with named meanings, yet callers pass raw `u16` values. The `as_u16()` on `IpcCommand` works because command is a closed enum, but flags have no such safety.

**Evidence:**
```rust
// frame.rs:231 — raw hex flag in test
encode_frame(IpcCommand::Health, 0x1234, 99, payload)
```

**Remediation:** Introduce `IpcFlags(u16)` wrapper with named flag constants:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcFlags(u16);

impl IpcFlags {
    pub const NONE: Self = Self(0);
    pub const PRIORITY: Self = Self(1 << 0);
    pub const NO_REPLY: Self = Self(1 << 1);
    // etc.
}
```

---

### VIOLATION 2: `correlation: u64` — No CorrelationId Type

**Locations:**
- `frame.rs:13` — `encode_frame(..., correlation: u64, ...)`
- `frame.rs:140` — `write_frame(..., correlation: u64, ...)`
- `frame_types.rs:21` — `IpcFrameHeader { correlation: u64, ... }`
- `frame_types.rs:29` — `IpcFrameHeader::new(..., correlation: u64, ...)`

**Problem:** Correlation IDs are opaque tokens used to match requests with responses. They have no arithmetic meaning, yet raw `u64` implies numeric operations. No type distinguishes correlation IDs from other u64 values.

**Evidence:**
```rust
// frame.rs:172 — test passes raw u64
encode_frame(IpcCommand::Health, 0, 7, b"")
// frame.rs:731 — u64::MAX correlation
let corr = u64::MAX;
encode_frame(IpcCommand::Health, 0, corr, b"")
```

**Remediation:** Introduce `CorrelationId(u64)` wrapper:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CorrelationId(u64);

impl CorrelationId {
    pub const ZERO: Self = Self(0);
    pub const fn new(v: u64) -> Self { Self(v) }
    pub const fn get(self) -> u64 { self.0 }
}
```

---

### VIOLATION 3: `payload: &[u8]` — No Payload Type

**Locations:**
- `frame.rs:14` — `encode_frame(..., payload: &[u8], ...)`
- `frame.rs:141` — `write_frame(..., payload: &[u8], ...)`
- `frame.rs:37` — `decode_frame_payload(..., payload: &[u8])`

**Problem:** Raw byte slices have no type distinction between "raw bytes," "postcard payload," and "header bytes." A caller could accidentally swap byte sources.

**Evidence:**
```rust
// frame.rs:192 — test passes b"" as payload
encode_frame(command, 0, 42, b"test")
```

**Remediation:** Introduce `IpcPayloadBytes<'a>(&'a [u8])` or use `BoundedPayload` more broadly in the public API.

---

### VIOLATION 4: `[u8; IPC_HEADER_LEN]` — Raw Byte Array Construction

**Location:**
- `frame.rs:89` — `let mut header_bytes = [0u8; IPC_HEADER_LEN];`

**Problem:** The fixed-size header array is constructed as raw bytes with no field-level construction. Each field is written via `Cursor` rather than built as a structured type first.

**Remediation:** Consider `IpcFrameHeader` having a `as_bytes()` method that calls `encode()`, and decode returning a fully-validated struct. The current design does have `encode()`/`decode()` on `IpcFrameHeader`, but public functions like `read_frame_header` expose raw `[u8; IPC_HEADER_LEN]` arrays.

---

### VIOLATION 5: `payload_len: u32` in Header — No Semantic Wrapper

**Location:**
- `frame_types.rs:23` — `IpcFrameHeader { payload_len: u32, ... }`

**Problem:** While `bounded.rs` has `MaxPayloadBytes` wrapper, the decoded header's `payload_len` is a raw `u32`. This field has range constraints (0 to max_payload) but no type enforces them.

**Note:** `MaxPayloadBytes` is a good example of the wrapper pattern that should be applied to `flags` and `correlation` as well.

---

## 4. Summary of Required Type Wrappers

| Current Type | Proposed Wrapper | Purpose |
|--------------|------------------|---------|
| `u16` (flags) | `IpcFlags` | Named flag constants, bitfield safety |
| `u64` (correlation) | `CorrelationId` | Type distinctness from other u64 values |
| `&[u8]` (payload) | `IpcPayloadBytes<'a>` | Semantic distinction from other byte sources |
| `u32` (payload_len in header) | `PayloadLen` or reuse `bounded.rs` approach | Range validation at construction |

---

## 5. Architectural Boundary Assessment

**Layer:** Wire protocol encoding/decoding  
**Expected Character:** This layer SHOULD be thin byte translation.  
**Current Assessment:** PARTIALLY COMPLIANT — the `IpcCommand` enum is well-typed, `bounded.rs` has proper wrappers, but the public frame API leaks raw primitives.

**DDD Cohesion:** LOW — functions operate on primitives without domain types. The `IpcFrameHeader` struct exists but its fields are raw primitives.

---

## 6. Recommendations

1. **Immediate (Low Effort):**
   - Add `IpcFlags` wrapper with `pub const` flag constants
   - Add `CorrelationId` wrapper
   - Add deprecation notes to raw-primitive parameters

2. **Short Term (Medium Effort):**
   - Replace `flags: u16` and `correlation: u64` in function signatures with wrapper types
   - Update all call sites (tests show usage patterns)
   - Add `PayloadBytes<'a>` wrapper for payload slices

3. **Long Term (Higher Effort):**
   - Consider a builder pattern for `IpcFrameHeader` construction
   - Consider `TryFrom<&[u8]>` implementations for validated byte slices

---

## 7. Test Coverage Assessment

**Strengths:**
- Excellent adversarial test coverage (300+ lines of adversarial tests)
- Tests cover magic validation, version checking, command validation, length bounds
- Tests include edge cases: u16::MAX, u64::MAX, u32::MAX payloads, all-zero/FF headers

**Weaknesses:**
- Tests use raw primitives, so they will need updating when wrapper types are introduced

---

## 8. Verdict

**DRIFT SEVERITY: MODERATE**

`frame.rs` is 1540 lines of well-tested code, but it suffers from pervasive primitive obsession. The `IpcCommand` enum and `bounded.rs` types show the codebase understands the wrapper pattern — it just wasn't applied consistently to `flags` and `correlation`.

The protocol is functionally correct and well-tested. The drift is in type safety, not behavior.

**Estimated Refactor Size:** ~200 lines of new types + signature updates across vb_ipc crate.
