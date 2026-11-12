# VB IPC Contract — Fowler/Wlaschin Style

## Contract: `IpcFrameHeader::decode`

### Preconditions
- `bytes` is a 24-byte slice (compile-time enforced by type signature `[u8; IPC_HEADER_LEN]`)
- `max_payload` is a valid `MaxPayloadBytes` (non-zero, compile-time enforced)

### Postconditions

**P1 — Magic First:**
```fsharp
decode(bytes, max).is_err() ∧ err == InvalidMagic { actual }
  ⇒ bytes[0..4].read_u32_le() != IPC_MAGIC
```

**P2 — Version After Magic:**
```fsharp
decode(bytes, max) == Err(UnsupportedVersion { actual: v })
  ⇒ bytes[0..4].read_u32_le() == IPC_MAGIC  // magic passed
  ∧ v != IPC_VERSION
```

**P3 — Command After Version:**
```fsharp
decode(bytes, max) == Err(UnknownCommand(id))
  ⇒ bytes[0..4].read_u32_le() == IPC_MAGIC
  ∧ bytes[4..6].read_u16_le() == IPC_VERSION
  ∧ id ∉ {1..16}
```

**P4 — Reserved Before Payload:**
```fsharp
decode(bytes, max) == Err(ReservedNonZero { actual: r })
  ⇒ bytes[0..4].read_u32_le() == IPC_MAGIC
  ∧ bytes[4..6].read_u16_le() == IPC_VERSION
  ∧ bytes[6..8].read_u16_le() ∈ {1..16}
  ∧ r ≠ 0
```

**P5 — PayloadLen Bound After All Structural Checks:**
```fsharp
decode(bytes, max) == Err(PayloadTooLarge { actual: a, limit: l })
  ⇒ bytes[0..4].read_u32_le() == IPC_MAGIC
  ∧ bytes[4..6].read_u16_le() == IPC_VERSION
  ∧ bytes[6..8].read_u16_le() ∈ {1..16}
  ∧ bytes[10..12].read_u16_le() == 0
  ∧ usize.from(bytes[20..24].read_u32_le()) == a
  ∧ a > l
  ∧ l == max.get()
```

**P6 — Ok Result Contains All Fields:**
```fsharp
decode(bytes, max) == Ok(h)
  ⇒ h.command == IpcCommand::from_u16(bytes[6..8].read_u16_le()).unwrap()
  ∧ h.flags == bytes[8..10].read_u16_le()
  ∧ h.correlation == bytes[12..20].read_u64_le()
  ∧ h.payload_len == bytes[20..24].read_u32_le()
  ∧ usize.from(h.payload_len) <= max.get()
```

---

## Contract: `encode_frame`

### Preconditions
- `payload.len() <= MaxPayloadBytes::DEFAULT.get()` (caller invariant)
- `payload.len()` fits in `u32` (caller invariant)

### Postconditions

**E1 — Header Contains Magic:**
```fsharp
encode_frame(cmd, flags, corr, payload) == Ok(frame)
  ⇒ frame[0..4] == IPC_MAGIC.to_le_bytes()
```

**E2 — Header Contains Version:**
```fsharp
encode_frame(cmd, flags, corr, payload) == Ok(frame)
  ⇒ frame[4..6] == IPC_VERSION.to_le_bytes()
```

**E3 — Header Contains Command:**
```fsharp
encode_frame(cmd, flags, corr, payload) == Ok(frame)
  ⇒ frame[6..8] == cmd.as_u16().to_le_bytes()
```

**E4 — Header Contains Payload Length:**
```fsharp
encode_frame(cmd, flags, corr, payload) == Ok(frame)
  ⇒ u32::from_le_bytes(frame[20..24]) == payload.len() as u32
```

**E5 — Frame Layout:**
```fsharp
encode_frame(cmd, flags, corr, payload) == Ok(frame)
  ⇒ frame.len() == IPC_HEADER_LEN + payload.len()
  ∧ frame[IPC_HEADER_LEN..] == payload
```

---

## Contract: Frame Decode Order

### Theorem: Decode Order is Total and Ordered

For all `bytes: [u8; 24]` and `max: MaxPayloadBytes`:

```
STEP 1: magic = bytes[0..4].read_u32_le()
  if magic ≠ IPC_MAGIC → return InvalidMagic

STEP 2: version = bytes[4..6].read_u16_le()
  if version ≠ IPC_VERSION → return UnsupportedVersion

STEP 3: command_id = bytes[6..8].read_u16_le()
  if command_id ∉ {1..16} → return UnknownCommand(command_id)

STEP 4: reserved = bytes[10..12].read_u16_le()
  if reserved ≠ 0 → return ReservedNonZero

STEP 5: correlation = bytes[12..20].read_u64_le()  // no failure possible

STEP 6: payload_len_u32 = bytes[20..24].read_u32_le()
  if usize::try_from(payload_len_u32).is_err() → return PayloadLengthOutOfRange
  if payload_len_usize > max.get() → return PayloadTooLarge

STEP 7: return Ok(IpcFrameHeader { command, flags, correlation, payload_len })
```

**Corollaries:**
- `InvalidMagic` is returned for any bytes where step 1 fails, regardless of later fields
- `UnsupportedVersion` is returned only when step 1 passes but step 2 fails
- `ReservedNonZero` is returned only when steps 1-3 pass but step 4 fails
- `PayloadTooLarge` is returned only when steps 1-5 pass but step 6 fails

---

## Contract: Partial Frame Server Behavior

### Invariant: Read Buffer Bounding

```
handle_readable reads at most READ_CHUNK_BYTES (4096) bytes per call
  ⇒ read_buffer.len() ≤ READ_CHUNK_BYTES * polls_since_connect
```

### Invariant: No Pre-Allocation

```
header.payload_len is NOT used to allocate a Vec before header decode completes
  ⇒ Vec allocation uses actual bytes read from socket, not declared payload_len
```

### Invariant: Frame Complete Before Dispatch

```
dispatch_command_with_resolver(header, payload_bytes) is called
  ⇒ read_buffer.len() >= IPC_HEADER_LEN + header.payload_len
```

---

## Contract: Oversize Payload Rejection

### Theorem: Header-Only Oversize Rejection

```
Given: header_bytes with payload_len = P where P > MaxPayloadBytes::DEFAULT.get()
When: IpcFrameHeader::decode(header_bytes, MaxPayloadBytes::DEFAULT)
Then: Result == Err(PayloadTooLarge { actual: P, limit: MaxPayloadBytes::DEFAULT.get() })
  AND: no payload bytes are read from socket
  AND: server disconnects client after sending error response
```

---

## Railway: Frame Processing

```
                    ┌─────────────────┐
                    │ bytes available │
                    └────────┬────────┘
                             │
              ┌──────────────▼──────────────┐
              │ read_buffer.len() >= 24 ?   │
              └──────────────┬──────────────┘
                    YES      │       NO
              ┌──────────────▼──────────────┐
              │ decode header (total fn)    │
              └──────────────┬──────────────┘
                    ┌────────▼────────┐
                    │ Ok(header) ?     │
                    └────────┬────────┘
              YES      │       │      NO
        ┌──────────────▼┐     │
        │ payload_len ≤ │     │
        │ max ?         │     │
        └────────┬──────┘     │
          YES    │   NO       │
    ┌───────────▼──┐  ┌──────▼────────────┐
    │ frame_total  │  │ send error,       │
    │ _len ≤ buf?  │  │ disconnect client │
    └───────┬──────┘  └───────────────────┘
      YES   │   NO
  ┌─────────▼──────────┐
  │ extract payload     │
  │ dispatch           │
  │ send response      │
  │ continue loop      │
  └────────────────────┘
```
