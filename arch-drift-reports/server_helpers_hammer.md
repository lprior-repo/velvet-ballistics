# Architectural Drift Report: `vb_ipc/src/server/helpers.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_ipc/src/server/helpers.rs`
**Status**: `VIOLATION`  
**Line Count**: 749 (exceeds 300-line limit by **149%**)

---

## Executive Summary

This file contains **6 public helper functions** and **52 test cases** (lines 209–749 are tests). The helpers handle buffer management, frame parsing, and response encoding for the IPC server. The file suffers from:

1. **LINE COUNT VIOLATION** — 749 lines, 2.5× over the limit
2. **Primitive Obsession** — 5 instances of raw `usize`/`u32`/`[u8; N]` where types should be introduced
3. **God Function** — `send_response` is 86 lines with branching IO handling that conflates encoding, writing, and polling
4. **Mixed Concerns** — test hooks embedded in the helpers module at lines 95–103

---

## Line Count Breakdown

| Section | Lines | Issue |
|---------|-------|-------|
| `append_read_bytes` | 25 | OK |
| `read_buffer_header` | 6 | OK |
| `frame_total_len` | 12 | OK |
| `extract_payload` | 13 | OK |
| `frame_error_response` | 5 | OK |
| `borrow_workflow_resolver` | 9 | OK |
| `send_response` | 86 | **TOO LONG** |
| `test_hooks` submodule | 9 | Mixed concern |
| `append_read_bytes_checked_add` | 3 | OK |
| `frame_total_len_checked_add` | 8 | OK |
| **Tests** | **540** | **TOO LONG** — must be split into `helpers/tests.rs` |

---

## Primitive Obsession Violations

### VP-1: Hardcoded `4096` Buffer Size
**Location**: Line 17, `append_read_bytes`

```rust
pub fn append_read_bytes(
    read_buffer: &mut Vec<u8>,
    temp_buf: &[u8; 4096],   // ← PRIMITIVE OBSESSION
    bytes_read: usize,
) -> Result<(), IpcServerError>
```

`4096` appears as a raw literal in the type signature. This is a **domain constant** — the standard read chunk size for socket IO — that should be:

```rust
// In vb_ipc/src/constants.rs or new buffer module:
pub const SOCKET_READ_CHUNK_SIZE: usize = 4096;

// Or as a newtype:
#[repr(transparent)]
pub struct TempReadBuf([u8; SOCKET_READ_CHUNK_SIZE]);
```

**Refactor**: Extract `SOCKET_READ_CHUNK_SIZE` to `vb_ipc/src/constants.rs` and rename the parameter to `temp_buf: &TempReadBuf`.

---

### VP-2: Raw `usize` for Byte Counts
**Location**: Line 18, `append_read_bytes` and throughout

`bytes_read: usize` and `next_len: usize` are unconstrained `usize` values. Scott Wlaschin's "Make illegal states unrepresentable" principle demands these be wrapped in **value objects**:

```rust
/// Number of bytes read from a socket in a single poll turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BytesRead(usize);

/// Total read buffer byte length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferLen(usize);
```

**Locations affected**:
- Line 18: `bytes_read: usize`
- Line 28: `next_len` 
- Line 30: `.checked_add(read_slice.len())`
- Line 65: `extract_payload(read_buffer: &mut Vec<u8>, total_len: usize)`
- Line 68: `total_len: usize`

---

### VP-3: Raw `u32` for `payload_len`
**Location**: `frame_total_len`, line 52

```rust
pub fn frame_total_len(header: &IpcFrameHeader) -> Result<usize, IpcServerError> {
    let payload_len = usize::try_from(header.payload_len)  // u32 → usize conversion
```

`header.payload_len` is `u32`. The conversion from `u32` to `usize` is performed inline without a named domain function. This should be a method on `IpcFrameHeader`:

```rust
impl IpcFrameHeader {
    /// Returns payload length as `usize`, failing if it cannot fit.
    pub fn payload_len_usize(self) -> Result<usize, IpcServerError> { ... }
}
```

Note: `u32_to_usize` already exists in `error.rs` (line 161), but it's not used here.

---

### VP-4: Raw `[u8; IPC_HEADER_LEN]` in Return Type
**Location**: Line 43, `read_buffer_header`

```rust
pub fn read_buffer_header(read_buffer: &[u8]) -> Result<[u8; IPC_HEADER_LEN], IpcServerError>
```

This returns a raw byte array. While `IPC_HEADER_LEN` is a constant, the array itself is untyped. Consider a newtype:

```rust
/// Encoded IPC frame header wire bytes.
#[repr(transparent)]
pub struct EncodedHeader([u8; IPC_HEADER_LEN]);
```

---

### VP-5: Unconstrained `Token` from Mio
**Location**: Line 110, `send_response`

```rust
pub fn send_response(
    stream: &mut UnixStream,
    write_buffer: &mut Vec<u8>,
    registry: &Registry,
    token: Token,  // ← Raw mio Token, which is just usize
```

`mio::Token` is a newtype over `usize`, but in this context it represents a **client connection identifier**. The `usize` inside has different semantics than a random `usize`. A wrapper would document intent:

```rust
/// Client connection token from mio poll registration.
pub struct ClientToken(mio::Token);
```

---

## God Function: `send_response` (86 lines)

**Lines 106–191** handle 4 distinct concerns:

1. **Payload encoding** (postcard serialization) — lines 114–124
2. **Header construction** — lines 129–134
3. **Header encoding** — lines 136–147
4. **Buffer write to socket** — lines 149–161
5. **Flush or reregister** — lines 162–188

This conflates the **encode → write → poll** state machine into one function. It should be split:

```
send_response
├── encode_response_frame  (payload + header → wire bytes)
├── write_frame_to_socket (write to stream, handle WouldBlock)
└── flush_or_reregister   (flush or re-register interest)
```

---

## Test Module Must Be Split (540 lines)

**Lines 209–749** are tests for the helper functions. Per the workspace structure rule:

> Never place production code, tests, or benchmarks at the repository root.

Tests embedded at the bottom of a 749-line source file are at the **module level**, not the crate level. They should be moved to `crates/vb_ipc/src/server/helpers/tests.rs` or `crates/vb_ipc/tests/helpers_tests.rs`.

---

## Findings Summary

| ID | Category | Severity | Description |
|----|----------|----------|-------------|
| LC-1 | Line Count | **CRITICAL** | 749 lines exceeds 300-line limit by 149% |
| VP-1 | Primitive Obsession | **HIGH** | Hardcoded `4096` buffer size in type signature |
| VP-2 | Primitive Obsession | **HIGH** | Raw `usize` for byte counts without value objects |
| VP-3 | Primitive Obsession | **MEDIUM** | `u32 → usize` conversion inline instead of method |
| VP-4 | Primitive Obsession | **LOW** | Raw `[u8; IPC_HEADER_LEN]` return type |
| VP-5 | Primitive Obsession | **LOW** | Unconstrained `Token` wrapper missing |
| GF-1 | God Function | **HIGH** | `send_response` is 86 lines, conflates 4 concerns |
| TM-1 | Test Location | **MEDIUM** | 540 lines of tests at module level |

---

## Recommended Refactor Map

```
helpers.rs (749 lines)  →  helpers.rs (~150 lines)
                        →  helpers/buffer.rs (~120 lines)   [append_read_bytes, extract_payload]
                        →  helpers/encoding.rs (~100 lines)  [send_response frame encoding]
                        →  helpers/tests.rs (~540 lines)     [all tests]
```

New types to introduce:
- `TempReadBuf` — `#[repr(transparent)] pub struct TempReadBuf([u8; 4096]);`
- `BytesRead(usize)` — value object for socket read counts  
- `BufferLen(usize)` — value object for buffer sizes
- `EncodedHeader([u8; IPC_HEADER_LEN])` — header wire bytes
- `ClientToken(mio::Token)` — connection identifier

---

## Verification

After refactoring:
- [ ] All resulting `.rs` files ≤ 300 lines
- [ ] All raw `usize` byte counts wrapped in named types
- [ ] `send_response` split into ≤ 3 focused functions
- [ ] Tests moved to separate file
- [ ] `SOCKET_READ_CHUNK_SIZE` extracted to `constants.rs`
