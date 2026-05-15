# Contract Specification: vb-0253.2 — vb_ipc Facade Conversion

## Context

- **bead**: vb-0253.2
- **phase**: State 3 — Contract and type model
- **change**: facade-conversion — remove duplicate definitions in `lib.rs`, promote `bounded`/`ingress`/`error` to `pub mod`, preserve all downstream re-exports
- **domain terms**: QueueCapacity, MaxPayloadBytes, BoundedPayload, IngressFrame, MemoryIngress, IpcError, encode_payload, decode_payload
- **assumptions**: The refactor is behavior-preserving; no semantic changes to any function or type. The 23 delivery scope rows define the complete scope.
- **open questions**: None

## Preconditions

- **PRE-001**: `lib.rs` lines 641–960 contain verbatim duplicates of `bounded.rs`, `ingress.rs`, `error.rs` definitions that must be removed
- **PRE-002**: `bounded`, `ingress`, and `error` modules are not declared in `lib.rs` module declarations (lines 15–17 only list `client`, `frame`, `server`)

## Postconditions

- **POST-001**: `lib.rs` declares `pub mod bounded; pub mod ingress; pub mod error;` module declarations
- **POST-002**: `lib.rs` re-exports all canonical types via `pub use bounded::{QueueCapacity, MaxPayloadBytes, BoundedPayload}; pub use ingress::{IngressFrame, MemoryIngress}; pub use error::IpcError;`
- **POST-003**: `lib.rs` re-exports codec functions via `pub use codec::{encode_payload, decode_payload};`
- **POST-004**: `lib.rs` no longer contains duplicate definitions for QueueCapacity, MaxPayloadBytes, BoundedPayload, IngressFrame, MemoryIngress, IpcError (lines 641–960 deleted)
- **POST-005**: `map_try_send` helper (lib.rs lines 955–960) is removed — unused after dedupe
- **POST-006**: `u32_to_usize` duplicate (lib.rs lines 948–953) is removed — `error.rs` version is authoritative
- **POST-007**: `tests.rs` imports updated from `crate::` to `crate::bounded::`, `crate::ingress::`, `crate::error::`, `crate::codec::` for renamed symbols

## Invariants

- **INV-001 (one-canonical-MemoryIngress)**: Exactly one `MemoryIngress` struct definition exists; `ingress.rs` is authoritative. Verified by absence of a second definition in `lib.rs` after facade conversion.
- **INV-002 (one-canonical-IngressFrame)**: Exactly one `IngressFrame` struct definition exists; `ingress.rs` is authoritative.
- **INV-003 (one-canonical-QueueCapacity)**: Exactly one `QueueCapacity` struct definition exists; `bounded.rs` is authoritative.
- **INV-004 (one-canonical-MaxPayloadBytes)**: Exactly one `MaxPayloadBytes` struct definition exists; `bounded.rs` is authoritative.
- **INV-005 (one-canonical-BoundedPayload)**: Exactly one `BoundedPayload` struct definition exists; `bounded.rs` is authoritative.
- **INV-006 (stable-re-exports)**: All public `vb_ipc` symbols remain re-exported from `lib.rs` facade; downstream imports do not break.
- **INV-007 (bounded-memory-invariant)**: `MemoryIngress` remains bounded; `Full`/`Disconnected`/`Empty` behavior unchanged by facade conversion.
- **INV-008 (payload-validation-invariant)**: Payload size validation remains parse-don't-validate; oversized payloads map to `IpcError::PayloadTooLarge`.
- **INV-009 (no-duplicate-IpcError)**: Exactly one `IpcError` enum definition exists; `error.rs` is authoritative.
- **INV-010 (no-unsafe)**: All vb_ipc files remain `#![forbid(unsafe_code)]`; no unsafe introduced.
- **INV-011 (no-concurrency-change)**: `MemoryIngress` crossbeam_channel usage unchanged; no new concurrency patterns.

## Error Taxonomy

This refactor does not introduce new error variants. The existing `IpcError` enum (14 variants in `error.rs`) is the canonical source:

| Variant | Semantic | Trigger |
|---|---|---|
| `IpcError::Full` | Queue at capacity | `MemoryIngress::try_submit` when `crossbeam_channel` is full |
| `IpcError::Disconnected` | Channel closed | Sender dropped or receiver dropped |
| `IpcError::PayloadTooLarge { actual, limit }` | Payload exceeds bound | `BoundedPayload::new` or `IngressFrame::new` size check |
| `IpcError::InvalidMagic { actual }` | Wire magic mismatch | `IpcFrameHeader::decode` |
| `IpcError::UnsupportedVersion { actual }` | Unsupported schema | `IpcFrameHeader::decode` |
| `IpcError::UnknownCommand(u16)` | Command id not in v1 set | `IpcCommand::from_u16` |
| `IpcError::ReservedNonZero { actual }` | Reserved header field non-zero | `IpcFrameHeader::decode` |
| `IpcError::PayloadLengthMismatch { header, actual }` | Header/bytes disagreement | `decode_frame` |
| `IpcError::HeaderEncodeFailed` | Wire encoding failure | `IpcFrameHeader::encode` |
| `IpcError::HeaderDecodeFailed` | Wire decoding failure | `IpcFrameHeader::decode` |
| `IpcError::PayloadLengthOutOfRange { actual }` | u32 does not fit usize | `u32_to_usize` |
| `IpcError::PayloadEncodeFailed` | Postcard encoding failure | `encode_payload` |
| `IpcError::PayloadDecodeFailed` | Postcard decoding failure | `decode_payload` |
| `IpcError::ResponseDecodeFailed` | Postcard response decoding failure | frame decode |

## Contract Signatures

```rust
// bounded.rs (canonical source)
pub struct QueueCapacity(NonZeroUsize);
impl QueueCapacity {
    pub const fn new(value: NonZeroUsize) -> Self;
    pub(crate) fn get(self) -> usize;
}

pub struct MaxPayloadBytes(NonZeroUsize);
impl MaxPayloadBytes {
    pub const DEFAULT: Self;
    pub const fn new(value: NonZeroUsize) -> Self;
    pub(crate) fn get(self) -> usize;
}

pub struct BoundedPayload(Bytes);
impl BoundedPayload {
    pub fn new(payload: Bytes, max: MaxPayloadBytes) -> Result<Self, IpcError>;
    pub const fn bytes(&self) -> &Bytes;
}

// ingress.rs (canonical source)
pub struct IngressFrame { run_id, workflow, payload }
impl IngressFrame {
    pub fn new(run_id, workflow, payload, max_payload) -> Result<Self, IpcError>;
    pub const fn run_id(&self) -> RunId;
    pub const fn workflow(&self) -> WorkflowDigest;
    pub const fn payload(&self) -> &BoundedPayload;
}

pub struct MemoryIngress { sender, receiver }
impl MemoryIngress {
    pub fn bounded(capacity: QueueCapacity) -> Self;
    pub fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError>;
    pub fn try_recv(&self) -> Result<Option<IngressFrame>, IpcError>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

// error.rs (canonical source)
pub enum IpcError { Full, Disconnected, PayloadTooLarge{actual,limit}, ... 14 variants }
impl IpcError {
    pub const fn diagnostic_code(&self) -> DiagnosticCode;
    pub const fn runtime_code(&self) -> Option<&'static str>;
}

// codec.rs (canonical source)
pub fn encode_payload(payload: &IpcPayload, max: MaxPayloadBytes) -> Result<BoundedPayload, IpcError>;
pub fn decode_payload(payload: &BoundedPayload) -> Result<IpcPayload, IpcError>;
```

## Facade Re-export Contract

After conversion, `lib.rs` facade must expose:

```rust
pub mod bounded;   // newly pub
pub mod ingress;   // newly pub
pub mod error;     // newly pub

pub use bounded::{QueueCapacity, MaxPayloadBytes, BoundedPayload};
pub use ingress::{IngressFrame, MemoryIngress};
pub use error::IpcError;
pub use codec::{encode_payload, decode_payload};
// All other public symbols unchanged (IpcCommand, IpcFrameHeader, IpcPayload, etc.)
```

## Verus-Owned Clauses

- **INV-007 (bounded-memory-invariant)**: `MemoryIngress` invariants (bounded channel capacity, FIFO ordering, disconnect semantics) are proven by the existing test suite in `tests.rs` and `crossbeam_channel` semantics. No Verus proof required for this refactor.
- **INV-008 (payload-validation-invariant)**: `BoundedPayload::new` parse-don't-validate contract is exercised by existing unit tests. No new proof obligations.

## TLA+-Owned Clauses

- **None** — this is a pure refactor with no temporal, workflow, protocol, scheduler, retry, claim/lease, concurrent, or distributed behavior changes. The `MemoryIngress` queue semantics are unchanged; `crossbeam_channel` is the trusted runtime component.

## Theorem-Owned Clauses

- **None** — no tiny algebraic theorem kernels, no protocol lattices, no arithmetic bounds beyond what the existing test suite covers.

## Non-goals

- No new behavioral features
- No TLA+/Verus/Kani/Loom/Miri proofs for this refactor
- No performance changes
- No API additions or removals (only re-arrangement for facade pattern)
- No changes to `frame.rs`, `frame_types.rs`, `payloads.rs`, `commands.rs`, `client/`, `server/`, `action_output/`, `ids/`, `metrics/`
