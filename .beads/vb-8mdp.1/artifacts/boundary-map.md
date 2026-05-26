# VB IPC Boundary Map — Fragmented-Frame and Oversize-Message Tests

## Crate Boundary: `vb_ipc`

```
┌─────────────────────────────────────────────────────────────────┐
│                         vb_ipc crate                            │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  public API  │  │  public API  │  │     public API       │  │
│  │  frame.rs    │  │ frame_types  │  │   ingress.rs         │  │
│  │              │  │              │  │                      │  │
│  │ encode_frame │  │IpcFrameHeader│  │ IngressFrame         │  │
│  │ decode_hdr   │  │IpcFrame      │  │ MemoryIngress        │  │
│  │ decode_payload│ │BoundedPayload│  │ MemoryIngressSender  │  │
│  │ read_hdr     │  │MaxPayloadBytes│ │                      │  │
│  │ read_payload │  │              │  │                      │  │
│  │ write_frame  │  │              │  │                      │  │
│  │validate_magic│  │              │  │                      │  │
│  │validate_bounds│ │              │  │                      │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘  │
│         │                 │                      │              │
│  ┌──────▼─────────────────▼──────────────────────▼───────────┐  │
│  │                    codec.rs                               │  │
│  │         encode_payload / decode_payload (Postcard)        │  │
│  └───────────────────────────────────────────────────────────┘  │
│                            │                                    │
│  ┌─────────────────────────▼─────────────────────────────────┐  │
│  │              server/impl_.rs + handlers.rs                 │  │
│  │                                                              │  │
│  │  IpcServer, ClientConnection, handle_readable, dispatch     │  │
│  │  mio-based Unix socket server                               │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Layer Architecture

### Layer 1: Pure Core (no I/O, no allocation)

```rust
// frame_types.rs — pure data types
IpcFrameHeader::new(command, flags, correlation, payload_len) -> Self
IpcFrameHeader::encode(self) -> Result<[u8; 24], IpcError>  // total
IpcFrameHeader::decode(bytes: &[u8; 24], MaxPayloadBytes) -> Result<Self, IpcError>

// bounded.rs — allocation guards
BoundedPayload::new(Bytes, MaxPayloadBytes) -> Result<BoundedPayload, IpcError>
MaxPayloadBytes::new(NonZeroUsize) -> Self

// commands.rs — command validation
IpcCommand::from_u16(u16) -> Result<Self, IpcError>
IpcCommand::as_u16(self) -> u16
```

### Layer 2: Codec Boundary (Postcard ↔ Rust types)

```rust
// codec.rs
encode_payload(&IpcPayload) -> Result<Vec<u8>, IpcError>   // postcard
decode_payload::<IpcPayload>(&[u8]) -> Result<IpcPayload, IpcError>  // postcard
```

**Boundary crossing:** Postcard bytes ↔ typed Rust enums. No domain logic here;
pure serialization/deserialization at the edge of the core.

### Layer 3: Frame Utilities (with std::io::Read/Write)

```rust
// frame.rs — Read/Write trait consumers
read_frame_header<R: Read>(reader) -> Result<IpcFrameHeader, IpcError>
read_frame_payload<R: Read>(reader, header) -> Result<Vec<u8>, IpcError>
write_frame<W: Write>(writer, ...) -> Result<(), IpcError>
```

### Layer 4: Server Shell (mio I/O, event loop)

```rust
// server/impl_.rs
IpcServer::bind(socket_path) -> Result<Self, IpcServerError>
IpcServer::poll_once(...) -> Result<bool, IpcServerError>
handle_readable(...)  // mio read event → decode → dispatch → response
handle_writable(...) // mio write event → drain write buffer
```

### Layer 5: Ingress Queue (crossbeam_channel)

```rust
// ingress.rs
MemoryIngress::bounded(QueueCapacity) -> Self
IngressFrame::new(RunId, WorkflowDigest, Bytes, MaxPayloadBytes) -> Result<Self, IpcError>
```

## Boundary Hazards

### Hazard 1: Postcard Decode at Protocol Boundary
- **Risk:** Malformed Postcard bytes cause panic or indefinite allocation
- **Mitigation:** `decode_frame_payload` wraps postcard error in `PayloadDecodeFailed`
- **Invariant:** `payload.len()` must equal `header.payload_len` before Postcard decode

### Hazard 2: Read Buffer Accumulation (Partial Frames)
- **Risk:** Slow client accumulates unbounded bytes in `read_buffer` before disconnect
- **Mitigation:** `frame_total_len(header)` is checked before reading payload;
  client is disconnected on header decode error before payload is ever read
- **Invariant:** `read_buffer.len() <= READ_CHUNK_BYTES * N` where N is polls since connect

### Hazard 3: Oversize Payload Declaration (Magic Injection)
- **Risk:** Attacker sends magic `0x5642_4C54` + oversized `payload_len` to trigger OOM
- **Mitigation:** `PayloadTooLarge` is checked in `IpcFrameHeader::decode` before any allocation
- **Invariant:** `header.payload_len <= MaxPayloadBytes::DEFAULT.get()` before `Vec` allocation

### Hazard 4: Reserved Field Non-Zero
- **Risk:** Protocol variant probing via non-zero reserved bytes
- **Mitigation:** `ReservedNonZero` returned before any payload processing
- **Invariant:** Reserved byte 10..12 must be 0 in all frames

### Hazard 5: Payload Length on 32-bit Targets
- **Risk:** `u32::MAX` payload_len causes `usize::try_from` to fail
- **Mitigation:** `PayloadLengthOutOfRange` checked before `PayloadTooLarge`
- **Invariant:** `u32::try_from(header.payload_len).is_ok()` on all architectures

## IPC Frame ↔ IngressFrame Boundary

```
Wire bytes (socket)
    │
    ▼
IpcFrameHeader::decode(bytes[0..24], max)     ←── frame.rs (decode order enforced)
    │
    ▼ IpcFrameHeader
frame_total_len(header)                        ←── server/helpers.rs
    │
    ▼ usize
extract_payload(&mut read_buffer, total_len)   ←── server/helpers.rs
    │
    ▼ Bytes
IpcFrame::new(header, payload_bytes, max)       ←── frame_types.rs
    │
    ▼ Result<IpcFrame, IpcError>
dispatch_command_with_resolver(...)             ←── server/dispatch.rs
    │
    ▼ IpcPayload
IngressFrame::new(run_id, workflow, payload, max) ←── ingress.rs
    │
    ▼ Result<IngressFrame, IpcError>
MemoryIngress::try_submit(frame)                ←── ingress.rs
    │
    ▼ Result<(), IpcError> (Full / Disconnected)
```

## Constants That Define the Wire Protocol

| Constant | Value | Type |
|----------|-------|------|
| `IPC_MAGIC` | `0x5642_4C54` | u32 |
| `IPC_VERSION` | `1` | u16 |
| `IPC_HEADER_LEN` | `24` | usize |
| `MaxPayloadBytes::DEFAULT` | `1_048_576` | usize |

**Immutability contract:** These constants are hardcoded protocol invariants.
Changing them constitutes a protocol version bump, not a bug fix.
