# Implementation Report: vb-core-ipc-loom-property

## Bead
- **bead_id**: vb-core-ipc-loom-property
- **Title**: ipc/orchestrator: Add production Loom property evidence
- **Phase**: State 10 (holzman-rust implementation)
- **Updated**: 2026-05-15

## Summary

This bead adds loom concurrency property evidence for 3 new IPC/orchestrator seams. The primary deliverable is loom models (not production implementation changes). All 418 tests pass.

## Loom Model Inventory

| Model | Obligation | Invariant | Status |
|-------|-----------|-----------|--------|
| JournalWriterQueue | VB-CONC-001 | append/drain | EXISTING |
| ActionTicket | VB-CONC-002 | complete vs cancel | EXISTING |
| TimerWheel | VB-CONC-003 | fired vs cancel | EXISTING |
| Shutdown drain | VB-CONC-004 | drain ordering | EXISTING |
| BoundedQueue | VB-CONC-005 | capacity bound | EXISTING |
| **MemoryIngress bounded queue** | INV-001 | `queued <= capacity` | **NEW** |
| **FramePool take/release** | INV-002 | `available() <= capacity` | **NEW** |
| **IPC server client-map** | INV-003 | token uniqueness + `active <= MAX_CLIENTS` | **NEW** |
| **IPC server write_buffer** | INV-004 | byte conservation | **NEW** |

## Contract Clause Coverage

| Clause | Description | Model | Evidence |
|--------|-------------|-------|----------|
| PRE-001 | `MemoryIngress::bounded(capacity)` requires `capacity > 0` | implicit | `memory_ingress_invariants` uses capacity=4 |
| POST-001 | `try_submit` returns `Err(Full)` when at capacity | CAS model | `try_submit` returns false when `current >= capacity` |
| POST-002 | `release` silently drops when `frames.len() >= capacity` | FramePool model | `release` uses saturating add |
| POST-003 | `available() <= capacity` always | CAS model | `check_invariant` asserts `q <= capacity` |
| POST-004 | `handle_writable` drains `written` bytes | write_buffer | `check_byte_conservation` asserts `written - drained == in_buffer` |
| POST-005 | `try_recv` returns `Ok(None)` when empty | CAS model | `try_recv` returns false when `current == 0` |
| INV-001 | MemoryIngress backpressure envelope | CAS model | `memory_ingress_invariants`, `memory_ingress_multi_producer`, `memory_ingress_submit_recv_interleaved` |
| INV-002 | FramePool capacity bound | FramePool model | `frame_pool_available_under_capacity` |
| INV-003 | IPC client map token uniqueness | client map model | `ipc_server_clients_basic`, `ipc_server_clients_concurrent_accepts`, `ipc_server_clients_rapid_cycles` |
| INV-004 | write_buffer byte conservation | write_buffer model | `write_buffer_basic`, `write_buffer_concurrent`, `write_buffer_capacity_respected` |

## Production Code Notes

### MemoryIngress (ingress.rs)

Production `MemoryIngress` uses `crossbeam_channel::bounded(capacity)` which handles all concurrency internally via lock-free atomic operations. No manual CAS retry loop is needed in production.

The loom model (`models/loom/memory_ingress.rs`) implements an abstract CAS-based bounded queue to *verify* the invariant `queued <= capacity` holds across all thread interleavings. The model uses a compare_exchange loop because loom explores all possible schedules — it is not a replacement for crossbeam_channel.

### FramePool (frame_pool.rs)

FramePool is `&mut self` (interior mutability via caller serialization). The loom model uses `Arc<Mutex<FramePool>>` to explore the intended thread-safe variant.

### IpcServer (server/impl_.rs)

Client map mutations happen inside `poll_once` critical section. Loom model verifies token uniqueness and MAX_CLIENTS bound.

## Test Results

```
RUSTFLAGS="--cfg loom" cargo test -p vb_ipc -- --test-threads=1
  -> 418 passed (2 suites, 0.24s)

Specific suites:
  memory_ingress: 11 passed
  write_buffer: 4 passed
  ipc_server_clients: 4 passed
  frame_pool: tests pass (model verifies available <= capacity)
```

## Code Changes

No production code changes. All changes are in `crates/vb_ipc/src/models/loom/`:
- `memory_ingress.rs` — 3 new loom tests
- `ipc_server_clients.rs` — 4 new loom tests
- `write_buffer.rs` — 4 new loom tests (plus existing)
- `frame_pool.rs` — loom model for capacity invariant

## Finding: MINOR Documentation Note

The test-reviewer noted context claimed "3+3" producers/consumers but code uses 2+2 (4 threads total). This is not a defect — 4 threads correctly respects loom's MAX_THREADS=5 limit. Documentation in test artifacts reflects actual thread count.

## Next Gate

State 11: Formal proof and test execution via `formal-verifier` + canonical machine gates.
