# Verification Layers: vb-core-ipc-loom-property

## Boundary

- **Verus-owned kernel**: FramePool capacity invariant (INV-002) — Rust-local pure data structure invariant
- **TLA+ temporal model**: MemoryIngress channel backpressure (INV-001), IPC client-map token uniqueness (INV-003), write buffer byte conservation (INV-004)
- **Theorem projection**: NONE — loom + existing VB-CONC-005 pattern is sufficient
- **Runtime shell**: `crossbeam_channel::bounded`, mio `Poll`, `HashMap` operations, `Vec::drain`
- **External systems excluded from formal proof**: OS socket buffers, crossbeam_channel mpsc internals, mio event loop internals

---

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Tertiary Layer | Waiver? |
|----------------|-------------|----------------|----------------|---------|
| INV-001 (MemoryIngress backpressure) | loom | tla-plus | proptest | NO |
| INV-002 (FramePool capacity) | loom | verus | N/A | NO |
| INV-003 (IPC client-map) | loom | tla-plus | N/A | NO |
| INV-004 (write buffer byte conservation) | loom | tla-plus | N/A | NO |
| VB-CONC-001 (JournalWriterQueue) | loom | N/A | N/A | EXISTING |
| VB-CONC-002 (ActionTicket) | loom | N/A | N/A | EXISTING |
| VB-CONC-003 (TimerWheel) | loom | N/A | N/A | EXISTING |
| VB-CONC-004 (ShutdownDrain) | loom | N/A | N/A | EXISTING |
| VB-CONC-005 (BoundedQueue) | loom | N/A | N/A | EXISTING |

---

## Loom Scope

### New Models

**1. MemoryIngress Bounded Queue (memory_ingress.rs)**
- **Module path**: `crates/vb_ipc/src/models/loom/memory_ingress.rs`
- **Pattern**: Abstract bounded counter (mirrors bounded_queue.rs VB-CONC-005)
- **Operations**: `try_submit` (CAS decrement available), `try_recv` (increment available)
- **Invariant**: `0 <= available <= capacity`
- **Evidence command**: `RUSTFLAGS="--cfg loom" cargo test -p vb_ipc memory_ingress`
- **Loom test functions**:
  - `memory_ingress_single_submit_recv`
  - `memory_ingress_concurrent_submit_recv`
  - `memory_ingress_at_capacity`

**2. FramePool Take/Release (frame_pool.rs)**
- **Module path**: `crates/vb_runtime/src/models/loom/frame_pool.rs`
- **Pattern**: Abstract bounded counter with `Arc<Mutex<FramePool>>` wrapper (mirrors VB-CONC-005)
- **Operations**: `try_take` (CAS decrement), `release` (increment, saturating)
- **Invariant**: `0 <= available <= capacity`
- **Evidence command**: `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime frame_pool`
- **Loom test functions**:
  - `frame_pool_take_release`
  - `frame_pool_concurrent_take_release`
  - `frame_pool_at_capacity_silent_drop`

**3. IPC Server Client-Map (ipc_server_clients.rs)**
- **Module path**: `crates/vb_ipc/src/models/loom/ipc_server_clients.rs`
- **Pattern**: HashMap token insert/remove with uniqueness invariant
- **Operations**: `insert_client`, `remove_client`, `get_client`
- **Invariant**: token uniqueness, `active.size() <= MAX_CLIENTS`
- **Evidence command**: `RUSTFLAGS="--cfg loom" cargo test -p vb_ipc ipc_server_clients`
- **Loom test functions**:
  - `client_map_insert_remove`
  - `client_map_concurrent_insert_remove`

---

## TLA+ Scope

### MemoryIngressChannel
- **Module/model path**: `specs/MemoryIngressChannel.tla`
- **Variables**: `queued`, `CAPACITY`
- **Actions**: `TrySubmit`, `TryRecv`, `SubmitFull`, `RecvEmpty`
- **Safety invariants**: `queued <= CAPACITY`, `queued >= 0`
- **Temporal properties**: `[][queued <= CAPACITY]_queued`
- **Fairness**: Weak fairness on `TrySubmit` and `TryRecv`
- **Evidence command**: `cd specs && tlc -config MemoryIngressChannel.cfg MemoryIngressChannel.tla`

### IpcServerClientMap
- **Module/model path**: `specs/IpcServerClientMap.tla`
- **Variables**: `clients`, `nextToken`, `active`
- **Actions**: `AcceptClient`, `RemoveClient`, `GetClient`
- **Safety invariants**: `Cardinality(active) <= MAX_CLIENTS`
- **Evidence command**: `cd specs && tlc -config IpcServerClientMap.cfg IpcServerClientMap.tla`

### WriteBuffer
- **Module/model path**: `specs/WriteBuffer.tla`
- **Variables**: `buffer`, `written`, `drained`
- **Actions**: `Fill`, `Drain`, `WouldBlock`
- **Safety invariant**: `Len(buffer) = written - drained`
- **Evidence command**: `cd specs && tlc -config WriteBuffer.cfg WriteBuffer.tla`

---

## Verus Scope

### FramePool Capacity Invariant
- **Rust target**: `crates/vb_runtime/src/frame_pool.rs::FramePool`
- **Spec functions**: `available()`, `capacity()`, `capacity_invariant()`
- **Proof obligations**:
  - `release` preserves `available() <= capacity()`
  - `take` (when pool non-empty) preserves invariant
  - Construction via `new()` establishes initial invariant
- **Trusted boundary**: `FramePool::new` constructor enforces capacity constraints
- **Shell exclusions**: Fresh `RunFrame::new` allocation, `Vec` internal reallocation
- **Evidence command**: `moon run :verify-proof` (gauntlet lane)

---

## Waivers

| Clause | Waiver Reason | Compensating Evidence |
|--------|--------------|----------------------|
| INV-001 (crossbeam mpsc) | Library-level concurrency; loom tests our usage surface | Loom model + TLA+ specification |
| INV-003 (mio Poll) | mio event loop is single-threaded; structural intent tested by loom | TLA+ model of client-map structure |
| INV-004 (Vec::drain) | Standard library operation; loom tests our usage surface | Loom model + TLA+ byte conservation |
| VB-CONC-001..005 | Already covered by existing passing loom models | Existing loom test evidence |
