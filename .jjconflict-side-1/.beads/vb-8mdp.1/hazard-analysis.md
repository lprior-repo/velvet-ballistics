# VB IPC Hazard Analysis — Fragmented-Frame and Oversize-Message Tests

## Hazard Categories

### H1: Oversize Allocation (Bounded State Violation)

**Description:** A malicious or misbehaving client declares a `payload_len` that exceeds
`MaxPayloadBytes` but is small enough to pass the initial socket read, causing the server
to attempt `Vec::with_capacity(plen)` before validation.

**Current mitigation:** `IpcFrameHeader::decode` performs the `PayloadTooLarge` check
on the raw u32 bytes before any allocation. The server never calls
`Vec::with_capacity(payload_len)` — it only appends actual bytes read.

**Code path:**
```
client sends header with payload_len = 1 GiB
server: read_buffer gets only header bytes (24 bytes)
IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT)
  → Err(PayloadTooLarge { actual: 1073741824, limit: 1048576 })
server sends error, disconnects
```

**Residual risk:** If a future refactor adds `Vec::with_capacity(header.payload_len)`
before the bounds check, this would be exploitable.

**Risk tag:** `[bounded-state] [no-panic] [IPC]`

---

### H2: Partial Frame Denial-of-Service

**Description:** A slow client sends partial frames byte-by-byte, causing the server's
`read_buffer` to grow unboundedly while the header is never complete enough to decode.

**Current mitigation:**
- Server only appends `READ_CHUNK_BYTES` (4096) per poll event
- Header decode is attempted only when `read_buffer.len() >= IPC_HEADER_LEN`
- A slow client with only partial header bytes accumulates at most 4096*N bytes
  where N is the number of polls without completing the header
- Client is disconnected on any header decode error (including timeout-triggered scenarios)

**Code path:**
```
client sends 1 byte per poll event (malicious slow-pull)
server: appends 4096 bytes max per poll
header is never complete → header decode never attempted
eventual timeout or resource limit would trigger cleanup
```

**Residual risk:** If `READ_CHUNK_BYTES` were increased or the header-complete check
were bypassed, partial frames could accumulate beyond intended bounds.

**Risk tag:** `[bounded-state] [DoS] [IPC]`

---

### H3: Magic Injection / Protocol Confusion

**Description:** An attacker sends bytes that start with `0x5642_4C54` (valid magic)
to confuse the server about the protocol version or to inject frames into a stream.

**Current mitigation:**
- `InvalidMagic` is checked FIRST before any other field
- `UnsupportedVersion` is checked second, rejecting non-v1 frames
- `UnknownCommand` rejects wire IDs outside 1..16
- `ReservedNonZero` catches non-standard header variants
- All of these occur before `payload_len` is interpreted

**Code path:**
```
attacker sends: 0x5642_4C54 [0xFF 0xFF ...] (valid magic, garbage rest)
IpcFrameHeader::decode → UnsupportedVersion { actual: 0xFFFF } OR UnknownCommand
server sends error, disconnects
```

**Residual risk:** Low. The strict decode order ensures magic is the first check.

**Risk tag:** `[protocol-boundary] [IPC] [magic-validation]`

---

### H4: Postcard Payload Decode Panic

**Description:** Malformed Postcard bytes in the payload section cause `postcard::from_bytes`
to panic (e.g., invalid enum discriminant, out-of-range value for constrained types).

**Current mitigation:**
- `decode_frame_payload` wraps postcard error in `PayloadDecodeFailed`
- `dispatch_command_with_resolver` handles this error and sends an `IpcResponse::PayloadError`
- The server does NOT panic; the client is NOT automatically disconnected on payload decode failure

**Code path:**
```
malicious payload: valid header + garbage Postcard bytes
server: header decode OK → payload extracted → Postcard fails
dispatch returns PayloadError response
client receives error, connection remains open
```

**Residual risk:** If Postcard itself contains unsafe code that panics on malformed input,
this could escalate. Postcard is a safe crate.

**Risk tag:** `[codec-boundary] [IPC] [no-panic]`

---

### H5: Reserved Field Non-Zero Bypass

**Description:** A client sends a frame with a non-zero reserved field (bytes 10..12) to
exploit future protocol extensions or cause divergent behavior.

**Current mitigation:**
- `ReservedNonZero` is checked after command and before payload_len
- This is the 4th check in decode order, ensuring protocol compliance before payload processing

**Risk tag:** `[protocol-boundary] [IPC]`

---

### H6: Truncated Payload (UnexpectedEOF)

**Description:** A client sends a complete header declaring `payload_len = N` but closes
the connection or stops sending after fewer than N bytes.

**Current mitigation:**
- `read_frame_payload` uses `reader.read_exact()` which returns `Err(IpcError::PayloadDecodeFailed)`
  when fewer bytes are available than declared
- The server disconnects the client on `PayloadDecodeFailed`
- No partial allocation occurs

**Code path:**
```
header declares payload_len = 1000
client sends 500 bytes then disconnects
server: read_exact → Err(PayloadDecodeFailed)
server disconnects client
```

**Risk tag:** `[IPC] [partial-frame] [no-panic]`

---

### H7: Correlation ID Collision / Prediction

**Description:** An attacker predicts correlation IDs to hijack or confuse request-response pairing.

**Current mitigation:** None in the IPC layer. `correlation: u64` is treated as opaque caller data.
This is an application-level concern.

**Risk tag:** `[IPC] [correlation]`

---

### H8: Version Downgrade (Version = 0)

**Description:** A client sends `version = 0` to probe for protocol downgrade or
to trigger alternate code paths.

**Current mitigation:** `UnsupportedVersion` rejects any version ≠ 1.

**Risk tag:** `[protocol-boundary] [IPC]`

---

### H9: u32→usize Overflow on 32-bit Targets

**Description:** On 32-bit targets, a `payload_len` value larger than `u32::MAX` cannot
fit in `usize`, causing `PayloadLengthOutOfRange` before `PayloadTooLarge` is checked.

**Current mitigation:** `u32_to_usize(payload_len)?` returns `PayloadLengthOutOfRange`
if the conversion fails, before comparing to `MaxPayloadBytes`.

**Risk tag:** `[arithmetic] [IPC] [portability]`

---

### H10: Read Buffer Accumulation Without Header Decode

**Description:** A client sends bytes that are always just short of `IPC_HEADER_LEN` (23 bytes),
causing `read_buffer` to accumulate up to `READ_CHUNK_BYTES * N` bytes without ever
triggering the header decode condition.

**Current mitigation:** Each poll event appends at most 4096 bytes. Eventually the client
would need to send more data, and the server's while-loop continues as long as
`read_buffer.len() >= IPC_HEADER_LEN`. A client sending exactly 23 bytes repeatedly
would not trigger header decode but also would not trigger any frame dispatch.

**Residual risk:** The server continues to read up to 4096 bytes per poll event.
If the client sends 23 bytes per event indefinitely, the buffer accumulates.
Eventually some other mechanism (mio watermarks, OS TCP buffers) would back-pressure.

**Risk tag:** `[bounded-state] [DoS] [IPC]`

---

### H11: Multiple Complete Frames in Single Read

**Description:** A client sends multiple complete frames in one `read()` syscall.
The server processes them in a `while` loop. If one frame causes a dispatch error,
the remaining frames in the buffer may be orphaned or cause confusion.

**Current mitigation:** The `while` loop processes frames in order until the buffer
is insufficient for the next header. On error, the client is disconnected, clearing
the buffer. Any remaining frames in the buffer are dropped with the client.

**Risk tag:** `[IPC] [pipelining]`

---

### H12: Command ID Enum Exhaustion

**Description:** All 16 valid command IDs are processed by `dispatch_command_with_resolver`.
A future 17th command would silently fall through or cause a panic if not handled.

**Current mitigation:** `IpcCommand` is `#[non_exhaustive]`. The `from_u16` function
rejects unknown IDs. The dispatch function has a `match` on all 16 variants.

**Risk tag:** `[IPC] [extensibility]`
