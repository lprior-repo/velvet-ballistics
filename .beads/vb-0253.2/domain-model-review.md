# Domain Model Review: vb-0253.2

## Model Summary

The vb_ipc domain model comprises four bounded-context modules that form the IPC layer for Velvet Ballistics. The facade conversion refactor does not introduce new domain concepts — it reorganizes existing canonical module definitions behind a re-export facade.

## Bounded Context: IPC Memory Ingress

### Canonical Types and Their Invariants

#### `QueueCapacity` (bounded.rs, line 12)
- **Type**: `pub struct QueueCapacity(NonZeroUsize)` — transparent newtype over `NonZeroUsize`
- **Domain invariant**: Value is always non-zero (enforced by `NonZeroUsize` type)
- **Factory**: `QueueCapacity::new(NonZeroUsize) -> Self` — const, no fallibility
- **Observer**: `QueueCapacity::get() -> usize` — exposes inner value
- **Canonical module**: `bounded.rs`; `lib.rs` lines 654–668 are verbatim duplicates to be removed

#### `MaxPayloadBytes` (bounded.rs, line 28)
- **Type**: `pub struct MaxPayloadBytes(NonZeroUsize)` — transparent newtype
- **Domain invariant**: Value is always non-zero; `DEFAULT = 1 MiB` (1_048_576 bytes)
- **Factory**: `MaxPayloadBytes::new(NonZeroUsize) -> Self` — const, no fallibility
- **Observer**: `MaxPayloadBytes::get() -> usize`
- **Canonical module**: `bounded.rs`; `lib.rs` lines 670–690 are verbatim duplicates to be removed

#### `BoundedPayload` (bounded.rs, line 49)
- **Type**: `pub struct BoundedPayload(Bytes)` — opaque wrapper around a byte buffer
- **Domain invariant**: `self.0.len() <= max_bytes` where `max_bytes` was the bound at construction time (parse-don't-validate pattern — the size check is in `new()`)
- **Factory**: `BoundedPayload::new(Bytes, MaxPayloadBytes) -> Result<Self, IpcError::PayloadTooLarge>` — fallible; enforces the bounded invariant at construction
- **Observer**: `BoundedPayload::bytes() -> &Bytes` — returns shared reference to inner buffer
- **Parse-don't-validate**: The `new()` constructor rejects oversized payloads rather than normalizing them
- **Canonical module**: `bounded.rs`; `lib.rs` lines 693–714 are verbatim duplicates to be removed

#### `IngressFrame` (ingress.rs, line 14)
- **Type**: `pub struct IngressFrame { run_id: RunId, workflow: WorkflowDigest, payload: BoundedPayload }`
- **Domain invariant**: `payload` is already bounded by construction (enforced by `BoundedPayload::new` in `IngressFrame::new`)
- **Factory**: `IngressFrame::new(RunId, WorkflowDigest, Bytes, MaxPayloadBytes) -> Result<Self, IpcError>` — fallible via `BoundedPayload::new` call
- **Observers**: `run_id()`, `workflow()`, `payload()` — all `#[must_use]`
- **Canonical module**: `ingress.rs`; `lib.rs` lines 716–756 are verbatim duplicates to be removed

#### `MemoryIngress` (ingress.rs, line 56)
- **Type**: `pub struct MemoryIngress { sender: Sender<IngressFrame>, receiver: Receiver<IngressFrame> }`
- **Domain invariant**: Bounded MPSC queue backed by `crossbeam_channel::bounded(capacity.get())`
- **Factory**: `MemoryIngress::bounded(QueueCapacity) -> Self`
- **Operations**:
  - `try_submit(frame) -> Result<(), IpcError::Full | IpcError::Disconnected>` — non-blocking; maps channel `Full`/`Disconnected` to IPC errors
  - `try_recv() -> Result<Option<IngressFrame>, IpcError::Disconnected>` — non-blocking; returns `Ok(None)` on empty, `Err(Disconnected)` on closed channel
  - `len() -> usize` — approximate queue depth (backed by `crossbeam_channel`)
  - `is_empty() -> bool`
- **Canonical module**: `ingress.rs`; `lib.rs` lines 758–798 are verbatim duplicates to be removed

#### `IpcError` (error.rs, line 9)
- **Type**: `pub enum IpcError` with 14 variants
- **Variants**: `Full`, `Disconnected`, `PayloadTooLarge { actual, limit }`, `InvalidMagic { actual }`, `UnsupportedVersion { actual }`, `UnknownCommand(u16)`, `ReservedNonZero { actual }`, `PayloadLengthMismatch { header, actual }`, `HeaderEncodeFailed`, `HeaderDecodeFailed`, `PayloadLengthOutOfRange { actual }`, `PayloadEncodeFailed`, `PayloadDecodeFailed`, `ResponseDecodeFailed`
- **Methods**: `diagnostic_code() -> DiagnosticCode`, `runtime_code() -> Option<&'static str>`
- **Canonical module**: `error.rs`; `lib.rs` lines 800–946 are verbatim duplicates to be removed

## Cross-Cutting Concerns

### Parse-Don't-Validate Pattern
`BoundedPayload::new` and `IngressFrame::new` apply the parse-don't-validate pattern: fallible construction rejects invalid inputs rather than normalizing them. This is the core defensive contract for the IPC layer.

### Error Mapping
`crossbeam_channel` errors (`TrySendError::Full`, `TrySendError::Disconnected`, `TryRecvError::Empty`, `TryRecvError::Disconnected`) are mapped to domain `IpcError` variants at the `MemoryIngress` boundary. This prevents channel-specific errors from leaking into the domain layer.

### Facade Contract
After the facade conversion, `lib.rs` must:
1. Declare `pub mod bounded; pub mod ingress; pub mod error;` to expose canonical modules
2. Re-export all canonical types so `vb_ipc::MemoryIngress`, `vb_ipc::IngressFrame`, `vb_ipc::QueueCapacity`, `vb_ipc::MaxPayloadBytes`, `vb_ipc::BoundedPayload`, `vb_ipc::IpcError`, `vb_ipc::encode_payload`, `vb_ipc::decode_payload` all resolve from the facade

### What Is NOT Changed
- `frame.rs` — `IpcFrame` (distinct from `IngressFrame`)
- `frame_types.rs` — `IpcFrameHeader`
- `payloads.rs` — `IpcPayload`, `SubmitRunPayload`
- `commands.rs` — `IpcCommand`
- `codec.rs` — canonical source (already correct)
- `client/`, `server/`, `action_output/`, `ids/`, `metrics/` modules

## Review Findings

1. **Duplicate definitions confirmed**: lib.rs lines 641–960 contain exact duplicates of bounded.rs, ingress.rs, error.rs definitions. Removal is safe and correct.
2. **map_try_send dead code**: lib.rs lines 955–960 define `map_try_send` which is used only by the duplicate `MemoryIngress::try_submit` in lib.rs (line 775). After dedupe, `ingress.rs` version inlines the match directly, making `map_try_send` unused.
3. **u32_to_usize duplicate**: lib.rs lines 948–953 duplicate `error.rs` `pub(crate)` version. The `error.rs` version is authoritative.
4. **No module declarations**: `bounded`, `ingress`, `error` are file-modules (not path-modules) and are not declared in lib.rs at all, making them inaccessible to external callers currently.
5. **All invariants already tested**: The existing test suite in `tests.rs` (60+ tests) covers all domain invariants.
