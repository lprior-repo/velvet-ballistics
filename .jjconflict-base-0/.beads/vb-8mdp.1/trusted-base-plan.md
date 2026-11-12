# Trusted Base Plan — vb-8mdp.1

## Trusted Surfaces

### 1. Rust Standard Library (trusted)
- `std::io::Read::read_exact` — partial reads return `Err(IoError)`, never panic
- `byteorder::LittleEndian::read_u32_le`, `read_u16_le` — guaranteed safe on `[u8; N]` slices
- `postcard::from_bytes` — returns `Result` for malformed input, never panics
- `std::io::Cursor` — read wrapper, no panics

**Justification**: Standard library, well-tested, no unsafe code in this crate.

### 2. IPC Protocol Constants (trusted — compile-time enforced)
- `IPC_HEADER_LEN = 24` — enforced by type signature `[u8; 24]`
- `IPC_MAGIC = 0x5642_4C54` — hardcoded constant, no runtime computation
- `IPC_VERSION = 1` — hardcoded constant
- `MaxPayloadBytes::DEFAULT = 1_048_576` — `NonZeroUsize`, zero is not representable

**Justification**: These are `const` values; the Rust compiler enforces their values at compile time.

### 3. Type Invariants (trusted — enforced by type system)
- `MaxPayloadBytes` wraps `NonZeroUsize` — zero cannot be represented
- `IpcCommand` is a closed enum with 16 variants — `from_u16` is total for 1..16
- `IpcFrameHeader` fields are all primitive types (u16, u64, u32)

**Justification**: Type system prevents illegal states; no runtime check needed.

### 4. byteorder ReadBytesExt (trusted — external crate)
- `read_u32::<LittleEndian>` on `[u8; 4]` cannot panic (slices are fixed size)
- `read_u16::<LittleEndian>` on `[u8; 2]` cannot panic

**Justification**: byteorder is a well-established crate; reads from fixed-size slices are safe.

## Model Reductions and Assumptions

### Bounded State Space (TLA+)
- `READ_CHUNK_BYTES = 4096` — hard limit per poll event, modeled as constant
- Buffer sizes are bounded: `read_buffer.len() <= 4096 * polls_since_connect`
- State machine: `WaitingHeader | WaitingPayload | Dispatching | Disconnected`
- Max payload: `MaxPayloadBytes::DEFAULT = 1_048_576` (1 MiB)

**Justification**: In practice, `polls_since_connect` is bounded by connection timeout; TLA+ model uses this finite bound.

### No Concurrency (TLA+/Loom)
- Server is single-threaded; no concurrent buffer access
- No lock-free structures in IPC path
- No atomic operations

**Justification**: The IPC server uses blocking I/O on a single thread; concurrency is handled at a higher layer.

### Kani Symbolic Execution Bounds
- Exhaust all `2^192` inputs via `kani::any()` on `[u8; 24]`
- Symbolic execution terminates because decode is a total function with no loops
- `#[kani::unwind(4)]` for decode order harnesses (4 nested error checks)

**Justification**: `kani::any()` generates all possible 24-byte combinations symbolically; decode has no recursion or loops.

### Verus Ghost Model Binding
- Spec function encodes decode steps as ordered sequence
- Exec function proven equivalent to spec via `requires`/`ensures`
- Step indexing: step 1 = magic, step 2 = version, ..., step 6 = payload_len

**Justification**: Ghost model must bind to actual Rust implementation; no vacuum proofs allowed.

## Known Assumptions and Stub Boundaries

| Assumption | Location | Justification |
|-----------|----------|---------------|
| byteorder reads never panic on fixed-size array | `frame.rs:4` | byteorder guarantees; `[u8; 24]` has fixed layout |
| `from_u16` total for 1..16 | `commands.rs` | Closed enum, match is exhaustive |
| `Vec::with_capacity` only called after header decode | server loop | Code review + TLA+ invariant |
| READ_CHUNK_BYTES = 4096 | server constant | Hardcoded, no runtime config |
| Single-threaded server | server architecture | Async but single-task per connection |
| No overflow in u32→u64 conversion for correlation | `frame.rs` | correlation is u64 from bytes[12..20], always valid |

## Non-Behavior Waivers (not applicable)
No waivers requested. All proof obligations address genuine behavior requirements.

## Reduction Justification

The TLA+ model reduces the infinite socket I/O to a finite state machine with bounded buffer sizes. This is justified because:
1. `READ_CHUNK_BYTES` is a hard cap enforced by the server loop
2. Payload reads only occur after header decode succeeds (verified by invariant)
3. Disconnect happens immediately on header decode error (verified by state transition)

The Kani exhaustiveness (`2^192`) is justified because:
1. The decode function has no loops or recursion
2. All reads are from fixed-size arrays
3. The function is total (returns `Result` for all inputs)