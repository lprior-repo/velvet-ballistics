# Proof Strategy — vb-0253.3

## Bead
vb-0253.3 — IPC bridge bounded channel change

## Scope
- **File**: `crates/vb_ui/src/ipc_bridge.rs`
- **Change**: Replace unbounded `mpsc::channel()` with bounded `mpsc::sync_channel(CHANNEL_CAPACITY)` for both request and reply channels
- **API impact**: `IpcBridge::send()` changes from blocking `send()` to non-blocking `try_send()` with backpressure error `"channel full"`

## Verification Layers Summary

| Layer | Status | Rationale |
|-------|--------|-----------|
| compile | REQUIRED | Bounded channel type change, `forbid(unsafe_code)` lint, `try_send` vs `send` method selection |
| unit-test | REQUIRED | All POST conditions, PRE conditions, INV variants, error taxonomy |
| proptest | OPTIONAL | Boundary flood testing of bounded channel capacity |
| TLA+ | NOT_APPLICABLE | No temporal/workflow behavior change — 100ms recv_timeout loop unchanged |
| Verus | NOT_APPLICABLE | Stdlib API change with exhaustively testable paths; no refinement types needed |
| Kani | NOT_APPLICABLE | No unsafe code; bounded channel is stdlib trusted |
| Loom | NOT_APPLICABLE | No concurrent Rust actors; single-producer single-consumer mpsc |
| Miri | NOT_APPLICABLE | No unsafe code in scope |

## Risk Classification

| Risk | Severity | Trigger | Verifier Lane |
|------|----------|---------|---------------|
| ui (UI thread starvation) | HIGH | Bounded channel with blocking `send()` would block render loop | compile (method selection: try_send) + unit-test |
| ipc (message loss) | HIGH | Bounded channel at capacity could silently drop or deadlock | unit-test (backpressure error path) |
| backpressure (signaling) | HIGH | `try_send` must return `"channel full"` not silently block | unit-test (VB0253-TEST-002, VB0253-TEST-007) |
| bounded-channel (capacity) | MEDIUM | Capacity value too small → excessive backpressure; too large → memory bloat | unit-test + proptest (boundary) |

## Waivers

| Waiver ID | Skipped Layer | Reason |
|-----------|---------------|--------|
| WAIVER-TLA-001 | TLA+ | No temporal protocol, workflow, liveness, or deadlock conditions. The 100ms recv_timeout polling loop and send/poll semantics are unchanged. Behavioral difference: `send()` returns backpressure error instead of silently queueing unbounded. |
| WAIVER-LEAN-001 | Theorem kernel | No algebraic/kernel proof obligations — stdlib `sync_channel` API is trusted |
| WAIVER-VERUS-001 | Verus | Bounded channel is a stdlib API change provable by unit tests. The `send()` backpressure logic is pure deterministic Rust with exhaustively testable error paths (Full vs Disconnected). |
| WAIVER-KANI-001 | Kani | No unsafe code; `sync_channel` is stdlib trusted; error paths exhaustively testable |
| WAIVER-LOOM-001 | Loom | Single-producer single-consumer mpsc; no concurrent actor interleavings to explore |

## Obligation Summary

| ID | Clause | Verifier | Command | Risk |
|----|--------|----------|---------|------|
| VB0253-COMPILE-001 | POST-001 | cargo build | `cargo build -p vb_ui --lib` | high |
| VB0253-COMPILE-002 | INV-001 | cargo build | `cargo build -p vb_ui --lib` | medium |
| VB0253-TEST-001 | POST-002 | cargo test | `cargo test -p vb_ui --lib ipc_bridge::tests::bridge_new_creates_channels_and_thread` | medium |
| VB0253-TEST-002 | POST-003 | cargo test | `cargo test -p vb_ui --lib ipc_bridge::tests::bridge_send_on_full_returns_error` | high |
| VB0253-TEST-003 | POST-004 | cargo test | `cargo test -p vb_ui --lib ipc_bridge::tests::bridge_send_without_connect_returns_not_connected_error` | medium |
| VB0253-TEST-004 | POST-005 | cargo test | `cargo test -p vb_ui --lib ipc_bridge::tests::bridge_new_creates_channels_and_thread` | low |
| VB0253-TEST-005 | POST-006 | cargo test | `cargo test -p vb_ui --lib ipc_bridge::tests::bridge_connect_to_nonexistent_socket_fails` | low |
| VB0253-TEST-006 | PRE-001 | cargo test | `cargo test -p vb_ui --lib ipc_bridge::tests` | medium |
| VB0253-TEST-007 | ERR-TX-001 | cargo test | `cargo test -p vb_ui --lib ipc_bridge::tests::bridge_send_on_full_returns_error` | high |
| VB0253-CLIPPY-001 | INV-001 | cargo clippy | `cargo clippy -p vb_ui --lib --bins --examples -- -D warnings` | medium |
| VB0253-LINT-001 | INV-001 | grep | `grep -c '#!\\[forbid(unsafe_code)\\]' crates/vb_ui/src/ipc_bridge.rs` | low |
| VB0253-PROPTEST-001 | POST-003 | cargo test | `cargo test -p vb_ui --lib ipc_bridge::proptest` (optional) | low |

## Open Questions (Q1, Q2 from contract.md)

| Question | Current Assumption | Impact if Wrong |
|----------|-------------------|-----------------|
| Q1: Exact CHANNEL_CAPACITY value? | Small power-of-two (16 or 32) | Too small → excessive backpressure; too large → memory bloat. Unit test and proptest boundary exploration will expose wrong values. |
| Q2: `try_send` vs blocking `send`? | Strictly non-blocking `try_send` semantics | If blocking `send` is required, UI thread could freeze. compile + unit-test enforce non-blocking behavior. |

## Evidence Requirements

1. **Compilation**: `cargo build -p vb_ui --lib` succeeds with no errors
2. **Tests**: `cargo test -p vb_ui --lib ipc_bridge::tests` passes — all 24+ existing tests + new `bridge_send_on_full_returns_error`
3. **Clippy**: `cargo clippy -p vb_ui --lib --bins --examples -- -D warnings` passes with no warnings
4. **Lint**: `forbid(unsafe_code)` attribute confirmed present in ipc_bridge.rs
5. **Proptest** (optional): Bounded channel capacity boundary explored

## Implementation Notes for proof-writer

The proof-writer should note the following target implementation (NOT present in current code):

```rust
// TARGET (not yet implemented):
const CHANNEL_CAPACITY: usize = 16; // or 32 — TBD power-of-two

// In IpcBridge::new() / Default:
let (req_tx, req_rx) = mpsc::sync_channel::<IpcRequest>(CHANNEL_CAPACITY);
let (rep_tx, rep_rx) = mpsc::sync_channel::<IpcReply>(CHANNEL_CAPACITY);

// In IpcBridge::send():
pub fn send(&self, request: IpcRequest) -> Result<(), String> {
    self.tx
        .try_send(request)
        .map_err(|e| match e {
            mpsc::TrySendError::Full(_) => "IPC send failed: channel full".into(),
            mpsc::TrySendError::Disconnected(_) => "IPC send failed: disconnected".into(),
        })
}
```

The **current code** uses `mpsc::channel()` (unbounded) and blocking `send()`. The proof obligations assume the **target implementation** described above.
