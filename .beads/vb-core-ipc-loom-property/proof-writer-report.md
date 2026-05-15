# Proof-Writer Report: vb-core-ipc-loom-property

## State
State: 5 (Proof Writing)
Bead: vb-core-ipc-loom-property
Attempt: 1

## Summary

Created 4 new loom concurrency models and resolved 2 critical blockers
(missing `models/loom/` directory and absent `loom` dev-dependency in vb_ipc).

## Blockers Resolved

| Blocker | Resolution |
|---|---|
| `crates/vb_ipc/src/models/loom/` missing | Created directory + `mod.rs` |
| `loom = "0.7"` absent from vb_ipc dev-dependencies | Added to `crates/vb_ipc/Cargo.toml` |

## New Artifacts Written

### vb_ipc Loom Models

| File | Obligation | LOC | Invariant |
|---|---|---|---|
| `crates/vb_ipc/src/models/loom/mod.rs` | module | 22 | — |
| `crates/vb_ipc/src/models/loom/memory_ingress.rs` | LOOM-MI-001 | 131 | available <= capacity |
| `crates/vb_ipc/src/models/loom/ipc_server_clients.rs` | LOOM-IPC-001 | 160 | token uniqueness + active <= MAX_CLIENTS |
| `crates/vb_ipc/src/models/loom/write_buffer.rs` | LOOM-IPC-002 | 162 | byte conservation: written == drained + len(buffer) |

### vb_runtime Loom Model (updated existing module)

| File | Obligation | LOC | Invariant |
|---|---|---|---|
| `crates/vb_runtime/src/models/loom/frame_pool.rs` | LOOM-FP-001 | 168 | available <= capacity |
| `crates/vb_runtime/src/models/loom/mod.rs` | (updated) | 22 | added `pub mod frame_pool` |

## Cargo.toml Changes

**File**: `crates/vb_ipc/Cargo.toml`
```diff
 [dev-dependencies]
+loom = "0.7"
 proptest = { workspace = true }
```

**File**: `crates/vb_ipc/src/lib.rs`
```diff
 pub mod client;
 pub mod frame;
 pub mod server;
+#[cfg(loom)]
+pub mod models;
```

## Loom Model Design Notes

### memory_ingress.rs (LOOM-MI-001)
- Abstract model of bounded mpsc channel backing `MemoryIngress`
- Models `try_submit` / `try_recv` as atomic CAS operations
- Invariant: queued <= capacity (backpressure envelope)
- 3 tests: basic, multi-producer, interleaved submit/recv

### ipc_server_clients.rs (LOOM-IPC-001)
- Abstract model of `HashMap<Token, ClientConnection>` with `MAX_CLIENTS = 256`
- Models `accept` (insert with capacity gate) and `remove` (drop)
- Invariant: token uniqueness (HashMap contract) + active <= MAX_CLIENTS
- 4 tests: basic, concurrent accepts, capacity-preserved, rapid cycles

### write_buffer.rs (LOOM-IPC-002)
- Abstract model of `Vec<u8>` fill/drain with `written` and `drained` counters
- Models WouldBlock path as zero-byte drain
- Invariant: `written == drained + len(buffer)` (byte conservation)
- 4 tests: basic, concurrent fill/drain, WouldBlock, capacity-respected

### frame_pool.rs (LOOM-FP-001)
- Thread-safe variant using `Arc<Mutex<FramePool>>` (production uses `&mut self`)
- Models `take`/`release` under mutex guard
- Invariant: `available() <= capacity`; release silently drops when full (POST-002)
- 4 tests: basic, concurrent take/release, capacity boundary, rapid cycles

## vb_ipc models/loom Module Declaration

Added `#[cfg(loom)] pub mod models;` to `vb_ipc/src/lib.rs` to expose the
`models/loom/` subtree for loom test discovery via `cargo test -p vb_ipc`.

## Verification Commands

```bash
# Compile check (no loom flag needed — cfg gates prevent compile-time loom dep)
cargo check -p vb_ipc --all-features
cargo check -p vb_runtime --all-features

# Run new vb_ipc loom models
RUSTFLAGS="--cfg loom" cargo test -p vb_ipc memory_ingress -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_ipc ipc_server_clients -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_ipc write_buffer -- --nocapture

# Run new vb_runtime loom model
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime frame_pool -- --nocapture

# Re-run existing vb_runtime loom models (EXISTING-001..005)
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime bounded_queue -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime action_completion_cancel -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime timer_fired_cancel -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime shutdown_drain -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime journal_writer_queue -- --nocapture
```

## Known Limitations

- `LOOM-FP-001` explores the `Arc<Mutex<FramePool>>` thread-safe variant, not the
  production `&mut self` variant. The production code is NOT thread-safe;
  this model proves the intended thread-safe design satisfies capacity invariants.
- Loom is a bounded permutation explorer; the 3×3×3 exploration bound is
  tractable but does not exhaustively prove all possible interleavings.
- IPC server `write_buffer` model uses `Vec<u8>` for simplicity; production uses
  a ring buffer but the byte conservation invariant is the same.

## Follow-up

- If production code adopts `Arc<Mutex<FramePool>>`, re-run `LOOM-FP-001` to
  verify the model still matches the implementation.
- If `MAX_CLIENTS` changes from 256, update the `MAX_CLIENTS` constant in
  `ipc_server_clients.rs` to match.
