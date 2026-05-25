# Verifier Lane Matrix — vb-8mdp.1

Maps each proof seed to its assigned verifier lanes.

## Matrix

| Proof Seed ID | Description | Kani | Verus | TLA+ | Proptest | Loom | Miri | Flux | Fuzz |
|---|---|---|---|---|---|---|---|---|---|
| VB-IPC-DECODE-001 | decode total over [u8;24] | ✅ | ✅ | — | ✅ | — | — | — | — |
| VB-IPC-DECODE-002 | magic before version | ✅ | — | — | — | — | — | — | — |
| VB-IPC-DECODE-003 | version before command | ✅ | ✅ | — | — | — | — | — | — |
| VB-IPC-DECODE-004 | command before reserved | ✅ | ✅ | — | — | — | — | — | — |
| VB-IPC-DECODE-005 | PayloadTooLarge after all structural | ✅ | — | — | — | — | — | — | — |
| VB-IPC-DECODE-006 | ReservedNonZero before PayloadTooLarge | ✅ | — | — | — | — | — | — | — |
| VB-IPC-DECODE-007 | u32→usize overflow → PayloadLengthOutOfRange | ✅ | — | — | — | — | — | — | — |
| VB-IPC-SERVER-001 | handle_readable ≤ READ_CHUNK_BYTES | — | — | ✅ | — | — | — | — | — |
| VB-IPC-SERVER-002 | no Vec::with_capacity(payload_len) pre-decode | — | — | ✅ | — | — | — | — | — |
| VB-IPC-SERVER-003 | oversize → disconnect without reading payload | ✅ | — | ✅ | — | — | — | — | — |
| VB-IPC-SERVER-004 | frame not dispatched until complete | — | — | ✅ | — | — | — | — | — |
| VB-IPC-FRAGMENT-001 | partial header → WaitingHeader, no error | — | — | ✅ | ✅ | — | — | — | — |
| VB-IPC-FRAGMENT-002 | partial payload → WaitingPayload, no allocation | — | — | ✅ | ✅ | — | — | — | — |
| VB-IPC-POSTCARD-001 | payload length mismatch detection | ✅ | — | — | ✅ | — | — | — | — |
| VB-IPC-POSTCARD-002 | encode→decode roundtrip | ✅ | — | — | ✅ | — | — | — | — |
| VB-IPC-BOUNDED-001 | BoundedPayload::new rejects oversized | ✅ | — | — | ✅ | — | — | — | — |
| VB-IPC-BOUNDED-002 | MaxPayloadBytes::DEFAULT == 1 MiB | ✅ | — | — | — | — | — | — | — |
| VB-IPC-MAGIC-001 | IPC_MAGIC == 0x5642_4C54 | ✅ | — | — | — | — | — | — | — |
| VB-IPC-MAGIC-002 | IPC_MAGIC != 0 and != 0xFFFF_FFFF | ✅ | — | — | — | — | — | — | — |
| VB-IPC-MAGIC-003 | IPC_VERSION == 1 | ✅ | — | — | — | — | — | — | — |
| VB-IPC-VERSION-001 | version != 1 rejected regardless of command | ✅ | — | — | ✅ | — | — | — | — |
| VB-IPC-COMMAND-001 | from_u16 accepts only 1..16 | ✅ | — | — | ✅ | — | — | — | — |
| VB-IPC-COMMAND-002 | as_u16/from_u16 inverses for 1..16 | ✅ | — | — | ✅ | — | — | — | — |
| VB-IPC-FRAME-001 | IPC_HEADER_LEN == 24 | ✅ | — | — | — | — | — | — | — |
| VB-IPC-FRAME-002 | encode_frame produces exact length | ✅ | — | — | ✅ | — | — | — | — |
| VB-IPC-FRAME-003 | validate_frame_magic zero-allocation | — | — | — | — | — | — | — | — |
| VB-IPC-PAYLOAD-001 | read_frame_payload exact bytes | ✅ | — | — | ✅ | — | — | — | — |
| VB-IPC-RESPONSE-001 | error response uses Health header | — | — | — | — | — | — | — | — |

## Non-Applicable Lanes

| Lane | Proof Seed | Reason |
|------|------------|--------|
| Loom | ALL IPC fragment/oversize seeds | Single-threaded sequential I/O, no concurrent memory ordering |
| Miri | ALL | `#![forbid(unsafe_code)]` in vb_ipc — no UB paths |
| Flux | ALL | Refinement types not yet in scope; decode-order proven via Kani+Verus |
| Cargo-fuzz | ALL | Kani exhausts all 2^192 header inputs; fuzz would not add coverage |

## Legend
- ✅ = Active lane (new obligations planned)
- ✅ existing = Already covered by existing harness
- — = Not applicable