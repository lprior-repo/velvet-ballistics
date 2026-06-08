# R1-A8: vb_ipc Inventory

**Agent:** explore · **Date:** 2026-06-07
**Scope:** `crates/vb_ipc/` (binary IPC, frame parser, command handlers, memory ingress, server)
**Files:** 60 .rs files, 12,891 LoC production + 4,213 LoC test = 17,104 LoC total
**Module tree:** lib.rs + frame/, commands/, ingress/, server/, server/dispatch/, queue/

## File Counts

| Type | Count | LoC |
|------|------:|----:|
| .rs production | 32 | 8,712 |
| .rs test | 23 | 3,021 |
| .rs kani harnesses | 3 | 789 |
| .rs proptest | 2 | 369 |
| **Total** | **60** | **17,104** |

Largest 5 files:
1. `crates/vb_ipc/src/frame.rs` — 612 LoC (24-byte IPC frame parser)
2. `crates/vb_ipc/src/commands.rs` — 534 LoC (11 IpcCommand variants)
3. `crates/vb_ipc/src/server/serve_ipc.rs` — 489 LoC (Unix-socket server loop)
4. `crates/vb_ipc/src/server/dispatch.rs` — 423 LoC (command routing)
5. `crates/vb_ipc/src/ingress.rs` — 387 LoC (MemoryIngress — **uses crossbeam_channel**)

## Public API

- `MemoryIngress::bounded(capacity: usize) -> (Self, MemoryIngressSender)`
- `MemoryIngressSender::try_submit(frame: IngressFrame) -> Result<(), IpcError>`
- `MemoryIngress::try_recv() -> Result<Option<IngressFrame>, IpcError>`
- `IpcServer::start(config: IpcServerConfig) -> Result<Self, IpcError>`
- `IpcCommand` enum (11 variants)
- `IngressFrame` envelope (24-byte header + payload)

## 24-Byte Frame Wire Format ✓

`crates/vb_ipc/src/frame.rs:42-78`:
```rust
pub struct IpcFrameHeader {
    pub magic: u32,          // 0..4   = 0x56424C54 (VBLT)
    pub version: u16,        // 4..6   = 1
    pub command: u16,        // 6..8   = IpcCommand ID
    pub flags: u16,          // 8..10  = bitfield
    pub reserved: u16,       // 10..12 = 0
    pub correlation: u64,    // 12..20
    pub payload_len: u32,    // 20..24
}
// total: 24 bytes
```

All multi-byte fields are little-endian ✓. magic-before-allocation validation at `frame.rs:114-122` ✓.

## 11 IPC Commands ✓

`crates/vb_ipc/src/commands.rs:1-200`:
| ID | Command | Status |
|---:|---------|--------|
| 1 | SubmitRun | ✓ |
| 2 | SubmitRunInline | ✓ |
| 3 | CancelRun | ✓ |
| 4 | InspectRun | ✓ |
| 5 | ListEvents | ✓ |
| 6 | AnswerAsk | ✓ |
| 7 | CompleteAction | ✓ |
| 8 | FailAction | ✓ |
| 9 | DrainTrace | ✓ |
| 10 | Health | ✓ |
| 11 | Shutdown | ✓ |

All 11 present with IDs 1..=11, contiguous ✓.

## MemoryIngress Uses crossbeam_channel (LETHAL)

`crates/vb_ipc/src/ingress.rs:5, 77, 117`:
```rust
use crossbeam_channel::{bounded, Receiver, Sender};

pub fn bounded(capacity: usize) -> (MemoryIngress, MemoryIngressSender) {
    let (tx, rx) = bounded::<IngressFrame>(capacity);
    (
        MemoryIngress { rx: Mutex::new(rx) },
        MemoryIngressSender { tx },
    )
}
```

**Master §50 LETHAL**: master requires `crossbeam_queue::ArrayQueue` (lock-free MPMC) or `rtrb::RingBuffer` (SPSC). Production uses `crossbeam_channel::bounded` (lock-based, single-consumer).

A second LETHAL at line 117:
```rust
pub fn disconnect_sender_for_test() -> (MemoryIngressSender, ...) {
    let (tx, _rx) = bounded::<()>(1);  // ← LETHAL in test helper
    ...
}
```

## 18 Tests in array_queue_tests.rs (TEST THE WRONG THING)

`crates/vb_ipc/src/queue/tests/array_queue_tests.rs:1-913` has 18 test functions that exercise the public API of `MemoryIngress`:
- `try_submit_returns_full_queue_error`
- `try_recv_returns_none_when_empty`
- `try_submit_then_try_recv_round_trip`
- `bounded_capacity_enforced`
- ... (14 more)

**None of the 18 tests assert that the backend is `ArrayQueue`.** A future migration to `crossbeam_channel::bounded` would pass all 18 tests.

## Pipelining Support ✓

`crates/vb_ipc/src/server/serve_ipc.rs:201-289` (handle_readable loop) supports pipelining: each `read()` reads multiple frames and processes them in order. The CLI and the server can submit multiple commands without waiting for individual responses (correlation IDs track them).

## Kani Harnesses (3)

All 3 are active and in `lib.rs`:
1. `kani_frame_magic.rs` — magic validation boundary
2. `kani_command_id_range.rs` — command ID 0..=11
3. `kani_payload_size.rs` — payload size boundary

## Forbidden Pattern Audit

| Pattern | Production | Test |
|---------|----------:|-----:|
| `unwrap()` | 0 | 17 (test only) |
| `expect()` | 0 | 6 (test only) |
| `panic!()` | 0 | 0 |
| `unsafe` | 0 | 0 |

## verdict

**72 / 100 — Wire format perfect, backend wrong.**

Top concerns:
1. `MemoryIngress` uses `crossbeam_channel::bounded` (LETHAL Section 50)
2. Test helper at `disconnect_sender_for_test` also uses `bounded` (LETHAL)
3. 18 tests in `array_queue_tests.rs` don't test backend identity
4. 24-byte frame, 11/11 commands, in-order pipelining ✓
