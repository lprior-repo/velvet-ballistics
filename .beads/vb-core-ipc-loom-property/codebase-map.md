# Codebase Map: vb-core-ipc-loom-property

## Bead
- **id**: vb-core-ipc-loom-property
- **title**: ipc/orchestrator: Add production Loom property evidence
- **goal**: Add loom concurrency property evidence for IPC/orchestrator seams

## Scope

### Crates

#### `crates/vb_ipc/`
The IPC crate handles Unix-domain-socket binary IPC with a mio-based event loop.

| File | Role | Notes |
|------|------|-------|
| `src/lib.rs` | Public types, `MemoryIngress`, `IpcFrame`, `IpcPayload` | `crossbeam_channel::bounded` for multi-producer ingress queue |
| `src/ingress.rs` | `MemoryIngress` multi-producer bounded queue | Uses `crossbeam_channel::bounded(capacity.get())`, `try_send`, `try_recv` — **key concurrency seam for loom** |
| `src/bounded.rs` | `QueueCapacity`, `MaxPayloadBytes`, `BoundedPayload` | No concurrency — pure size types |
| `src/client.rs` | IPC client for in-process use | Sync client using `std::sync::mpsc` channels |
| `src/server/mod.rs` | `IpcServer` struct, `IpcResponse`, `WorkflowResolver` trait | mio `Poll`, `HashMap<Token, ClientConnection>` |
| `src/server/impl_.rs` | `IpcServer::poll_once`, `accept_client`, `handle_readable`, `handle_writable` | **Key concurrency seam**: `clients: HashMap` mutated in poll loop, `write_buffer` accessed concurrently |
| `src/server/dispatch.rs` | `dispatch_command` / `dispatch_command_with_resolver` | Pure routing to handlers — runtime calls are the concurrency seam |
| `src/server/handlers.rs` | 15 command handlers (submit, cancel, inspect, list, complete_action, fail_action, etc.) | Handlers call `runtime.cancel_run`, `runtime.shutdown_graceful`, etc. |
| `src/server/ticket.rs` | `step_from_ticket`, `action_ticket_from_wire`, `payload_len` | Pure conversion functions |
| `src/server/error.rs` | `IpcServerError` |
| `src/server/trace.rs` | Trace drain handler |
| `src/server/helpers.rs` | Frame encode/decode helpers |

#### `crates/vb_runtime/`
The runtime crate owns shard scheduling, frame pools, and journal.

| File | Role | Notes |
|------|------|-------|
| `src/models/loom/mod.rs` | Loom model registry (`#[cfg(loom)]` only) | 5 existing models |
| `src/models/loom/bounded_queue.rs` | VB-CONC-005: bounded queue capacity invariant | **EXISTING** — uses `AtomicUsize` with `compare_exchange` |
| `src/models/loom/action_completion_cancel.rs` | VB-CONC-002: `try_complete` vs `try_cancel` race | **EXISTING** — `AtomicBool` completed/cancelled |
| `src/models/loom/timer_fired_cancel.rs` | VB-CONC-003: timer wheel fire vs cancel race | **EXISTING** — `Mutex<TimerWheel>` |
| `src/models/loom/shutdown_drain.rs` | VB-CONC-004: shutdown drain ordering | **EXISTING** — `AtomicUsize` pending counter |
| `src/models/loom/journal_writer_queue.rs` | VB-CONC-001: journal writer queue append/drain | **EXISTING** — `AtomicUsize` with `compare_exchange` |
| `src/frame_pool.rs` | `FramePool` — take/release of `RunFrame` | `Vec<RunFrame>` with `take`/`release` — **concurrency seam** |
| `src/runtime.rs` | `Runtime` struct with `cancel_run`, `shutdown_graceful` | Routes to shard-level operations |
| `src/shard/timer_wheel.rs` | `TimerWheel` — `insert`/`cancel`/`fire` | Used by existing loom model |

### Concurrency Seams Requiring Loom Evidence

| Obligation | Seam | Location | Status |
|------------|------|----------|--------|
| VB-CONC-001 | Journal writer queue append/drain | `models/loom/journal_writer_queue.rs` | **EXISTING** |
| VB-CONC-002 | Action ticket completion vs cancel | `models/loom/action_completion_cancel.rs` | **EXISTING** |
| VB-CONC-003 | Timer fired vs cancel ordering | `models/loom/timer_fired_cancel.rs` | **EXISTING** |
| VB-CONC-004 | Shutdown drain ordering | `models/loom/shutdown_drain.rs` | **EXISTING** |
| VB-CONC-005 | Bounded queue capacity invariant | `models/loom/bounded_queue.rs` | **EXISTING** |
| **NEW** | MemoryIngress bounded queue backpressure | `vb_ipc/src/ingress.rs` | **MISSING** — `crossbeam_channel::bounded` mpsc channel with `try_send`/`try_recv` |
| **NEW** | IPC server client map concurrent access | `vb_ipc/src/server/impl_.rs` | **MISSING** — `HashMap<usize, ClientConnection>` mutated in poll loop |
| **NEW** | IPC slow-client write buffer backpressure | `vb_ipc/src/server/impl_.rs` | **MISSING** — `write_buffer: Vec<u8>` drain vs fill race |

### Risk Tags

- **concurrency**: IPC server poll loop, HashMap mutation, channel operations
- **backpressure**: MemoryIngress bounded queue, slow-client write buffer
- **temporal**: cancel vs completion race, timer ordering
- **persistence**: journal writer queue

### Downstream Owners

- `rust-contract` / `proof-planner`: VB-CONC-001..005 already contracted
- `proof-writer`: New loom models for IPC seams
- `formal-verifier`: Loom execution results for evidence bundle

### Open Questions

1. Does `vb_ipc` have loom as a dev-dependency? Need to verify `Cargo.toml`.
2. The `vb-core-ipc-loom-property` bead is "production Loom property evidence" — are the existing 5 models sufficient, or are new IPC-specific models needed? The bead description says "cancel versus completion, shutdown drain reports, timer ordering, bounded queue backpressure, and slow-client IPC behavior". The first 3 are already covered; the last 2 (bounded queue backpressure, slow-client IPC) are not.
3. The `engine/property_tests.rs` is empty — is this where integration loom tests should live?
