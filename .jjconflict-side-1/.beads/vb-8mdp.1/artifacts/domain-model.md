# VB IPC Domain Model — Fragmented-Frame and Oversize-Message Tests

## Ubiquitous Language

| Term | Definition |
|------|------------|
| IPC Frame | A binary message consisting of a fixed 24-byte header followed by a variable-length payload |
| Magic | 4-byte signature `0x5642_4C54` (`VBLT` in LE) identifying valid VB IPC frames |
| Header | 24-byte fixed-width binary record encoding command, flags, correlation ID, and payload length |
| Payload | Variable-length Postcard-encoded bytes following the header |
| Fragmented Frame | A frame received in multiple TCP/Mio chunks before the complete header or payload is available |
| Oversize Message | A frame whose declared `payload_len` exceeds the configured `MaxPayloadBytes` bound (default: 1 MiB) |
| Partial Header | Fewer than 24 bytes of header data available in the read buffer |
| Partial Payload | Fewer than `header.payload_len` bytes available after a complete header is decoded |
| Decode Order | Strict sequential validation: magic → version → command → reserved → correlation → payload_len |
| Bounded Allocation | Server allocates receive buffer only to the actual bytes read, never to the declared payload length |

## Wire Format (§21 Binary Layout)

```
Byte 0..4:   MAGIC (u32 LE, 0x5642_4C54)
Byte 4..6:   VERSION (u16 LE, must be 1)
Byte 6..8:   COMMAND (u16 LE, 1..16)
Byte 8..10:  FLAGS (u16 LE, caller-selected)
Byte 10..12: RESERVED (u16 LE, must be 0)
Byte 12..20: CORRELATION (u64 LE, caller-selected)
Byte 20..24: PAYLOAD_LEN (u32 LE, max 1 MiB default)
```

Total header: **24 bytes** (IPC_HEADER_LEN)

## Value Objects

### `IpcFrameHeader`
- `command: IpcCommand` — v1 command enum, IDs 1..16
- `flags: u16` — opaque caller flags
- `correlation: u64` — request/reply correlator
- `payload_len: u32` — exact byte count of Postcard payload

### `MaxPayloadBytes`
- Newtype over `NonZeroUsize`
- Default: 1 MiB (1_048_576 bytes)
- Used as a hard ceiling on `payload_len` before any allocation

### `BoundedPayload`
- Newtype over `Bytes`
- Constructed only after `payload_len` has been validated against `MaxPayloadBytes`

### `IpcCommand`
- Fixed v1 enum: SubmitRun=1, SubmitRunInline=2, CancelRun=3, InspectRun=4,
  ListEvents=5, AnswerAsk=6, CompleteAction=7, FailAction=8, DrainTrace=9,
  Health=10, Shutdown=11, ListRuns=12, GetMetrics=13, GetWorkflowGraph=14,
  GetTaintReport=15, VerifyWorkflow=16

### `IpcPayload`
- Postcard-serializable enum covering all command variants
- Contains nested owned vectors (`Vec<u8>`) for inputs/outputs/errors

## Frame Decode State Machine

```
                    partial_bytes ≥ 1
                    ┌──────────────────────────────┐
                    │                              │
                    ▼                              │
              WAITING_HEADER                       │
                    │                              │
         ┌──────────┴───────────┐                  │
         │                      │                  │
  bytes < 24              bytes ≥ 24               │
  partial header          try decode ──────► InvalidHeader ──► ERROR
         │                      │                  │
         │                      │ magic ok         │
         │                      ▼                  │
         │               WAITING_HEADER_VALIDATED  │
         │                      │                  │
         │           ┌──────────┴──────────┐      │
         │           │                     │      │
         │    version != 1         version == 1   │
         │           │                     │      │
         │           ▼                     │      │
         │    UnsupportedVersion            │      │
         │           │                     │      │
         │           │          ┌──────────┴───┐  │
         │           │          │              │  │
         │           │    command ∉ v1    command ∈ v1 │
         │           │          │              │  │
         │           │          ▼              │  │
         │           │    UnknownCommand       │  │
         │           │          │              │  │
         │           │          │       reserved ≠ 0 │
         │           │          │              │  │
         │           │          │              ▼  │
         │           │          │      ReservedNonZero │
         │           │          │              │   │
         │           │          │    payload_len > max │
         │           │          │              │   │
         │           │          │              ▼   │
         │           │          │       PayloadTooLarge │
         │           │          │              │   │
         │           │          │              │◄──┘
         │           │          │              │
         │           │          └──────┬───────┘
         │           │                 │
         │           │        payload_len ≤ max
         │           │                 │
         │           │                 ▼
         │           │         HEADER_VALID
         │           │                 │
         │           │    ┌────────────┴────────────┐
         │           │    │                         │
         │           │ bytes_total ≤ buf_len  bytes_total > buf_len
         │           │    │                         │
         │           │    ▼                         ▼
         │           │ WAITING_PAYLOAD          WAITING_PAYLOAD
         │           │    │                         │
         │           │    │◄────────────────────────┘
         │           │    │
         │           │    ▼
         │           │ bytes_total ≤ buf_len
         │           │    │
         │           │    ▼
         │           │ PAYLOAD_READY
         │           │    │
         │           │    ▼
         │           │ decode_postcard_payload
         │           │    │
         │           │    ├── Ok(IpcPayload) ──► DISPATCH
         │           │    └── Err(_) ──► PayloadDecodeFailed
         │           │
         └──────────┘
```

## Entities / Aggregates

- **`ClientConnection`**: Per-client state on the server — `read_buffer: Vec<u8>`, `write_buffer: Vec<u8>`, `stream: UnixStream`
- **`IpcServer`**: Manages `HashMap<Token, ClientConnection>`, `Poll`, `Token` allocator

## Forbidden States

1. Allocating `Vec<u8>` of size `payload_len` before header validation completes
2. Decoding Postcard payload before header magic/version/command/reserved/payload_len are fully validated
3. Accepting a frame where `payload_len > MaxPayloadBytes::DEFAULT.get()`
4. Server `read_buffer` growing beyond the declared `payload_len` bytes
5. A reserved header field value other than 0

## Invariants

- `IPC_HEADER_LEN == 24` always
- `IPC_MAGIC == 0x5642_4C54` always
- `IPC_VERSION == 1` always
- `MaxPayloadBytes::DEFAULT.get() == 1_048_576` always
- `IpcFrameHeader::decode` is total over `[u8; 24]` inputs, returning `Result<Self, IpcError>` for all 2^192 inputs
- The decode order is a total function: any 24-byte sequence maps to exactly one error variant or one valid header
