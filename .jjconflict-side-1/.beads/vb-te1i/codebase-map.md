# vb-te1i Codebase Map: Binary IPC

## Bead
- **ID**: vb-te1i
- **Title**: bdd: Binary IPC acceptance scenarios
- **Focus**: Binary IPC, message protocols, inter-process communication, serialization for BDD scenarios

## Scope Summary

This bead targets executable BDD acceptance scenarios for the Binary IPC public surface:
- Frame encoding/decoding (24-byte fixed header + payload)
- Command dispatch (16 v1 commands)
- Backpressure and queue capacity (IpcError::Full)
- Malformed frame rejection before payload allocation
- Correlation ID preservation
- Shutdown and health probe behavior

## Primary Crate: vb_ipc

**Location**: `crates/vb_ipc/src/`

### Core Frame Structure

| File | Symbol/Type | Purpose |
|------|-------------|---------|
| `constants.rs` | `IPC_MAGIC = 0x5642_4C54` (VBLT) | Frame magic bytes |
| `constants.rs` | `IPC_VERSION = 1` | Supported schema version |
| `constants.rs` | `IPC_HEADER_LEN = 24` | Fixed header size |
| `frame_types.rs` | `IpcFrameHeader` | Fixed 24-byte header struct |
| `frame_types.rs` | `IpcFrame` | Decoded frame with bounded payload |
| `frame.rs` | `encode_frame()`, `decode_frame_header()`, `validate_frame_magic()` | Frame encoding/validation |
| `frame.rs` | `read_frame_header()`, `read_frame_payload()` | Streaming read |
| `bounded.rs` | `MaxPayloadBytes` | 1 MiB default payload bound |
| `bounded.rs` | `QueueCapacity` | Ingress queue capacity |

### Wire Layout (24 bytes)

```
Byte 0-3:   MAGIC (little-endian VBLT = 0x5642_4C54)
Byte 4-5:   VERSION (u16, must be 1)
Byte 6-7:   COMMAND (u16, 1-16)
Byte 8-9:   FLAGS (u16)
Byte 10-11: RESERVED (must be 0)
Byte 12-19: CORRELATION (u64)
Byte 20-23: PAYLOAD_LEN (u32)
```

### Commands (16 total)

| ID | Command | Payload |
|----|---------|---------|
| 1 | SubmitRun | `SubmitRunPayload` |
| 2 | SubmitRunInline | `SubmitRunPayload` |
| 3 | CancelRun | `{ run_id: RunId }` |
| 4 | InspectRun | `{ run_id: RunId }` |
| 5 | ListEvents | `{ run_id: RunId, from_sequence: u64 }` |
| 6 | AnswerAsk | `{ run_id, ticket, answer, taint }` |
| 7 | CompleteAction | `{ run_id, ticket, output }` |
| 8 | FailAction | `{ run_id, ticket, error }` |
| 9 | DrainTrace | `{ run_id, max_records }` |
| 10 | Health | None |
| 11 | Shutdown | None |
| 12 | ListRuns | `{ limit, workflow }` |
| 13 | GetMetrics | None |
| 14 | GetWorkflowGraph | `{ digest }` |
| 15 | GetTaintReport | `{ digest }` |
| 16 | VerifyWorkflow | `{ digest }` |

### Error Types

**File**: `error.rs`

| Error | Diagnostic Code | Runtime Code | When |
|-------|----------------|--------------|-------|
| `Full` | 0x3001 | QUEUE_FULL | Queue at capacity |
| `Disconnected` | 0x3002 | None | Queue disconnected |
| `PayloadTooLarge` | 0x3003 | IPC_PAYLOAD_TOO_LARGE | Payload > max |
| `InvalidMagic` | 0x3004 | IPC_FRAME_INVALID | Magic != VBLT |
| `UnsupportedVersion` | 0x3005 | IPC_FRAME_INVALID | Version != 1 |
| `UnknownCommand` | 0x3006 | IPC_FRAME_INVALID | Command ID invalid |
| `ReservedNonZero` | 0x3007 | IPC_FRAME_INVALID | Reserved != 0 |
| `PayloadLengthMismatch` | 0x3008 | IPC_FRAME_INVALID | Header/actual disagree |
| `HeaderEncodeFailed` | 0x3009 | None | Encoding failed |
| `HeaderDecodeFailed` | 0x300A | IPC_FRAME_INVALID | Decoding failed |
| `PayloadLengthOutOfRange` | 0x300B | IPC_PAYLOAD_TOO_LARGE | u32 won't fit usize |
| `PayloadEncodeFailed` | 0x300C | None | Postcard encode failed |
| `PayloadDecodeFailed` | 0x300D | IPC_FRAME_INVALID | Postcard decode failed |
| `ResponseDecodeFailed` | 0x300E | IPC_FRAME_INVALID | Response decode failed |

### Client API

**File**: `client.rs`

```rust
pub struct IpcClient { stream: UnixStream }

impl IpcClient {
    pub fn connect(socket_path: &Path) -> Result<Self, IpcClientError>
    pub fn send_command(&mut self, command: IpcCommand, correlation: u64, payload: &IpcPayload) -> Result<(), IpcClientError>
    pub fn send_raw(&mut self, command: IpcCommand, correlation: u64, payload: &[u8]) -> Result<(), IpcClientError>
    pub fn recv_response_header(&mut self) -> Result<IpcFrameHeader, IpcClientError>
    pub fn recv_response_payload(&mut self, header: &IpcFrameHeader) -> Result<Vec<u8>, IpcClientError>
    pub fn recv_response(&mut self, max_payload: MaxPayloadBytes) -> Result<(IpcFrameHeader, IpcResponse), IpcClientError>
    pub fn health(&mut self, correlation: u64) -> Result<(), IpcClientError>
    pub fn shutdown(&mut self, correlation: u64) -> Result<(), IpcClientError>
}
```

### Server API

**File**: `server/mod.rs`

```rust
pub struct IpcServer { ... }
pub fn serve_ipc(server: &mut IpcServer, runtime: &mut Runtime, timeout: Option<Duration>) -> Result<bool, IpcServerError>
```

**File**: `server/impl_.rs`

```rust
impl IpcServer {
    pub fn bind(socket_path: &Path) -> Result<Self, IpcServerError>
    pub fn poll_once(&mut self, runtime: &mut Runtime, timeout: Option<Duration>) -> Result<bool, IpcServerError>
}
```

### Handlers

**File**: `server/handlers.rs`

| Handler | Function | Response |
|---------|----------|----------|
| Health | `handle_health()` | `IpcResponse::Healthy` |
| Shutdown | `handle_shutdown(runtime)` | `IpcResponse::ShuttingDown` |
| SubmitRun | `handle_submit_run(header, payload, runtime, resolver)` | `IpcResponse::AcceptedRun` |
| CancelRun | `handle_cancel_run(...)` | `IpcResponse::Inspected` |
| InspectRun | `handle_inspect_run(...)` | `IpcResponse::Inspected` |
| ListEvents | `handle_list_events(...)` | `IpcResponse::Events` |
| ListRuns | `handle_list_runs(...)` | `IpcResponse::RunList` |
| DrainTrace | `handle_drain_trace(...)` | `IpcResponse::TraceCount` |
| GetMetrics | `handle_get_metrics(...)` | `IpcResponse::Metrics` |
| GetWorkflowGraph | `handle_get_workflow_graph(...)` | `IpcResponse::WorkflowGraph` |
| GetTaintReport | `handle_get_taint_report(...)` | `IpcResponse::TaintReport` |
| VerifyWorkflow | `handle_verify_workflow(...)` | `IpcResponse::VerifyWorkflow` |
| AnswerAsk | `handle_answer_ask(...)` | `IpcResponse::AcceptedRun` |
| CompleteAction | `handle_complete_action(...)` | `IpcResponse::AcceptedRun` |
| FailAction | `handle_fail_action(...)` | `IpcResponse::AcceptedRun` |

### Response Types

**File**: `server/mod.rs`

```rust
pub enum IpcResponse {
    AcceptedRun { run_id: u64 },
    Healthy,
    ShuttingDown,
    TraceCount { count: u32 },
    Events { events: Vec<IpcTraceEvent> },
    Inspected { run_id: u64 },
    BadRequest,
    PayloadError { diagnostic: u16, message: String },
    CommandPayloadMismatch,
    WorkflowResolutionRequired,
    WorkflowResolutionUnsupported,
    WorkflowDigestMismatch,
    CountOutOfRange { actual: usize, limit: u32 },
    FrameError { message: String },
    RuntimeError { message: String },
    RunList { runs: Vec<RunSummary> },
    Metrics(RuntimeMetrics),
    VerifyWorkflow { result: VerificationResult },
    TaintReport { sources, sinks, finish_safe, paths },
    WorkflowGraph { nodes, edges },
}
```

### Payload Types

**File**: `payloads.rs`

```rust
pub enum IpcPayload {
    SubmitRun(SubmitRunPayload),
    SubmitRunInline(SubmitRunPayload),
    CancelRun { run_id: RunId },
    InspectRun { run_id: RunId },
    ListEvents { run_id: RunId, from_sequence: u64 },
    AnswerAsk { run_id: RunId, ticket: u64, answer: Vec<u8>, taint: Option<Taint> },
    CompleteAction { run_id: RunId, ticket: u64, output: Vec<u8> },
    FailAction { run_id: RunId, ticket: u64, error: Vec<u8> },
    DrainTrace { run_id: RunId, max_records: u32 },
    ListRuns { limit: u32, workflow: Option<WorkflowDigest> },
    Health,
    Shutdown,
    GetMetrics,
    GetTaintReport { digest: WorkflowDigest },
    GetWorkflowGraph { digest: WorkflowDigest },
    VerifyWorkflow { digest: WorkflowDigest },
}
```

## Related Crates

| Crate | Role | Key Files |
|-------|------|-----------|
| `vb_core` | Core types | `RunId`, `WorkflowDigest`, `DiagnosticCode` |
| `vb_runtime` | Runtime execution | `Runtime`, `Shard` |
| `vb_storage` | Journal persistence | Used by IPC for durable runs |
| `vb_cli` | CLI integration | `agent_context.rs` (IPC server lifecycle) |

## Tests

### Unit Tests in vb_ipc

| File | Coverage |
|------|----------|
| `src/tests.rs` | Queue backpressure, command IDs, header roundtrips, bad magic rejection |
| `src/frame/tests.rs` | Frame encode/decode, adversarial inputs, boundary conditions |
| `src/client/tests.rs` | Client connect/send/recv |
| `src/server/impl_tests.rs` | Server poll, client handling |
| `src/queue/tests/array_queue_tests.rs` | Queue capacity, full/empty signaling |

### Integration Tests

| File | Purpose |
|------|---------|
| `vb_hxm0_acceptance_catalog.rs` | Catalog VB-BDD-CATALOG-005 (binary IPC rejects malformed frames) |
| `vb_qi37_2_4_integration_budget_errors.rs` | IPC payload bounds (max_ipc_payload_bytes) |
| `vb_y1zq_*_boundary_*` | IPC frame boundary classification |

## BDD Scenarios (from bead)

### Happy Paths

1. **test_ipc_submit_run_roundtrips_when_frame_is_valid**
   - Given: Valid IPC frame with SubmitRun command
   - When: Frame is encoded, sent, received, and decoded
   - Then: Response preserves correlation ID and returns AcceptedRun

2. **test_ipc_health_and_shutdown_return_expected_responses**
   - Given: Connected IPC client
   - When: Health and Shutdown commands are sent
   - Then: Health returns Healthy, Shutdown returns ShuttingDown

### Error Paths

3. **test_ipc_rejects_bad_magic_before_payload_allocation**
   - Given: Frame with invalid magic (not VBLT)
   - When: Frame header is decoded
   - Then: InvalidMagic error returned BEFORE any payload allocation

4. **test_ipc_returns_queue_full_when_backpressure_limit_is_hit**
   - Given: MemoryIngress queue at capacity
   - When: New frame is submitted
   - Then: IpcError::Full returned

## Risk Tags

- **parser/codec**: Binary frame parsing with adversarial input (magic, version, bounds)
- **concurrency**: Server poll loop with mio, client socket I/O
- **performance**: Frame encoding/decoding hot path, bounded allocations
- **persistence**: IPC commands trigger durable runs (Fjall journal)
- **public_api**: IPC socket endpoint is public surface

## Required Verifier Modes

Based on risk assessment:

| Risk | Recommended Verification |
|------|-------------------------|
| Frame parsing adversarial | Kani (header decode harness), fuzz (frame decode) |
| Queue backpressure | Property test (capacity exhaustion) |
| Concurrency | Loom (server poll, client/server interaction) |
| Codec roundtrip | Unit tests (existing in frame/tests.rs) |

## Downstream Owners

- **State 3 (contract)**: `rust-contract` skill
- **State 4-6 (proof)**: `proof-planner`, `proof-writer` for Kani/Loom lanes
- **State 7-9 (test)**: `test-planner`, `test-writer` for BDD scenario tests
- **State 10 (implement)**: `holzman-rust` for any handler changes

## Open Questions

1. **executable_evidence_target**: VB-BDD-CATALOG-005 currently has `None` for executable_evidence_target - needs implementation in vb-te1i
2. **IPC server fixture**: Need isolated Unix socket + runtime fixture for integration tests
3. **Correlation ID tracking**: Verify correlation IDs propagate through server dispatch to response

## Files to Read for Contract Phase

- `velvet-ballistics-MASTER.md` (binary IPC section)
- `crates/vb_ipc/src/frame.rs` (full)
- `crates/vb_ipc/src/server/handlers.rs` (full)
- `crates/vb_ipc/src/tests.rs` (backpressure tests)
- `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` (catalog structure)