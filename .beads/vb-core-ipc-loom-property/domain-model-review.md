# Domain Model Review: vb-core-ipc-loom-property

## Existing Loom Models (VB-CONC-001..005)

All 5 existing loom models use the same structural pattern:
- `Arc<Something>` shared across loom threads
- `loom::model(|| { ... })` as the test harness
- `loom::thread::spawn(move || { ... })` for concurrent operations
- `check_invariants()` assertion at end

This is the canonical pattern to follow for the 3 new models.

---

## Review: MemoryIngress (vb_ipc/src/ingress.rs)

### Concurrency Seam
- `crossbeam_channel::bounded(capacity)` creates a multi-producer mpsc channel
- `try_send` and `try_recv` are the concurrency boundary
- The channel capacity is the backpressure envelope

### Model Design Decision
- **Model type**: Abstract counter mirroring the bounded_queue.rs pattern
- **Why not model crossbeam internals**: loom explores our *usage* surface, not the library's internals
- **Invariant**: `available <= capacity` where `available` approximates channel depth
- **Operations to model**: `try_submit` (decrement available), `try_recv` (increment available)

### Risk Assessment
- Risk: `concurrency`, `backpressure`
- Is multi-producer safe? `crossbeam_channel::bounded` is multi-producer safe
- Could available exceed capacity? Only if crossbeam has a bug (unlikely); loom tests our surface

### Command
```
RUSTFLAGS="--cfg loom" cargo test -p vb_ipc memory_ingress
```

---

## Review: FramePool (vb_runtime/src/frame_pool.rs)

### Concurrency Seam
- `take(&mut self, ...)` and `release(&mut self, ...)` mutate `self.frames: Vec<RunFrame>`
- Current impl is NOT thread-safe (requires `&mut self`)
- Loom model wraps in `Arc<Mutex<FramePool>>` to explore the *intended* thread-safe API
- The `available() <= capacity` invariant must hold regardless of concurrent access pattern

### Model Design Decision
- **Model type**: Abstract bounded counter (same as bounded_queue.rs)
- **Why not model Vec internals**: loom tests the capacity invariant, not the Vec allocation strategy
- **Invariant**: `available <= capacity` (frames.len() never exceeds capacity)
- **Operations**: `try_take` (decrement), `release` (increment, saturating)

### Risk Assessment
- Risk: `concurrency`, `backpressure`
- Is concurrent access safe? Only with `Arc<Mutex<FramePool>>` wrapper
- Does silent drop on capacity breach match production? YES — `release` silently drops when full
- Is fresh allocation on empty pool covered? YES — loom model handles the non-full path

### Command
```
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime frame_pool
```

---

## Review: IPC Server Client-Map + Write-Buffer (vb_ipc/src/server/impl_.rs)

### Concurrency Seam
- `clients: HashMap<usize, ClientConnection>` mutated only inside `poll_once`
- `write_buffer: Vec<u8>` in each `ClientConnection` — accessed in `handle_readable` (fill) and `handle_writable` (drain)
- The poll loop is effectively single-threaded; but loom still explores handler interleavings

### Model Design Decision
- **Model type**: Two separate models
  1. `IpcClientMap` — concurrent insert/remove/get on HashMap with token uniqueness invariant
  2. `WriteBuffer` — concurrent fill/drain on Vec with bytes_written == bytes_drained invariant
- **Why separate models**: Different invariants and different failure modes
- **Token uniqueness**: Each token is assigned monotonically; removal is the only mutating operation on the map
- **Write buffer**: `drain(..written)` is the critical operation — bytes drained must equal bytes written on non-WouldBlock path

### Risk Assessment
- Risk: `concurrency`
- Is HashMap mutated concurrently? Only inside single-threaded poll loop — LOW risk
- Is write_buffer accessed concurrently? handle_readable fills, handle_writable drains — sequential per token
- Still worth modeling? YES — models the *intent* and catch future refactoring errors

### Command
```
RUSTFLAGS="--cfg loom" cargo test -p vb_ipc ipc_server_clients
RUSTFLAGS="--cfg loom" cargo test -p vb_ipc write_buffer
```

---

## Summary

| Model | Complexity | Correctness Argument |
|-------|------------|---------------------|
| MemoryIngress | Low | crossbeam bounded channel; available <= capacity |
| FramePool | Low | Abstract counter with compare_exchange |
| IpcClientMap | Low | HashMap token uniqueness; sequential poll loop |
| WriteBuffer | Low | Bytes conserved on drain path |

All 3 new models are tractable. No theorem kernel projection needed — loom + existing VB-CONC-005 pattern is sufficient.
