# vb-0253.3: IPC Bridge Bounded Channel — Codebase Map

## Scope

Single-file change: `crates/vb_ui/src/ipc_bridge.rs`

## 1. Current `std::sync::mpsc::channel` Usage

### Channel Creation (ipc_bridge.rs:144-145)

```rust
let (req_tx, req_rx) = mpsc::channel::<IpcRequest>();
let (rep_tx, rep_rx) = mpsc::channel::<IpcReply>();
```

Both channels are **unbounded** (built via `mpsc::channel()`). No capacity limit is applied.

- **Request channel** (`Sender<IpcRequest>` / `Receiver<IpcRequest>`): UI thread sends `IpcRequest` variants; background IPC thread receives.
- **Reply channel** (`Sender<IpcReply>` / `Receiver<IpcReply>`): Background thread sends `IpcReply` variants; UI thread polls via `try_recv`.

### Ownership Chain

| Field | Type | Owner |
|-------|------|-------|
| `IpcBridge.tx` | `Sender<IpcRequest>` | `IpcBridge` (UI thread) |
| `IpcBridge.rx` | `Receiver<IpcReply>` | `IpcBridge` (UI thread) |
| `req_rx` | `Receiver<IpcRequest>` | background thread (moved at line 150) |
| `rep_tx` | `Sender<IpcReply>` | background thread (moved at line 150) |

### Where Unbounded Channels Are Problematic

- **UI thread**: calls `bridge.send(request)` which can succeed even under load. If the background IPC thread stalls (e.g., server socket blocks), `IpcRequest` messages accumulate unboundedly in the request channel.
- **Background thread**: sends replies without backpressure. If `poll()` is not called for many frames, `IpcReply` messages accumulate unboundedly in the reply channel.
- **`IpcRequest::SubmitRun` with large `input: Vec<u8>`** — unbounded channel can absorb arbitrarily large serialized workflows with no signal back to the caller.

## 2. `recv_timeout` Polling Behavior

### Thread Loop (ipc_bridge.rs:228-233)

```rust
loop {
    let request = match rx.recv_timeout(Duration::from_millis(RECV_TIMEOUT_MS)) {
        Ok(req) => req,
        Err(mpsc::RecvTimeoutError::Timeout) => continue,
        Err(mpsc::RecvTimeoutError::Disconnected) => break,
    };
    // ...
}
```

- `RECV_TIMEOUT_MS = 100` (line 217)
- On `Timeout`, the loop `continue`s — it does NOT process any pending messages from the server socket.
- On `Disconnected`, the thread exits gracefully.
- The timeout keeps the thread responsive to shutdown signals but introduces **100ms latency** on request processing under load.

### Interaction With Unbounded Channel

The timeout does NOT drain the request channel. If requests arrive faster than the server can process them, the channel grows without bound while the thread waits on `recv_timeout`. The background thread can only process one request at a time — each request involves a full send+recv round-trip to the server.

## 3. Typed Commands/Requests

### `IpcRequest` Variants (ipc_bridge.rs:20-93)

| Variant | Fields | Notes |
|---------|--------|-------|
| `Connect` | `socket_path: PathBuf` | Establishes Unix domain socket connection |
| `Disconnect` | — | Drops `IpcClient` |
| `SubmitRun` | `run_id: RunId`, `workflow: WorkflowDigest`, `input: Vec<u8>` | **Largest payload** — `input` can be large |
| `CancelRun` | `run_id: RunId` | Fire-and-forget |
| `InspectRun` | `run_id: RunId` | |
| `ListEvents` | `run_id: RunId`, `from_sequence: u64` | |
| `AnswerAsk` | `run_id: RunId`, `ticket: u64`, `answer: Vec<u8>` | `answer` can be large |
| `DrainTrace` | `run_id: RunId`, `max_records: u32` | |
| `VerifyWorkflow` | `digest: WorkflowDigest` | |
| `RequestTaintReport` | `run_id: RunId`, `digest: WorkflowDigest` | |
| `RequestWorkflowGraph` | `digest: WorkflowDigest` | |
| `Health` | — | |
| `Shutdown` | — | |

### `IpcReply` Variants (ipc_bridge.rs:96-129)

| Variant | Payload | Notes |
|---------|---------|-------|
| `Connected` | — | |
| `Disconnected` | — | |
| `ConnectionFailed` | `String` | |
| `RunAccepted` | `RunId` | |
| `RunCancelled` | `RunId` | |
| `Inspected` | `IpcResponse` | |
| `Events` | `IpcResponse` | Buffered in `events_buffer` |
| `TraceCount` | `u32` | |
| `Healthy` | — | |
| `ShuttingDown` | — | |
| `Error` | `String` | |
| `NotImplemented` | `String` | |
| `VerifyWorkflowResult` | `IpcResponse` | |
| `TaintReportReceived` | `IpcResponse` | |
| `WorkflowGraphReceived` | `IpcResponse` | |

## 4. Backpressure and Error Handling

### Current Backpressure: None

- `send(&self, request: IpcRequest) -> Result<(), String>` (line 182-186): Returns error only if the channel is disconnected (thread died). If the channel is open but full, `send` blocks the calling thread — which is the UI render loop. This blocks Makepad's render loop until the background thread processes the message.

### Error Handling Pattern (all branches)

```rust
if let Err(_err) = tx.send(IpcReply::Error(...)) {
    return; // Silently drops the error and exits the thread
}
```

Every `send` on the reply channel ignores the error and exits the thread on failure. This is a silent failure mode.

### `poll()` (ipc_bridge.rs:189-203)

Uses `try_recv` — non-blocking. Drains all available replies per call. No backpressure signal is generated.

## 5. Test Surfaces for IpcBridge

### Unit Tests in `ipc_bridge.rs` (lines 629-893)

| Test | What It Covers |
|------|----------------|
| `bridge_new_creates_channels_and_thread` | `new()` creates thread, `poll()` is empty initially |
| `bridge_connect_to_nonexistent_socket_fails` | `Connect` to invalid path → `ConnectionFailed` |
| `bridge_send_without_connect_returns_not_connected_error` | Any request without client → `Error("Not connected")` |
| `bridge_submit_run_without_connect_returns_not_connected_error` | `SubmitRun` without client → error |
| `bridge_answer_ask_without_connect_returns_not_connected_error` | `AnswerAsk` without client → error |
| `bridge_drain_trace_without_connect_returns_not_connected_error` | `DrainTrace` without client → error |
| `next_correlation_advances` | Counter increments |
| `next_correlation_wraps_at_max` | Wrapping arithmetic |
| `reply_from_response_*` (4 tests) | Response mapping |
| `reply_from_submit_*` (4 tests) | Submit response mapping |
| `reply_from_answer_*` (4 tests) | Answer response mapping |
| `reply_from_drain_trace_*` (4 tests) | Drain-trace response mapping |

### Tests in `ipc_wiring.rs` (ipc_wiring.rs:444-1483+)

`IpcAppWiring` wraps `IpcBridge` and adds routing tests:
- `verify_workflow_sends_correct_request`
- `request_taint_report_sends_correct_request`
- `request_workflow_graph_sends_correct_request`
- `verify_workflow_without_connect_returns_not_connected_error`
- `request_taint_report_without_connect_returns_not_connected_error`

### Integration Test Pattern

All tests using `IpcBridge` follow this pattern:
1. Create `IpcBridge::new()`
2. Call `bridge.send(IpcRequest::...)`
3. Spin on `bridge.poll()` with a 500ms deadline, checking for expected `IpcReply` variant.

## 6. Related Files

| File | Role |
|------|------|
| `crates/vb_ui/src/ipc_bridge.rs` | **Target file** — owns channels, background thread, request/reply types |
| `crates/vb_ui/src/ipc_wiring.rs` | Consumer of `IpcBridge` — routes replies to `AppState` |
| `crates/vb_ui/src/replay/controller.rs` | Owns a second `IpcBridge` instance for replay |
| `crates/vb_ipc/src/bounded.rs` | `QueueCapacity`, `MaxPayloadBytes`, `BoundedPayload` — existing bounded types in sibling crate |
| `crates/vb_ipc/src/payloads.rs` | `IpcPayload` enum — typed command payloads |
| `crates/vb_ipc/src/client.rs` | `IpcClient` — the actual socket client used by the background thread |
| `crates/vb_ui/src/lib.rs` | Public re-exports `ipc_bridge` module |

## 7. Public API Summary

### `ipc_bridge.rs` Public Items

| Item | Kind | Visibility |
|------|------|------------|
| `IpcBridge` | struct | `pub` |
| `IpcRequest` | enum | `pub` |
| `IpcReply` | enum | `pub` |
| `IpcBridge::new()` | fn | `pub` |
| `IpcBridge::send(&self, IpcRequest) -> Result<(), String>` | fn | `pub` |
| `IpcBridge::poll(&mut self) -> Vec<IpcReply>` | fn | `pub` |
| `IpcBridge::is_connected(&self) -> bool` | fn | `pub` |

### Bounded Channel Design Points

To convert to bounded channels:
1. Replace `mpsc::channel()` with `mpsc::sync_channel(capacity)` in `IpcBridge::default()`
2. `send()` on a full bounded channel returns `Err(TrySendError::Full(...))` — needs handling
3. Need a capacity constant — could reuse `vb_ipc::QueueCapacity` or define a new constant
4. Need typed backpressure: `TrySendError<IpcRequest>` or a custom `IpcBridgeError` enum
5. `recv_timeout` behavior can remain unchanged (it already works with bounded channels)
6. The `poll()` method uses `try_recv` which works identically with bounded channels
