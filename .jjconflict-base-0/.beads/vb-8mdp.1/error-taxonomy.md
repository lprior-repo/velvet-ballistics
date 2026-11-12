# VB IPC Error Taxonomy — Fragmented-Frame and Oversize-Message Tests

## Error Category Hierarchy

```
IpcError
  ├── Decoding Errors (wire → memory)
  │     ├── HeaderDecodeFailed        — short read on 24-byte boundary
  │     ├── InvalidMagic             — magic ≠ 0x5642_4C54
  │     ├── UnsupportedVersion       — version ≠ 1
  │     ├── UnknownCommand           — command ∉ {1..16}
  │     ├── ReservedNonZero          — reserved field ≠ 0
  │     ├── PayloadLengthOutOfRange  — u32 payload_len doesn't fit usize
  │     ├── PayloadTooLarge          — payload_len > MaxPayloadBytes
  │     ├── PayloadLengthMismatch    — supplied bytes ≠ declared payload_len
  │     └── PayloadDecodeFailed      — Postcard deserialize failed
  │
  ├── Encoding Errors (memory → wire)
  │     ├── HeaderEncodeFailed       — byteorder write into [u8; 24] failed
  │     ├── PayloadEncodeFailed      — Postcard serialize failed
  │     └── ResponseDecodeFailed     — server-side Postcard response decode failed
  │
  └── Queue/Connection Errors
        ├── Full             — MemoryIngress queue at capacity
        └── Disconnected     — all producers or consumers dropped
```

## Detailed Error Variants

### HeaderDecodeFailed
- **Category:** Decoding
- **Byte source:** `IpcFrameHeader::decode` — `byteorder::ReadBytesExt::read_u32/u16`
- **Trigger:** Input slice shorter than required for field (short TCP read)
- **Guarantee:** Precedes all semantic errors (magic, version, etc.) in decode order

### InvalidMagic { actual: u32 }
- **Category:** Decoding — Semantic validation
- **Byte offset:** bytes 0..4 (read as u32 LE)
- **Trigger:** `actual != 0x5642_4C54`
- **Adversarial uses:** Magic injection attacks, probing for协议 confusion
- **Decode-order guarantee:** Checked FIRST before version, command, reserved, payload_len
- **Diagnostic code:** `0x3004`

### UnsupportedVersion { actual: u16 }
- **Category:** Decoding — Semantic validation
- **Byte offset:** bytes 4..6 (read as u16 LE)
- **Trigger:** `actual != 1`
- **Adversarial uses:** Version downgrade/probing attacks
- **Decode-order guarantee:** Checked AFTER magic, BEFORE command
- **Diagnostic code:** `0x3005`

### UnknownCommand(u16)
- **Category:** Decoding — Command validation
- **Byte offset:** bytes 6..8 (read as u16 LE)
- **Trigger:** `value ∉ {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16}`
- **Decode-order guarantee:** Checked AFTER version, BEFORE reserved
- **Diagnostic code:** `0x3006`

### ReservedNonZero { actual: u16 }
- **Category:** Decoding — Protocol compliance
- **Byte offset:** bytes 10..12 (read as u16 LE)
- **Trigger:** `actual != 0`
- **Adversarial uses:** Foot-in-door attacks, protocol variant probing
- **Decode-order guarantee:** Checked AFTER command, BEFORE correlation
- **Diagnostic code:** `0x3007`

### PayloadLengthOutOfRange { actual: u32 }
- **Category:** Decoding — Arithmetic bounds
- **Byte offset:** bytes 20..24 (read as u32 LE)
- **Trigger:** `usize::try_from(actual).is_err()` (on 32-bit platforms or very large values)
- **Decode-order guarantee:** Checked as part of `u32_to_usize(payload_len)` before `PayloadTooLarge`
- **Diagnostic code:** `0x300B`

### PayloadTooLarge { actual: usize, limit: usize }
- **Category:** Decoding — Allocation guard
- **Byte offset:** bytes 20..24 (read as u32 LE, converted to usize)
- **Trigger:** `actual > limit` where `limit = MaxPayloadBytes::DEFAULT.get() = 1_048_576`
- **Adversarial uses:** Memory exhaustion attacks, declaring huge payloads
- **Decode-order guarantee:** Checked AFTER reserved, AFTER u32→usize conversion
- **Diagnostic code:** `0x3003`

### PayloadLengthMismatch { header: usize, actual: usize }
- **Category:** Decoding — Frame integrity
- **Trigger:** `header.payload_len != payload_bytes.len()` during `IpcFrame::new`
- **Adversarial uses:** Truncated frame attacks, payload truncation
- **Diagnostic code:** `0x3008`

### PayloadDecodeFailed
- **Category:** Decoding — Postcard parse
- **Trigger:** `postcard::from_bytes::<IpcPayload>(&payload_bytes).is_err()`
- **Adversarial uses:** Garbage payload injection, malformed Postcard bytes
- **Diagnostic code:** `0x300D`

### HeaderEncodeFailed
- **Category:** Encoding — Memory→Wire
- **Trigger:** `byteorder::WriteBytesExt::write_u32/u16` into `[u8; 24]` fails (cannot happen)
- **Diagnostic code:** `0x3009`

### PayloadEncodeFailed
- **Category:** Encoding — Memory→Wire
- **Trigger:** `postcard::to_allocvec(&ipc_payload)` fails
- **Diagnostic code:** `0x300C`

### ResponseDecodeFailed
- **Category:** Encoding — Server response
- **Trigger:** Client fails to deserialize server `IpcResponse`
- **Diagnostic code:** `0x300E`

### Full
- **Category:** Queue — Backpressure
- **Trigger:** `MemoryIngress::try_submit` on a full bounded channel
- **Diagnostic code:** `0x3001`

### Disconnected
- **Category:** Queue — Connection lifecycle
- **Trigger:** All senders dropped (producer disconnect) or receiver dropped (consumer disconnect)
- **Diagnostic code:** `0x3002`

## Error-to-Runtime-Code Mapping

| Error Variant | Runtime Code |
|--------------|-------------|
| `Full` | `"QUEUE_FULL"` |
| `PayloadTooLarge` | `"IPC_PAYLOAD_TOO_LARGE"` |
| `PayloadLengthOutOfRange` | `"IPC_PAYLOAD_TOO_LARGE"` |
| `InvalidMagic` | `"IPC_FRAME_INVALID"` |
| `UnsupportedVersion` | `"IPC_FRAME_INVALID"` |
| `UnknownCommand(_)` | `"IPC_FRAME_INVALID"` |
| `ReservedNonZero(_)` | `"IPC_FRAME_INVALID"` |
| `PayloadLengthMismatch(_,_)` | `"IPC_FRAME_INVALID"` |
| `HeaderDecodeFailed` | `"IPC_FRAME_INVALID"` |
| `PayloadDecodeFailed` | `"IPC_FRAME_INVALID"` |
| `ResponseDecodeFailed` | `"IPC_FRAME_INVALID"` |
| `Disconnected` | (no runtime code) |
| `HeaderEncodeFailed` | (no runtime code) |
| `PayloadEncodeFailed` | (no runtime code) |

## Decode-Order Error Priority

When given a 24-byte header that triggers multiple error conditions:

1. **Priority 1:** `InvalidMagic` — checked first, must reject magic before version
2. **Priority 2:** `UnsupportedVersion` — checked second, after magic confirmed
3. **Priority 3:** `UnknownCommand` — checked third, after version confirmed
4. **Priority 4:** `ReservedNonZero` — checked fourth, after command confirmed
5. **Priority 5:** `PayloadLengthOutOfRange` — u32→usize conversion
6. **Priority 6:** `PayloadTooLarge` — final allocation guard

**Rationale:** Magic is the first line of defense against protocol confusion.
Reserved non-zero is checked before payload_len to catch protocol variant probing.
Payload bounds are checked last, after all structural fields are valid.

## Server-Side Error Handling

| Error from header decode | Server Action |
|-------------------------|---------------|
| `InvalidMagic` | Send `FrameError` response, disconnect client |
| `UnsupportedVersion` | Send `FrameError` response, disconnect client |
| `UnknownCommand` | Send `FrameError` response, disconnect client |
| `ReservedNonZero` | Send `FrameError` response, disconnect client |
| `PayloadTooLarge` | Send `FrameError` response, disconnect client |
| `PayloadLengthOutOfRange` | Send `FrameError` response, disconnect client |

**Policy:** All header-decode errors cause client disconnection without reading
the declared payload bytes from the socket.
