# Proof Strategy — vb-8mdp.1: IPC Fragmented-Frame and Oversize-Message Tests

## Scope
Planning proof obligations for IPC frame decoding, with focus on:
- 24-byte header decode order (magic → version → command → reserved → correlation → payload_len)
- No pre-budget allocation (payload_len never used for Vec::with_capacity before decode)
- Decode order theorem (each step gates the next)
- Oversize rejection before any payload bytes are read from socket

## Verification Lanes Selected

### Kani (primary — bounded model checking)
**Why**: All decode functions operate over `[u8; 24]` — a finite, exhaustively checkable domain.
Kani can enumerate all 2^192 inputs via `kani::any()` and verify decode-order theorems.

**Covered by existing harnesses**:
- `crates/vb_ipc/src/kani_ipc_header.rs` — VB-IPC-DECODE-001/003 (valid decode, magic/version/reserved rejection)
- `crates/vb_ipc/src/kani_ipc_decode_order.rs` — VB-IPC-POSTCARD-ENVELOPE-001 (decode order, magic before version, reserved before payload_len)
- `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` — VB-IPC-DECODE-002 (oversize payload rejection at various bounds)

**New obligations planned**:
- `kani_ipc_header_rejects_oversize_before_payload_read`: Prove `PayloadTooLarge` returned without reading any payload bytes (addressing VB-IPC-SERVER-003)
- `kani_harness_decode_order_total_fn`: Prove total function over all 2^192 inputs (VB-IPC-DECODE-001)
- `kani_harness_decode_order_command_before_reserved`: Prove command checked before reserved (VB-IPC-DECODE-004)
- `kani_harness_decode_order_version_before_command`: Prove version checked before command (VB-IPC-DECODE-003)

### Verus (spec-fn refinement — Rust-native formal spec)
**Why**: `IpcFrameHeader::decode` is a pure total function. Verus `spec fn` can formally specify the 6-step decode order as a ghost model, and the `exec fn` can be proven equivalent to it.

**New obligations planned**:
- `verus_ipc_decode_order_spec`: Verus spec function encoding the ordered decode steps
- `verus_ipc_decode_order_proof`: Proof that `decode` exec matches spec for all 24-byte inputs

### TLA+ (state machine — server partial-frame behavior)
**Why**: The server's `handle_readable` loop accumulates bytes and waits for `read_buffer.len() >= frame_total_len`. TLA+ can model the state machine: `WaitingHeader | WaitingPayload | Dispatching` and prove that `PayloadTooLarge` causes disconnect without entering `WaitingPayload`.

**New obligations planned**:
- `tla+_ipc_server_fragmentation`: TLA+ spec for partial header/payload accumulation
- `tla+_ipc_oversize_rejection`: TLA+ proof that oversize header causes immediate disconnect

### Loom (not applicable)
**Why**: The IPC server is single-threaded; there are no concurrent channel operations, no lock-free structures, no interleavings. The fragment/oversize behaviors are about sequential I/O, not memory-ordering races.

### Miri (not applicable — no unsafe)
**Why**: The `vb_ipc` crate is `#![forbid(unsafe_code)]`. All byte reads are through safe `Read::read_exact` and `byteorder::ReadBytesExt`. No raw pointers, no `MaybeUninit`, no aliasing.

### Proptest (property-based testing — complement to Kani)
**Why**: Kani proves absence of panics and decode-order theorems for all 2^192 inputs exhaustively. Proptest generates 10,000+ random inputs at runtime, catching edge cases Kani might miss in concrete execution (though Kani already covers all inputs).

**New obligations planned**:
- `proptest_decode_order_fuzz`: Property test for decode order with random 24-byte inputs
- `proptest_oversize_rejection`: Property test for oversize payload rejection across various max values

### Cargo-fuzz (not applicable)
**Why**: Fuzzing targets byte-level malformed input, but the Kani harnesses already exhaust all 2^192 header byte combinations. The server-side fragment/oversize behaviors are structural state-machine properties, not input-format edge cases.

## Lane Coverage Summary

| Proof Seed | Kani | Verus | TLA+ | Proptest | Status |
|------------|------|-------|------|----------|--------|
| VB-IPC-DECODE-001 | ✅ existing+new | ✅ new | — | ✅ new | partial |
| VB-IPC-DECODE-002 | ✅ existing | — | — | — | existing |
| VB-IPC-DECODE-003 | ✅ existing+new | ✅ new | — | — | partial |
| VB-IPC-DECODE-004 | ✅ existing+new | ✅ new | — | — | partial |
| VB-IPC-DECODE-005 | ✅ existing | — | — | — | existing |
| VB-IPC-DECODE-006 | ✅ existing | — | — | — | existing |
| VB-IPC-DECODE-007 | ✅ existing | — | — | — | existing |
| VB-IPC-SERVER-002 | ✅ new | — | ✅ new | — | new |
| VB-IPC-SERVER-003 | ✅ new | — | ✅ new | — | new |
| VB-IPC-FRAGMENT-001 | — | — | ✅ new | ✅ new | new |
| VB-IPC-FRAGMENT-002 | — | — | ✅ new | ✅ new | new |

## Key Risks Addressed
- **Decode order violation**: magic checked before version checked before command checked before reserved checked before payload_len — covered by Kani + Verus
- **Pre-budget allocation**: payload_len never used for Vec::with_capacity before header decode — covered by TLA+ state machine + code review
- **Oversize rejection before payload read**: server disconnects on PayloadTooLarge without reading declared bytes — covered by TLA+ + Kani new harness
- **Partial frame waits**: server stays in WaitingHeader/WaitingPayload without error — covered by TLA+ + proptest

## Assumptions and Bounds
- `IPC_HEADER_LEN == 24` (compile-time constant, enforced by type signature `[u8; 24]`)
- `IPC_MAGIC == 0x5642_4C54` (compile-time constant)
- `IPC_VERSION == 1` (compile-time constant)
- `MaxPayloadBytes::DEFAULT == 1_048_576` (compile-time constant, NoZeroUsize)
- Decode uses `byteorder::LittleEndian::read_u32_le` and `read_u16_le` — safe, no panics on `[u8; 24]`
- Server loop: single-threaded, no async concurrency in fragment/oversize path