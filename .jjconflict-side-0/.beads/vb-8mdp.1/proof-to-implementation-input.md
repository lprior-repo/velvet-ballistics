# Proof-to-Implementation Input — vb-8mdp.1

Maps proof claims to Rust source refs, test refs, and exact evidence commands.

## VB-IPC-DECODE-001: decode is total over [u8; 24]

**Proof Claims**:
- `IpcFrameHeader::decode` returns `Result<Self, IpcError>` for all 2^192 inputs without panicking
- decode preserves all fields on success

**Rust Source Refs**:
- `crates/vb_ipc/src/frame_types.rs` — `IpcFrameHeader::decode` implementation
- `crates/vb_ipc/src/constants.rs` — `IPC_HEADER_LEN`, `IPC_MAGIC`, `IPC_VERSION` constants
- `crates/vb_ipc/src/commands.rs` — `IpcCommand` enum and `from_u16`
- `crates/vb_ipc/src/error.rs` — `IpcError` variants
- `crates/vb_ipc/src/bounded.rs` — `MaxPayloadBytes`

**Existing Harnesses**:
- `crates/vb_ipc/src/kani_ipc_header.rs`: `kani_ipc_header_decode_valid`, `kani_ipc_header_preserves_all_fields`

**New Harness**:
- `crates/vb_ipc/src/kani_ipc_decode_total.rs`: `kani_harness_decode_total_fn`
  - Evidence: `cd /home/lewis/src/velvet-ballistics && cargo kani -p vb_ipc --crate-type=lib 2>&1 | tee artifacts/kani-decode-total.log`

**New Verus Spec**:
- `verification/verus/ipc_decode_order.vir`: `verus_ipc_decode_order_spec`, `verus_ipc_decode_order_proof`
  - Evidence: `cd /home/lewis/src/velvet-ballistics && cargo verus -p vb_ipc --crate-type=lib 2>&1 | tee artifacts/verus-decode.log`

**Behavior Tests**:
- `crates/vb_ipc/src/tests.rs`: `fuzz_decode_frame_rejects_short_input`, `adversarial_all_zero_bytes_header_rejected_as_bad_magic`

---

## VB-IPC-DECODE-002/003/004: Decode Order Theorems

**Proof Claims**:
- STEP 1 (magic) gates all other checks — InvalidMagic returned before UnsupportedVersion
- STEP 2 (version) gates STEP 3 (command) — UnsupportedVersion only when magic valid
- STEP 3 (command) gates STEP 4 (reserved) — UnknownCommand only when magic+version valid
- STEP 4 (reserved) gates STEP 6 (payload_len) — ReservedNonZero before PayloadTooLarge

**Rust Source Refs**:
- `crates/vb_ipc/src/frame_types.rs` — decode order implemented as nested if-let chains

**Existing Harnesses**:
- `crates/vb_ipc/src/kani_ipc_decode_order.rs`: `kani_harness_ipc_decode_order`, `kani_harness_ipc_reserved_nonzero_before_payload_len`, `kani_harness_ipc_magic_before_version`

**New Harness (VB-IPC-DECODE-004 command→reserved)**:
- `crates/vb_ipc/src/kani_ipc_decode_order.rs`: `kani_harness_decode_order_command_before_reserved`
  - Evidence: `cd /home/lewis/src/velvet-ballistics && cargo kani -p vb_ipc --crate-type=lib 2>&1 | tee artifacts/kani-decode-order.log`

**New Verus Spec**:
- `verification/verus/ipc_decode_order.vir`: `verus_ipc_decode_order_proof` for version→command and command→reserved ordering
  - Evidence: `cd /home/lewis/src/velvet-ballistics && cargo verus -p vb_ipc --crate-type=lib 2>&1 | tee artifacts/verus-decode.log`

---

## VB-IPC-SERVER-002: No Pre-Budget Allocation

**Proof Claims**:
- `Vec::with_capacity(payload_len)` is NEVER called before header decode succeeds
- Allocation uses actual bytes read from socket, not declared `payload_len`

**Rust Source Refs**:
- `crates/vb_ipc/src/server/*.rs` — `handle_readable` server loop
- `crates/vb_ipc/src/frame.rs` — `read_frame_payload` which does call `vec![0u8; payload_len]` AFTER header decode

**Evidence (TLA+)**:
- `verification/tla+/IPCServerFragmentation.tla`
  - Evidence: `cd /home/lewis/src/velvet-ballistics/verification/tla+ && java -cp tla2tools.jar tlc2.TLC IPCServerFragmentation 2>&1 | tee artifacts/tla-fragment.log`

**Code Review** (secondary):
- `crates/vb_ipc/src/frame.rs:109-122` — `read_frame_payload` uses `vec![0u8; payload_len]` only after header already decoded
- Server loop checks `read_buffer.len() >= IPC_HEADER_LEN` before calling decode

---

## VB-IPC-SERVER-003: Oversize Rejection Without Payload Read

**Proof Claims**:
- When `IpcFrameHeader::decode` returns `PayloadTooLarge`, server disconnects without reading any payload bytes
- No `read_exact` on payload bytes for oversized frames

**Rust Source Refs**:
- `crates/vb_ipc/src/server/*.rs` — error path handling in `handle_readable`
- `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` — decode-level proof

**New Harness (decode-level)**:
- `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs`: `kani_ipc_header_rejects_oversize_before_payload_read`
  - Evidence: `cd /home/lewis/src/velvet-ballistics && cargo kani -p vb_ipc --crate-type=lib 2>&1 | tee artifacts/kani-oversize.log`

**New TLA+ (server-level)**:
- `verification/tla+/IPCOversizeRejection.tla`
  - Evidence: `cd /home/lewis/src/velvet-ballistics/verification/tla+ && java -cp tla2tools.jar tlc2.TLC IPCOversizeRejection 2>&1 | tee artifacts/tla-oversize.log`

**Behavior Tests**:
- `crates/vb_ipc/src/server/impl_tests.rs`: `slow_client_oversized_frame_disconnects_without_unbounded_growth`

---

## VB-IPC-FRAGMENT-001/002: Partial Frame Waiting

**Proof Claims**:
- Server stays in `WaitingHeader` when `read_buffer.len() < 24` (no error)
- Server stays in `WaitingPayload` when header valid but `read_buffer.len() < frame_total_len` (no allocation)

**Rust Source Refs**:
- `crates/vb_ipc/src/server/*.rs` — `handle_readable` while loop

**New TLA+**:
- `verification/tla+/IPCServerFragmentation.tla`
  - Evidence: `cd /home/lewis/src/velvet-ballistics/verification/tla+ && java -cp tla2tools.jar tlc2.TLC IPCServerFragmentation 2>&1 | tee artifacts/tla-fragment.log`

**New Proptest**:
- `crates/vb_ipc/src/server/impl_tests.rs`: `proptest_partial_header_waits`, `proptest_partial_payload_waits`
  - Evidence: `cd /home/lewis/src/velvet-ballistics && cargo test -p vb_ipc --release -- server_partial 2>&1 | tee artifacts/proptest-fragment.log`

**Behavior Tests (existing)**:
- `crates/vb_ipc/src/tests.rs`: `server_waits_for_complete_frame_when_partial_sent`
- `crates/vb_ipc/src/server/impl_tests.rs`: `slow_client_partial_frame_keeps_read_buffer_bounded`

---

## VB-IPC-DECODE-005/007: Payload Bounds Checking

**Proof Claims**:
- `PayloadTooLarge` returned only when all structural fields valid AND payload_len > max
- `PayloadLengthOutOfRange` returned when u32 payload_len cannot fit in usize

**Rust Source Refs**:
- `crates/vb_ipc/src/frame_types.rs` — decode step 6 bounds check
- `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` — existing proof harnesses

**Existing Harnesses** (already sufficient):
- `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs`: `kani_ipc_header_rejects_oversize_payload`, `kani_ipc_header_rejects_exactly_over_limit`, `kani_ipc_header_rejects_any_payload_when_max_zero`

---

## Independent Behavior Tests (complement to formal proofs)

| Test | File | Command |
|------|------|---------|
| Partial header wait | `impl_tests.rs` | `cargo test -p vb_ipc -- server_waits_for_complete_frame_when_partial_sent` |
| Oversize disconnect | `impl_tests.rs` | `cargo test -p vb_ipc -- slow_client_oversized_frame_disconnects_without_unbounded_growth` |
| Decode order | `tests.rs` | `cargo test -p vb_ipc -- adversarial_` |
| Roundtrip encode/decode | `tests.rs` | `cargo test -p vb_ipc -- encode_frame_roundtrip_` |
| Buffer bounded | `impl_tests.rs` | `cargo test -p vb_ipc -- slow_client_partial_frame_keeps_read_buffer_bounded` |