# Contract Specification: vb-core-ipc-loom-property

## Context
- **Bead**: vb-core-ipc-loom-property
- **Title**: ipc/orchestrator: Add production Loom property evidence
- **Goal**: Add loom concurrency property evidence for 3 new IPC/orchestrator seams
- **Existing models**: VB-CONC-001..005 already pass; 3 new loom models needed
- **New seams**: MemoryIngress bounded queue, FramePool take/release, IPC server client-map/write-buffer

## Domain Terms
- `MemoryIngress`: multi-producer bounded mpsc channel using `crossbeam_channel::bounded`
- `FramePool`: concurrent frame pool with `take`/`release` under capacity bound
- `IpcServer`: mio-based poll loop with `HashMap<Token, ClientConnection>`
- `write_buffer`: `Vec<u8>` drain vs fill race in poll loop
- `VB-CONC-005`: atomic bounded counter; available never exceeds capacity
- `VB-CONC-002`: atomic bool completed/cancelled; cannot be both

## Assumptions
- `crossbeam_channel::bounded` is the concurrency boundary for MemoryIngress
- FramePool is NOT currently thread-safe (interior mutability via `&mut self`); loom model uses `Arc<Mutex<FramePool>>` to explore the intended thread-safe variant
- IPC server client-map mutations happen only inside `poll_once` critical section
- The bead adds loom models, not production implementation changes

## Open Questions
- Does `vb_ipc` have `loom` as a dev-dependency? Verified below.
- Are the 3 new models sufficient to cover the 5 IPC risk tags (concurrency, backpressure, temporal)?

---

## Preconditions

- **PRE-001**: `MemoryIngress::bounded(capacity)` requires `capacity > 0`
- **PRE-002**: `FramePool::new(step_count, slot_count, capacity)` requires `0 < capacity <= 4096` and `step_count > 0`
- **PRE-003**: `IpcServer::handle_readable` / `handle_writable` require the client token exists in `clients`

## Postconditions

- **POST-001**: `MemoryIngress::try_submit` returns `Err(Full)` when channel is at capacity; `Err(Disconnected)` when receiver dropped
- **POST-002**: `FramePool::release` silently drops frames when `frames.len() >= capacity`; never panics
- **POST-003**: `FramePool::available() <= capacity` always holds
- **POST-004**: `IpcServer::handle_writable` drains `written` bytes from `write_buffer`; no data loss on `WouldBlock`
- **POST-005**: `MemoryIngress::try_recv` returns `Ok(None)` when empty; `Err(Disconnected)` when sender dropped

## Invariants

- **INV-001**: `MemoryIngress` available slots never exceed channel capacity (backpressure envelope)
- **INV-002**: `FramePool::available() <= capacity` for all states
- **INV-003**: IPC server `clients` map token uniqueness: each token maps to at most one `ClientConnection`
- **INV-004**: `write_buffer` bytes written equals bytes drained on non-WouldBlock path
- **INV-005**: VB-CONC-005 invariant: `BoundedQueue::available <= capacity` (existing, passes)
- **INV-006**: VB-CONC-002 invariant: `ActionTicket` cannot be both completed and cancelled (existing, passes)

## Error Taxonomy

- `IpcError::Full` — MemoryIngress channel at capacity (backpressure signal)
- `IpcError::Disconnected` — ingress channel closed; no recovery possible
- `IpcServerError::TooManyClients` — `clients.len() >= MAX_CLIENTS`
- `IpcServerError::PollFailed` — mio poll registration error
- `CoreError::ResourceLimitExceeded` — FramePool capacity exceeded on construction
- `CoreError::AllocationFailed` — FramePool take on empty pool when fresh alloc denied (should not happen with current impl)

## Contract Signatures

```rust
// vb_ipc/src/ingress.rs
pub fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError>
pub fn try_recv(&self) -> Result<Option<IngressFrame>, IpcError>

// vb_runtime/src/frame_pool.rs
pub fn take(&mut self, run_id: RunId, first_step: StepIdx) -> CoreResult<RunFrame>
pub fn release(&mut self, frame: RunFrame)
pub fn available(&self) -> usize
pub const fn capacity(&self) -> usize
```

## TLA+-Owned Clauses

- **INV-001** (MemoryIngress backpressure): TLA+ model of multi-producer bounded channel with `try_submit`/`try_recv`; invariant: `queued <= capacity`
- **INV-002** (FramePool capacity): covered by existing VB-CONC-005 model — same atomic counter pattern
- **INV-003** (IPC client map token uniqueness): TLA+ model of `HashMap` insert/remove/get in mio poll loop

## Loom Model Inventory

| Model | Obligation | Status | File |
|-------|-----------|--------|------|
| JournalWriterQueue append/drain | VB-CONC-001 | EXISTING | `models/loom/journal_writer_queue.rs` |
| ActionTicket complete vs cancel | VB-CONC-002 | EXISTING | `models/loom/action_completion_cancel.rs` |
| TimerWheel fired vs cancel | VB-CONC-003 | EXISTING | `models/loom/timer_fired_cancel.rs` |
| Shutdown drain ordering | VB-CONC-004 | EXISTING | `models/loom/shutdown_drain.rs` |
| BoundedQueue capacity | VB-CONC-005 | EXISTING | `models/loom/bounded_queue.rs` |
| **MemoryIngress bounded queue** | INV-001 | **NEW** — needs loom model | `models/loom/memory_ingress.rs` |
| **FramePool take/release** | INV-002 | **NEW** — needs loom model | `models/loom/frame_pool.rs` |
| **IPC server client-map** | INV-003 | **NEW** — needs loom model | `models/loom/ipc_server_clients.rs` |

## Non-goals
- Production implementation changes to vb_ipc or vb_runtime (loom models only)
- Formal proof of crossbeam_channel internals (loom explores our usage surface only)
- IPC server dispatch sequentiality proof (dispatch is already marked LOW RISK)
