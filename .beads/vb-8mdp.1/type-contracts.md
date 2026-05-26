# VB IPC Type Contracts — Fragmented-Frame and Oversize-Message Tests

## IpcFrameHeader

### Smart Constructor
```rust
pub const fn new(command: IpcCommand, flags: u16, correlation: u64, payload_len: u32) -> Self
```
- Total: accepts any u32 `payload_len` (bounds check deferred to `decode`)

### `encode(self) -> Result<[u8; IPC_HEADER_LEN], IpcError>`
**Postconditions:**
- Returns `Ok(bytes)` where `bytes[0..4] == IPC_MAGIC.to_le_bytes()`
- Returns `Ok(bytes)` where `bytes[4..6] == IPC_VERSION.to_le_bytes()`
- Returns `Ok(bytes)` where `bytes[20..24] == self.payload_len.to_le_bytes()`
- `Err(HeaderEncodeFailed)` only if byteorder write fails (cannot occur for `[u8; 24]`)

### `decode(bytes: &[u8; IPC_HEADER_LEN], max_payload: MaxPayloadBytes) -> Result<Self, IpcError>`

**Decode order (total function, no panics for any [u8; 24]):**

| Step | Field | Check | Error |
|------|-------|-------|-------|
| 1 | magic (bytes 0..4) | `magic == IPC_MAGIC` | `InvalidMagic { actual }` |
| 2 | version (bytes 4..6) | `version == IPC_VERSION` | `UnsupportedVersion { actual }` |
| 3 | command (bytes 6..8) | `command ∈ v1 set (1..16)` | `UnknownCommand(u16)` |
| 4 | reserved (bytes 10..12) | `reserved == 0` | `ReservedNonZero { actual }` |
| 5 | correlation (bytes 12..20) | any u64 valid | — |
| 6 | payload_len (bytes 20..24) | `u32_to_usize(payload_len) <= max_payload.get()` | `PayloadTooLarge { actual, limit }` |

**Type-level constraints enforced by decode order:**
1. Magic must be checked before version
2. Version must be checked before command
3. Command validity must be checked before reserved
4. Reserved must be checked before payload_len
5. payload_len must be checked against `MaxPayloadBytes` before any allocation

**Key properties:**
- No panics on any 24-byte input
- Returns `Result<Self, IpcError>` for all 2^192 inputs
- `InvalidMagic` is returned before `UnsupportedVersion` for any magic mismatch
- `ReservedNonZero` is returned before `PayloadTooLarge` for any reserved non-zero

## MaxPayloadBytes

```rust
pub const fn new(value: NonZeroUsize) -> Self
pub const DEFAULT: Self  // 1_048_576 bytes (1 MiB)
```

- Wraps `NonZeroUsize` — zero is not representable
- `DEFAULT.get() == 1_048_576` is a compile-time constant

## BoundedPayload

```rust
pub fn new(payload: Bytes, max: MaxPayloadBytes) -> Result<Self, IpcError>
```

**Precondition:** `payload.len() <= max.get()`
**Postcondition on Ok:** `self.bytes().len() == payload.len()`
**Error:** `PayloadTooLarge { actual: payload.len(), limit: max.get() }`

## IpcCommand

```rust
pub fn from_u16(value: u16) -> Result<Self, IpcError>
```

- Returns `Ok` for values 1..16 only
- All other values return `Err(UnknownCommand(value))`

## IpcError Variants (IPC-specific)

| Variant | Semantic Meaning |
|---------|-----------------|
| `InvalidMagic { actual: u32 }` | Frame bytes do not start with `0x5642_4C54` |
| `UnsupportedVersion { actual: u16 }` | Frame version is not 1 |
| `UnknownCommand(u16)` | Wire command ID not in v1 set |
| `ReservedNonZero { actual: u16 }` | Header byte 10..12 is non-zero |
| `PayloadTooLarge { actual: usize, limit: usize }` | `payload_len` exceeds configured bound |
| `PayloadLengthOutOfRange { actual: u32 }` | `payload_len` cannot fit in target `usize` |
| `PayloadLengthMismatch { header: usize, actual: usize }` | Supplied bytes ≠ declared `payload_len` |
| `PayloadDecodeFailed` | Postcard decode of payload bytes failed |
| `HeaderDecodeFailed` | Short read on 24-byte header decode |
| `HeaderEncodeFailed` | byteorder write into fixed array failed |
| `PayloadEncodeFailed` | Postcard encoding failed |
| `ResponseDecodeFailed` | Postcard decoding of server response failed |

## Decode/Encode Roundtrip Contract

For any `IpcFrameHeader h` produced by `IpcFrameHeader::new(cmd, flags, corr, plen)`:
```
let enc = h.encode()        // MUST succeed (write into [u8; 24] cannot fail)
let dec = IpcFrameHeader::decode(&enc, MaxPayloadBytes::DEFAULT)
dec == Ok(h)
```

## Fragmented-Read Contract (Server-Side)

For a `ClientConnection` with `read_buffer: Vec<u8>`:
1. Bytes are appended via `append_read_bytes` only up to `READ_CHUNK_BYTES` (4096) per poll event
2. Header decode is attempted only when `read_buffer.len() >= IPC_HEADER_LEN` (24)
3. If header decode fails → error response sent, client disconnected
4. If header decode succeeds → `frame_total_len(header)` is computed
5. Payload read proceeds only when `read_buffer.len() >= frame_total_len(header)`
6. **Critical**: `Vec::with_capacity(payload_len)` is NEVER called before `payload_len` is validated

## Oversize-Payload Contract

Given `header.payload_len = plen` and `MaxPayloadBytes max`:
- If `usize::try_from(plen).is_err()` → `PayloadLengthOutOfRange`
- If `usize::from(plen) > max.get()` → `PayloadTooLarge { actual, limit }`
- The above check occurs AFTER magic, version, command, reserved checks
- A frame with oversized `payload_len` is rejected before any allocation
