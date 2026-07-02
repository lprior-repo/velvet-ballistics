# Codebase Map: vb-8mdp.1 - IPC Fragmented-Frame and Oversize-Message Tests

## Bead Context
- **Bead ID**: vb-8mdp.1
- **Title**: Add IPC fragmented-frame and oversize-message tests
- **Source Checkout**: /home/lewis/src/velvet-ballistics
- **Isolated Workspace**: /home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-1

## Target Crate
**`crates/vb_ipc/`** - VB binary IPC protocol implementation

---

## 1. IPC Protocol Constants

**File**: `crates/vb_ipc/src/constants.rs`

| Constant | Value | Description |
|----------|-------|-------------|
| `IPC_MAGIC` | `0x5642_4C54` | Frame magic "VBLT" little-endian |
| `IPC_VERSION` | `1` | Supported schema version |
| `IPC_HEADER_LEN` | `24` | Fixed header bytes |

**Wire Layout**: `magic(4) + version(2) + command(2) + flags(2) + reserved(2) + correlation(8) + payload_len(4) = 24`

---

## 2. Frame Header Parsing

**File**: `crates/vb_ipc/src/frame_types.rs`

### IpcFrameHeader
```rust
pub struct IpcFrameHeader {
    pub command: IpcCommand,     // IpcCommand enum
    pub flags: u16,              // Command-specific flags
    pub correlation: u64,         // Request/reply correlation
    pub payload_len: u32,        // Postcard payload byte length
}
```

### Key Methods
- **`encode() -> Result<[u8; IPC_HEADER_LEN], IpcError>`** - Encodes header to wire format
- **`decode(bytes: &[u8; IPC_HEADER_LEN], max_payload: MaxPayloadBytes) -> Result<Self, IpcError>`** - Decodes and validates header

### Validation Order (per `kani_ipc_decode_order.rs`)
1. Magic check (must match `IPC_MAGIC`)
2. Version check (must match `IPC_VERSION`)
3. Command parse (`IpcCommand::from_u16`)
4. Reserved field check (must be 0)
5. Payload length bounds check against `MaxPayloadBytes`

---

## 3. Error Types

**File**: `crates/vb_ipc/src/error.rs`

### IpcError Variants (relevant to fragmented/oversize)
```rust
pub enum IpcError {
    Full,                                      // Queue full
    Disconnected,                              // Queue disconnected
    PayloadTooLarge { actual: usize, limit: usize },
    InvalidMagic { actual: u32 },
    UnsupportedVersion { actual: u16 },
    UnknownCommand(u16),
    ReservedNonZero { actual: u16 },
    PayloadLengthMismatch { header: usize, actual: usize },
    HeaderEncodeFailed,
    HeaderDecodeFailed,
    PayloadLengthOutOfRange { actual: u32 },
    PayloadEncodeFailed,
    PayloadDecodeFailed,
    ResponseDecodeFailed,
}
```

### Key Validation Functions
- **`u32_to_usize(value: u32) -> Result<usize, IpcError>`** - Safe conversion with bounds check

---

## 4. Bounded Payload Types

**File**: `crates/vb_ipc/src/bounded.rs`

### MaxPayloadBytes
```rust
pub struct MaxPayloadBytes(NonZeroUsize);

impl MaxPayloadBytes {
    pub const DEFAULT: Self = Self(NonZeroUsize::new(1_048_576).unwrap()); // 1 MiB
    pub const fn new(value: NonZeroUsize) -> Self
    pub(crate) fn get(self) -> usize
}
```

### BoundedPayload
```rust
pub struct BoundedPayload(Bytes);

impl BoundedPayload {
    pub fn new(payload: Bytes, max: MaxPayloadBytes) -> Result<Self, IpcError>
    pub const fn bytes(&self) -> &Bytes
}
```

---

## 5. Frame Encoding/Decoding Utilities

**File**: `crates/vb_ipc/src/frame.rs`

### Key Functions
| Function | Signature |
|----------|-----------|
| `encode_frame` | `(command: IpcCommand, flags: u16, correlation: u64, payload: &[u8]) -> Result<Vec<u8>, IpcError>` |
| `decode_frame_header` | `(bytes: &[u8; IPC_HEADER_LEN]) -> Result<IpcFrameHeader, IpcError>` |
| `decode_frame_payload` | `(header: &IpcFrameHeader, payload: &[u8]) -> Result<IpcPayload, IpcError>` |
| `validate_frame_magic` | `(bytes: &[u8]) -> Result<(), IpcError>` |
| `validate_frame_bounds` | `(header: &IpcFrameHeader, max_payload: MaxPayloadBytes) -> Result<(), IpcError>` |
| `read_frame_header` | `<R: Read>(reader: &mut R) -> Result<IpcFrameHeader, IpcError>` |
| `read_frame_payload` | `<R: Read>(reader: &mut R, header: &IpcFrameHeader) -> Result<Vec<u8>, IpcError>` |
| `read_frame_payload_bounded` | `<R: Read>(reader: &mut R, header: &IpcFrameHeader, max_payload: MaxPayloadBytes) -> Result<Vec<u8>, IpcError>` |

---

## 6. Postcard Encode/Decode

**File**: `crates/vb_ipc/src/codec.rs`

### Key Functions
```rust
pub fn encode_payload<T: Serialize>(value: &T) -> Result<Vec<u8>, IpcError>
pub fn decode_payload<T: Deserialize<'b>>(bytes: &'b [u8]) -> Result<T, IpcError>
```

Postcard is used for all IPC payload serialization. Frame validation occurs before Postcard decode.

---

## 7. IpcCommand Enum

**File**: `crates/vb_ipc/src/commands.rs`

```rust
pub enum IpcCommand {
    SubmitRun = 1,
    SubmitRunInline = 2,
    CancelRun = 3,
    InspectRun = 4,
    ListEvents = 5,
    AnswerAsk = 6,
    CompleteAction = 7,
    FailAction = 8,
    DrainTrace = 9,
    Health = 10,
    Shutdown = 11,
    ListRuns = 12,
    GetMetrics = 13,
    GetWorkflowGraph = 14,
    GetTaintReport = 15,
    VerifyWorkflow = 16,
}
```

---

## 8. Existing Fragmented/Partial Frame Tests

**File**: `crates/vb_ipc/src/server/impl_tests.rs`

### Partial Frame Tests
- **`server_waits_for_complete_frame_when_partial_sent`** (line 592)
  - Sends 3 bytes of magic (partial header)
  - Verifies server handles without error

- **`slow_client_partial_frame_keeps_read_buffer_bounded`** (line 312)
  - Slow client writes partial header-only frame
  - Verifies bounded partial frame retained

- **`handle_readable_returns_false_for_partial_header`** (line 1866)
  - Sends 10 bytes (partial header)
  - Verifies `handle_readable` returns `Ok(false)` (WouldBlock)

- **`handle_readable_returns_false_for_complete_header_partial_payload`** (line 1893)
  - Sends complete header + partial payload
  - Verifies `handle_readable` returns `Ok(false)` (WouldBlock)

---

## 9. Existing Oversize Message Tests

**File**: `crates/vb_ipc/src/tests.rs`

- **`oversized_payload_is_rejected`** (line 85)
- **`bounded_payload_rejects_oversized_with_exact_counts`** (line 314)
- **`adversarial_decode_frame_rejects_oversized_payload_bytes`** (line 1498)
- **`frame_validation_oversized_payload_exceeding_default_max_returns_error`** (line 1649)

**File**: `crates/vb_ipc/src/server/impl_tests.rs`

- **`slow_client_oversized_frame_disconnects_without_unbounded_growth`** (line 364)

**File**: `crates/vb_ipc/src/frame.rs`

- **`fuzz_decode_frame_rejects_oversized_payload`** (line 284)
- **`validate_frame_bounds_rejects_oversized_length`** (line 524)

---

## 10. Existing Kani Proofs (IPC Header Validation)

**File**: `crates/vb_ipc/src/kani_ipc_header.rs`
- `kani_ipc_header_decode_valid`
- `kani_ipc_header_rejects_bad_magic`
- `kani_ipc_header_rejects_bad_version`
- `kani_ipc_header_rejects_reserved_nonzero`
- `kani_ipc_header_decode_various_commands`
- `kani_ipc_header_preserves_all_fields`

**File**: `crates/vb_ipc/src/kani_ipc_decode_order.rs`
- `kani_harness_ipc_decode_order` - Magic before version ordering
- `kani_harness_ipc_reserved_nonzero_before_payload_len`
- `kani_harness_ipc_magic_before_version`

**File**: `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs`
- `kani_ipc_header_rejects_oversize_payload`
- `kani_ipc_header_accepts_within_bound`
- `kani_ipc_header_rejects_exactly_over_limit`
- `kani_ipc_header_accepts_exactly_at_limit`
- `kani_ipc_header_rejects_any_payload_when_max_zero`
- `kani_ipc_header_accepts_large_with_large_max`

---

## 11. Target Test File for New Tests

**Primary location**: `crates/vb_ipc/src/tests.rs`
- Integration tests for IPC frame encoding/decoding
- Uses `proptest` for property-based tests

**Alternative location**: `crates/vb_ipc/src/server/impl_tests.rs`
- Server integration tests with socket I/O

---

## 12. Dependencies

**Key dependencies** (from `Cargo.toml`):
- `byteorder` - Little-endian read/write
- `bytes` - `Bytes` buffer type
- `postcard` - Compact Postcard serialization
- `serde` - Serialization framework

---

## 13. Gaps Identified (Test Opportunities)

### Fragmented Frame Gaps
1. **Multi-frame streaming**: No test for receiving multiple frames in a single recv() call
2. **Frame boundary with partial payload at buffer edge**: No adversarial test for frame straddling recv() boundaries
3. **Pipelined partial frames**: Server receiving frame 1 partial + frame 2 partial
4. **Header reassembly after partial reads**: Byte-level boundary testing

### Oversize Message Gaps
1. **Header-declared oversize vs actual oversize**: Different rejection paths
2. **Oversize at exact MaxPayloadBytes boundary**
3. **Oversize with valid/invalid magic combinations**

### Reserved Flags
1. **Non-zero reserved flags handling** (already tested via Kani)

---

## 14. APIs for Test Implementation

### Frame Construction
```rust
IpcFrameHeader::new(command, flags, correlation, payload_len)
IpcFrameHeader::encode() -> Result<[u8; IPC_HEADER_LEN], IpcError>
IpcFrame::new(header, payload, max_payload) -> Result<Self, IpcError>
```

### Validation
```rust
IpcFrameHeader::decode(bytes, max_payload) -> Result<Self, IpcError>
validate_frame_magic(bytes) -> Result<(), IpcError>
validate_frame_bounds(header, max_payload) -> Result<(), IpcError>
```

### Constants
```rust
IPC_MAGIC: u32 = 0x5642_4C54
IPC_HEADER_LEN: usize = 24
MaxPayloadBytes::DEFAULT = 1_048_576
```

---

## 15. Risk Tags

- `ipc` - Core IPC protocol
- `binary-protocol` - Wire format parsing
- `streaming` - Partial frame handling
- `parser-codec` - Frame decode/encode
- `unsafe-ub` - Byte order operations, buffer handling (but no actual unsafe code)
