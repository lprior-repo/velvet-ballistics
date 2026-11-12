# Contract Specification — vb-0253.3

## Context
- **Bead**: vb-0253.3 — ui: Bound IPC bridge channels with backpressure
- **Scope**: `crates/vb_ui/src/ipc_bridge.rs` — `IpcBridge::new()` and `IpcBridge::send()`
- **Risk tags**: ui, ipc, backpressure, bounded-channel
- **Assumptions**:
  - The 100ms recv_timeout polling loop in `ipc_thread` is preserved exactly
  - `poll()` continues using `try_recv` to drain replies without blocking
  - Background thread exits silently when the reply channel disconnects (existing behavior)
  - Bounded channel capacity is a compile-time constant with a value that prevents UI thread starvation while providing backpressure signaling

## Domain Terms
- **Unbounded channel**: `mpsc::channel()` — sender never blocks, channel grows without limit
- **Bounded/sync channel**: `mpsc::sync_channel(capacity)` — sender blocks (or returns error) when capacity is reached
- **Backpressure**: The signal returned to the UI when the IPC request queue is full
- **Request channel**: `tx: Sender<IpcRequest>` — UI thread sends `IpcRequest` values
- **Reply channel**: `rx: Receiver<IpcReply>` — background thread sends `IpcReply` values
- **Channel disconnect**: Sender half dropped or receiver half dropped; `send()` returns `SendError`
- **Channel full**: Bounded channel at capacity; `sync_channel::send()` returns `TrySendError`

## Open Questions
- **Q1**: What is the exact bounded capacity value? (Assumed: small power-of-two, e.g., 16 or 32)
- **Q2**: Is `IpcBridge::send()` allowed to block briefly before returning the backpressure error, or must it be strictly non-blocking `try_send`? (Assumed: strictly non-blocking `try_send` semantics)

## Preconditions
- **PRE-001**: `IpcBridge::new()` may only fail if the background thread cannot be spawned (resource exhaustion). When it fails, the returned `IpcBridge` has `tx` disconnected and subsequent `send()` calls return errors.
- **PRE-002**: `IpcBridge::send(&self, request)` requires `self.tx` to still be connected. If the background thread has died, `send()` returns an error.

## Postconditions
- **POST-001**: `IpcBridge::new()` initializes both request and reply channels as bounded `sync_channel` with capacity `CHANNEL_CAPACITY`.
- **POST-002**: `IpcBridge::send(&self, request)` returns `Ok(())` when the request channel has capacity available.
- **POST-003**: `IpcBridge::send(&self, request)` returns `Err(String)` containing `"channel full"` when the bounded request channel is at capacity (backpressure signal).
- **POST-004**: `IpcBridge::send(&self, request)` returns `Err(String)` containing `"disconnected"` when the request channel sender half is dropped (background thread died).
- **POST-005**: `poll()` continues to drain all currently-available replies via `try_recv` and return them as a `Vec<IpcReply>` without blocking the UI thread.
- **POST-006**: `is_connected()` returns `true` after receiving `IpcReply::Connected` and `false` after receiving `IpcReply::Disconnected` or `IpcReply::ConnectionFailed(_)`.

## Invariants
- **INV-001**: `IpcBridge` owns exactly one `tx: Sender<IpcRequest>` and one `rx: Receiver<IpcReply>`. Both channels are bounded at construction.
- **INV-002**: `connected` field accurately tracks whether the last seen reply was `Connected` (true) or `Disconnected`/`ConnectionFailed` (false).
- **INV-003**: The background thread loop (recv_timeout 100ms) does not change the bounded channel capacity or the send/try_send semantics.

## Error Taxonomy
All errors returned from `send()` are `String` (not a typed enum, per existing API):

| Variant label in string | Trigger condition |
|---|---|
| `"IPC send failed: channel full"` | Bounded request channel at capacity — backpressure |
| `"IPC send failed: ..."` | Any other `TrySendError` (disconnected, etc.) |

The error string format matches the existing `map_err` pattern: `format!("IPC send failed: {e}")`.

## Contract Signatures

```rust
// crates/vb_ui/src/ipc_bridge.rs

/// Channel capacity constant for bounded sync channels.
const CHANNEL_CAPACITY: usize = /* TBD: power-of-two, e.g., 16 or 32 */;

pub struct IpcBridge {
    tx: Sender<IpcRequest>,   // was unbounded channel, now bounded sync_channel
    rx: Receiver<IpcReply>,   // was unbounded channel, now bounded sync_channel
    connected: bool,
    _handle: Option<JoinHandle<()>>,
}

impl IpcBridge {
    /// Creates a new bridge with bounded request/reply channels.
    /// Thread spawn failure results in disconnected tx (send returns errors).
    pub fn new() -> Self;

    /// Sends a request to the background IPC thread (non-blocking).
    ///
    /// Returns `Ok(())` if the request was queued.
    /// Returns `Err(String)` if the channel is full (backpressure) or disconnected.
    pub fn send(&self, request: IpcRequest) -> Result<(), String>;

    /// Polls for all pending replies without blocking the UI.
    pub fn poll(&mut self) -> Vec<IpcReply>;

    /// Returns whether the bridge considers itself connected.
    pub fn is_connected(&self) -> bool;
}
```

## Verus-Owned Clauses
- **INV-001**: `IpcBridge` struct field types and bounded channel construction correctness — proven via compilation and unit tests
- **POST-001**: Channel capacity constant is applied at construction — proven via compilation + unit test
- **POST-002 / POST-003 / POST-004**: `send()` error taxonomy for full vs. disconnected — proven via unit test + proptest
- **INV-002**: `connected` field tracks connection state correctly — proven via unit tests

## TLA+-Owned Clauses
None — this is a Rust-local API change with no temporal/workflow behavior change. The 100ms recv_timeout polling loop and send/poll semantics are unchanged. The only behavioral difference is that `send()` now returns a backpressure error when the request queue is full instead of silently queueing an unbounded number of requests.

## Non-goals
- No changes to `IpcRequest` or `IpcReply` enum variants
- No changes to `poll()`, `is_connected()` signatures or behavior
- No changes to the background thread's recv_timeout loop or `send_and_recv` helper
- No changes to `reply_from_*` helper functions
- No changes to any downstream `IpcAppWiring` or `ReplayController` callers
