# Contract Specification: vb-te1i Binary IPC BDD Acceptance

## Context

- **Bead**: vb-te1i
- **Feature**: bdd: Binary IPC acceptance scenarios
- **Protocol**: 24-byte fixed frame, Unix domain socket, 16 v1 commands
- **Domain terms**:
  - `IpcFrameHeader` — fixed 24-byte wire header (magic[4] + version[2] + command[2] + flags[2] + reserved[2] + correlation[8] + payload_len[4])
  - `IpcFrame` — header + bounded payload bytes
  - `IpcCommand` — u16 command ID (1..=16)
  - `IpcError` — 14-variant error taxonomy
  - `MaxPayloadBytes` — 1 MiB default bound
  - `QueueCapacity` — SPSC ingress queue capacity
  - `BoundedPayload` — post-size-check payload bytes
- **Assumptions**:
  - No authentication on Unix domain socket (OS-level socket permissions suffice)
  - IPC server runs in-process via `serve_ipc` with mio polling
  - All 16 commands are reachable via the public client API
  - Malformed frame rejection happens before any heap allocation
- **Open questions**:
  - None — domain model is fully resolved from existing source

---

## Preconditions

- **PRE-001**: Caller must provide a 24-byte header slice when calling `IpcFrameHeader::decode`.
- **PRE-002**: `max_payload` passed to decode must be > 0 (`NonZeroUsize` enforced by `MaxPayloadBytes`).
- **PRE-003**: Payload bytes supplied to `IpcFrame::new` must exactly match `header.payload_len` in byte count.

---

## Postconditions

- **POST-001** (`frame_roundtrip`): Encoding a valid `IpcFrameHeader` via `encode()` and then decoding the 24 bytes via `decode()` with the same `max_payload` produces a header structurally equal to the original.
- **POST-002** (`health_returns_healthy`): A frame with `command = Health`, `correlation = N`, and `payload_len = 0` decodes successfully and the server handler returns a response with `correlation = N`.
- **POST-003** (`shutdown_returns_shutting_down`): A frame with `command = Shutdown`, `correlation = N` decodes successfully and the server transitions to `ShuttingDown` state.
- **POST-004** (`submit_run_preserves_correlation`): A `SubmitRun` frame with `correlation = N` and a valid `SubmitRunPayload` returns a response frame with `correlation = N`.
- **POST-005** (`bad_magic_rejected_before_allocation`): Any 24-byte sequence whose first 4 bytes are not `0x5642_4C54` causes `decode()` to return `Err(InvalidMagic { actual })` **without** reading or allocating the payload region.
- **POST-006** (`version_mismatch_rejected`): Any frame with version field ≠ 1 causes `decode()` to return `Err(UnsupportedVersion { actual })`.
- **POST-007** (`unknown_command_rejected`): Any frame whose command field is not 1..=16 causes `decode()` to return `Err(UnknownCommand(n))`.
- **POST-008** (`reserved_nonzero_rejected`): Any frame whose reserved field (bytes 10-11) is non-zero causes `decode()` to return `Err(ReservedNonZero { actual })`.
- **POST-009** (`payload_too_large_rejected`): Any frame whose `payload_len` field exceeds `max_payload` causes `decode()` to return `Err(PayloadTooLarge { actual, limit })`.
- **POST-010** (`payload_length_mismatch_rejected`): When `IpcFrame::new` receives a payload whose byte count ≠ `header.payload_len`, it returns `Err(PayloadLengthMismatch { header, actual })`.
- **POST-011** (`backpressure_returns_full`): When `MemoryIngress::try_submit` is called on an ingress whose queue is at capacity, it returns `Err(IpcError::Full)` and does not drop the frame.
- **POST-012** (`queue_disconnected_returns_disconnected`): When `MemoryIngress::try_recv` is called on a disconnected ingress, it returns `Err(IpcError::Disconnected)`.

---

## Invariants

- **INV-001** (`header_length_fixed`): `IPC_HEADER_LEN == 24`. The wire layout is fixed and cannot change without a version bump.
- **INV-002** (`magic_value_immutable`): `IPC_MAGIC == 0x5642_4C54`. Any change is a protocol-breaking event.
- **INV-003** (`command_range`): Valid `IpcCommand` values are exactly the integers 1 through 16, no others.
- **INV-004** (`decode_before_alloc`): `IpcFrameHeader::decode` performs all header-field validations (magic, version, command, reserved, payload_len bounds) **before** any payload allocation can occur.
- **INV-005** (`bounded_payload_enforced`): A `BoundedPayload` always satisfies `self.bytes().len() <= max_payload.get()`.
- **INV-006** (`correlation_preserved`): Every successful decode/encode roundtrip preserves the `correlation` field exactly.
- **INV-007** (`diagnostic_code_stable`): Every `IpcError` variant has a stable `diagnostic_code()` return value that never changes across versions.

---

## Error Taxonomy

| Error Variant | Semantic Trigger | Recovery |
|---|---|---|
| `IpcError::Full` | Queue at capacity on `try_submit` | Caller applies backpressure |
| `IpcError::Disconnected` | Producer/consumer disconnected | Caller tears down session |
| `IpcError::PayloadTooLarge` | `payload_len > max_payload` | Caller reduces payload |
| `IpcError::InvalidMagic` | First 4 bytes ≠ `VBLT` LE | Frame is discarded |
| `IpcError::UnsupportedVersion` | version ≠ 1 | Protocol mismatch |
| `IpcError::UnknownCommand` | command not in 1..=16 | Unknown command |
| `IpcError::ReservedNonZero` | reserved field ≠ 0 | Protocol violation |
| `IpcError::PayloadLengthMismatch` | actual payload ≠ header.payload_len | Caller fixes serialization |
| `IpcError::HeaderEncodeFailed` | `write_*` fails on fixed buffer | Hard failure |
| `IpcError::HeaderDecodeFailed` | `read_*` fails on fixed slice | Hard failure |
| `IpcError::PayloadLengthOutOfRange` | u32 payload_len doesn't fit usize | Hard failure on 32-bit |
| `IpcError::PayloadEncodeFailed` | Postcard encoding fails | Caller fixes payload |
| `IpcError::PayloadDecodeFailed` | Postcard decoding fails | Caller fixes payload |
| `IpcError::ResponseDecodeFailed` | Response postcard decoding fails | Caller handles corrupted response |

---

## Contract Signatures

```rust
// Header decode — all fallible
fn IpcFrameHeader::decode(bytes: &[u8; 24], max_payload: MaxPayloadBytes) -> Result<IpcFrameHeader, IpcError>

// Frame decode — all fallible
fn decode_frame(header: &[u8; 24], payload: Bytes, max_payload: MaxPayloadBytes) -> Result<IpcFrame, IpcError>

// Payload bounds
fn BoundedPayload::new(payload: Bytes, max: MaxPayloadBytes) -> Result<BoundedPayload, IpcError>

// Queue ingress — all fallible
fn MemoryIngress::try_submit(frame: IngressFrame) -> Result<(), IpcError>
fn MemoryIngress::try_recv() -> Result<IngressFrame, IpcError>

// Client send/recv — all fallible
fn IpcClient::send_command<C: Serialize>(&self, command: IpcCommand, correlation: u64, payload: &C) -> Result<IpcResponse, IpcError>
fn IpcClient::recv_response(&self, correlation: u64) -> Result<IpcResponse, IpcError>
```

---

## Verus-Owned Clauses

- **INV-004** (`decode_before_alloc`): Pure decode function; no I/O, no mutable shared state. Verus can prove all header-field checks happen before any payload read.
- **INV-005** (`bounded_payload_enforced`): `BoundedPayload::new` is a pure constructor. Verus invariant: `BoundedPayload::bytes().len() <= max_payload.get()` always holds.
- **INV-006** (`correlation_preserved`): Pure encode/decode roundtrip. Verus can prove `encode().decode() == original` for all valid headers.
- **INV-003** (`command_range`): `IpcCommand::from_u16` is a total mapping from 1..=16; all other values return `UnknownCommand`. Verus can prove exhaustive match coverage.

---

## TLA+-Owned Clauses

**Non-applicability rationale**: This bead covers a binary IPC frame codec — a pure data-validation and serialization layer with no temporal, concurrent, workflow, or state-over-time behavior. There are no:
- State machines with transitions over time
- Concurrent sessions interacting
- Schedulers, queues with enqueue/dequeue ordering invariants
- Retry/lease/claim logic
- Liveness or fairness properties
- Deadlock possibilities (single-threaded decode/encode)

All behavior is bounded data transformation: `bytes → validated header → bounded frame`. The concurrency surface (mio server loop, SPSC queue) is exercised by integration tests and Loom, not TLA+.

**TLA+-Owned Clauses**: None.

---

## Theorem-Owned Clauses

**Non-applicability rationale**: All critical properties (bounds, invariants, no-panic on decode) are expressible in Verus. No algebraic protocol lattice, parser grammar theorem, or refinement claim exceeds Verus's scope.

**Theorem-Owned Clauses**: None.

---

## Non-goals

- TLA+ modeling of the mio event loop or Unix socket accept loop
- Verus proofs of the async `serve_ipc` server loop (async shell excluded from pure core)
- Proof of the SPSC queue's lock-free properties (Loom covers concurrent correctness)
- Performance benchmarking of the hot path (defined elsewhere in proof obligations)
