# VB IPC Workflow Model — Fragmented Frame and Oversize Message

## IPC Frame Decode State Machine

### States

| State | Definition |
|-------|------------|
| `WaitingHeader` | Fewer than 24 bytes in `read_buffer` |
| `HeaderDecoding` | Attempting `IpcFrameHeader::decode` on first 24 bytes of buffer |
| `WaitingPayload` | Header decoded, fewer than `frame_total_len` bytes in buffer |
| `PayloadReady` | `read_buffer.len() >= frame_total_len`, payload bytes extractable |
| `Dispatching` | Payload extracted, `dispatch_command_with_resolver` executing |
| `SendingResponse` | Response written to client `write_buffer` |
| `ClientDisconnected` | Client removed from server |

### Transitions

```
WAITING_HEADER
  │
  │ append_read_bytes → read_buffer.len() >= 24
  ▼
HEADER_DECODING ──► decode(&bytes, max) ──► InvalidHeader ──► SEND_ERROR ──► DISCONNECT
  │                                                       (any IpcError)
  │ Ok(header)
  ▼
frame_total_len(header) ≤ read_buffer.len() ?
  │
  ├─ YES ──► WAITING_PAYLOAD ──► extract_payload ──► DISPATCHING
  │                                        │
  │◄───────────────────────────────────────┘
  │
  └─ NO ──► WAITING_PAYLOAD ──► (return, wait for next poll)

DISPATCHING
  │
  │ dispatch_command_with_resolver(header, payload_bytes)
  ▼
SENDING_RESPONSE
  │
  │ send_response(...)
  ▼
WAITING_HEADER (loop for next frame on same connection)
```

## Partial Frame Scenarios

### Scenario 1: Partial Header (TCP Chunk Split)

```
Client sends: [0x54, 0x4C, 0x42] (3 bytes, partial magic)
Server poll:  read_buffer = [0x54, 0x4C, 0x42]
Condition:    read_buffer.len() < IPC_HEADER_LEN (24)
Result:       WAITING_HEADER → return Ok(false) → wait for next poll
```

**Guarantee:** Server does NOT attempt to decode incomplete header bytes.

### Scenario 2: Header Complete, Payload Fragmented

```
Client sends: 24-byte valid header + 0 payload bytes
Server poll:  read_buffer.len() == 24
Header decode: Ok(header) with payload_len = 4096
frame_total_len = 24 + 4096 = 4120
Condition:    read_buffer.len() (24) < frame_total_len (4120)
Result:       WAITING_PAYLOAD → return Ok(false) → wait for next poll
```

**Guarantee:** Server reads only the actual bytes buffered; does NOT pre-allocate 4096 bytes.

### Scenario 3: Two Polls to Complete Frame

```
Poll 1: read_buffer.len() == 24, header decoded, waiting for 4096 bytes total
Poll 2: read_buffer.len() >= 4120, payload extracted, dispatched
```

**Guarantee:** Frame is fully received before `dispatch_command_with_resolver` is called.

### Scenario 4: Slow Client Oversize Frame (Disconnection)

```
Client sends: 24-byte header with payload_len = 1_048_577 (one over 1 MiB default)
Server poll: read_buffer.len() == 24
Header decode: Err(PayloadTooLarge { actual: 1048577, limit: 1048576 })
Result:       frame_error_response sent, client disconnected, read_buffer dropped
```

**Guarantee:** Server never attempts to read the oversize payload bytes from the socket.

## Oversize Message Workflow

```
Client declares: payload_len = u32::MAX
Server receives: header bytes only
Header decode:  PayloadTooLarge checked AFTER magic/version/command/reserved
                → error response sent → client disconnected
```

**Key point:** The u32→usize conversion fails with `PayloadLengthOutOfRange` before the
`PayloadTooLarge` check is reached when `payload_len > usize::MAX as u32` on 32-bit targets.

## Frame Pipelining

The server processes frames in a `while` loop per poll event:

```
while read_buffer.len() >= IPC_HEADER_LEN:
    decode header
    if error: send error response, return (disconnect)
    compute frame_total_len
    if read_buffer.len() < frame_total_len: return (partial)
    extract payload
    dispatch
    send response
    continue loop  ← next frame in same buffer
```

**Guarantee:** Multiple complete frames in a single `read()` syscall are each dispatched
in order before returning to the event loop.

## Pipelined-Frame Workflow

```
Client sends: frame1 (24+N bytes) + frame2 (24+M bytes) in one write()
Server poll:  read_buffer = [frame1][frame2]
Loop:        frame1 extracted and dispatched
             frame2 extracted and dispatched
             read_buffer cleared
```

## Error Response Workflow

For any header decode error:
1. `frame_error_response(error)` builds an `IpcResponse::FrameError` payload
2. Response header uses `IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0)` as fallback
3. Response payload contains the error message
4. Response is sent via `send_response`
5. Client is disconnected (return `Ok(true)` from `handle_readable`)

## Bounded Read Contract

```rust
// handle_readable pseudo-code
let mut temp_buf = [0u8; READ_CHUNK_BYTES]; // 4096 bytes max per poll
let bytes_read = client.stream.read(&mut temp_buf)?;
append_read_bytes(&mut client.read_buffer, &temp_buf, bytes_read)?;
// NEVER: Vec::with_capacity(header.payload_len)
```

**Bounded property:** At most `READ_CHUNK_BYTES * N` bytes can accumulate in
`read_buffer` where N is the number of poll events before processing. The
`frame_total_len` guard gates actual dispatch.
