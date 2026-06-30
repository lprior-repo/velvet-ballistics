# Theorem Kernel Projection — vb-0253.3

## Boundary
- **TLA+-owned temporal model**: None — no temporal protocol or workflow behavior
- **Verus-owned Rust core**: Bounded channel construction, `send()` backpressure error taxonomy, `poll()` drain correctness, `connected` state tracking, `next_correlation` wrapping arithmetic
- **Theorem-owned kernel**: None — no algebraic state transitions, no parser/codec invariants, no arithmetic bounds theorems beyond what `wrapping_add` provides
- **Rust/runtime shell**: `IpcClient` socket I/O, Makepad UI thread, Unix domain sockets
- **External systems excluded from theorem proof**: IPC server, socket transport, OS thread scheduler

## Theorem-Owned Clauses
None — no Lean/Aeneas/Hax theorem kernel required.

## Verus Scope
The following Rust-local pure/deterministic properties are provable by Verus and/or unit tests:

1. **`next_correlation`**: `c.wrapping_add(1)` produces a correct u64 counter that wraps at `u64::MAX → 0`. Already tested by `next_correlation_wraps_at_max` unit test.

2. **Bounded channel construction**: `mpsc::sync_channel::<IpcRequest>(CHANNEL_CAPACITY)` produces a channel with exactly `CHANNEL_CAPACITY` buffer slots. Enforced at construction time.

3. **`send()` backpressure**: When the bounded channel is full, `try_send()` returns `Err(TrySendError::Full(_))` and `send()` maps this to `Err("IPC send failed: channel full")`. When the channel is disconnected, `try_send()` returns `Err(TrySendError::Disconnected(_))` and `send()` maps this to `Err("IPC send failed: ...")` with the disconnected message. Verified by unit tests.

4. **`poll()` drain**: `try_recv()` in a loop drains all currently-available replies without blocking. Verified by `bridge_new_creates_channels_and_thread` test that `poll().is_empty()` immediately after `new()`.

## Lean Obligations
None — no algebraic lemmas, no refinement projections, no parser/codec theorems.

## Waivers
- **WAIVER-LEAN-001**: No Lean/Aeneas/Hax theorem kernel
  - **Owner**: vb-0253.3 contract
  - **Reason**: Pure Rust API change; no algebraic state transitions, no protocol lattices, no arithmetic bounds beyond u64 wrapping, no parser/codec. Verus + unit tests are sufficient.
  - **Expiry**: Never
  - **Compensating evidence**: Unit tests + compile + optional proptest for capacity boundary exploration
