# Verification Layers — vb-0253.3

## Boundary
- **Verus-owned kernel**: Rust-local pure/deterministic behavior: bounded channel construction, `send()` backpressure error taxonomy, `poll()` drain behavior, `connected` tracking, `next_correlation` wrapping
- **TLA+ temporal model**: None — no temporal protocol, workflow, liveness, or deadlock conditions
- **Theorem projection**: None — no algebraic kernels
- **Runtime shell**: Background IPC thread (recv_timeout 100ms loop), Makepad UI render loop caller, socket I/O
- **External systems**: IPC server, Unix socket transport

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layers | Notes |
|---|---|---|---|
| PRE-001 (thread spawn failure) | compile | unit-test | Thread spawn failure produces disconnected tx; verified by compile |
| PRE-002 (tx connected) | compile | unit-test | `send()` returns error when tx dropped |
| POST-001 (bounded channel init) | compile | unit-test | `sync_channel(capacity)` construction verified by compile |
| POST-002 (send Ok on capacity) | unit-test | proptest | Happy-path send succeeds when channel not full |
| POST-003 (send Err on full) | unit-test | proptest | Backpressure error when bounded channel at capacity |
| POST-004 (send Err on disconnect) | unit-test | — | Existing behavior preserved |
| POST-005 (poll non-blocking drain) | unit-test | — | `try_recv` drain verified; compile ensures no blocking call |
| POST-006 (connected tracking) | unit-test | — | `is_connected()` state machine verified |
| INV-001 (struct fields bounded) | compile | unit-test | Channel type change verified by compile |
| INV-002 (connected state) | unit-test | — | State transitions verified by existing tests |
| ERR-TX-001 (channel full error) | unit-test | proptest | Typed error variant for backpressure |
| ERR-TX-002 (disconnected error) | unit-test | — | Existing error path preserved |

## Verus Scope
**Target**: `crates/vb_ui/src/ipc_bridge.rs` — `IpcBridge` construction, `send()`, `poll()`

**Spec/proof surface**:
- `CHANNEL_CAPACITY` constant is a positive power-of-two (enforced at compile time via const assertion)
- `IpcBridge::new()` initializes exactly two bounded `sync_channel` handles
- `send()` uses `try_send` (not `send`) for non-blocking semantics
- `send()` maps `TrySendError::Full` → `"channel full"` error string
- `send()` maps `TrySendError::Disconnected` → `"disconnected"` error string
- `poll()` uses `try_recv` in a loop (no blocking)
- `connected` field transitions: `false → true` on `IpcReply::Connected`; `true → false` on `Disconnected` or `ConnectionFailed`

**Trusted boundary**: `std::sync::mpsc::sync_channel` is trusted stdlib; `IpcBridge` struct fields are private with controlled construction

**Shell exclusions**: Socket I/O, thread scheduling, Makepad UI, IPC server

**Evidence command**: `cargo test -p vb_ui ipc_bridge::tests` + `cargo build -p vb_ui`

## TLA+ Scope
Not applicable — no temporal model.

## Theorem Scope
Not applicable — no Lean/Aeneas/Hax kernel.

## Proptest Scope (Optional)
**Target**: `send()` backpressure behavior

**Property**: Flooding `send()` with more requests than `CHANNEL_CAPACITY` eventually produces at least one `Err("channel full")` before the background thread can drain.

**Generator**: `IpcRequest` arbitrary variant generator, sequential sends up to `CHANNEL_CAPACITY * 2` attempts.

**Command** (if proptest is added): `cargo test -p vb_ui --test ipc_bridge_proptest` or similar.

## Compilation Scope
- `cargo build -p vb_ui --lib` must succeed
- `cargo test -p vb_ui --lib` must compile and pass
- `cargo clippy -p vb_ui --lib --bins --examples -- -D warnings` must pass

## Waivers
- **WAIVER-TLA-001**: No TLA+ model — reason documented in `tla-spec.md`
- **WAIVER-LEAN-001**: No theorem kernel — reason documented in `lean-contract.md`
- **WAIVER-VERUS-001**: Verus not required for this change — bounded channel is a stdlib API change provable by unit tests and compile verification. The `send()` backpressure logic is pure deterministic Rust with exhaustively testable error paths.
