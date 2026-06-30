# vb_ipc Handlers Codebase Map

## Bead
vb-qi37.26.1 - fix: vb_ipc typed handler compile errors blocking workspace-tests

## Root Cause
25 E0308 mismatched-type errors in `crates/vb_ipc/src/server/handlers.rs` caused by using `String` literals where strongly-typed enums were expected. This occurred during a module split/restore cycle:

1. `d2ed2ba2` - deleted `handlers.rs`, created `handlers/` directory with split submodules
2. `988fb18a` - removed `handlers/mod.rs` to fix E0761 conflicts, restoring `handlers.rs` from pre-split state
3. The restored `handlers.rs` contained older code using `String::from("...")` for fields that had already been migrated to enums in `payloads.rs`
4. `0ebc5270` fixed the errors by replacing String literals with proper enum variants

## Affected Types (all defined in `crates/vb_ipc/src/payloads.rs`)

| Type | Fields / Variants | Where Used in handlers.rs |
|------|-------------------|---------------------------|
| `EdgeType` | Branch, LoopBody, LoopExit, ParallelBranch, ParallelJoin, Fallthrough, ErrorHandler, Jump | `collect_edges_from_node`, `handle_get_workflow_graph` |
| `PassFail` | Pass, Fail | `handle_verify_workflow` (CertificateWire.status) |
| `GateKind` | Gate07..Gate15 | `handle_verify_workflow` (CertificateWire.kind) |
| `NodeKind` | Nop, SetConst, Copy, EvalExpr, ... | `handle_get_workflow_graph` (NodeDescriptor.kind) |
| `TaintPathStatus` | Dangerous, Warning | `handle_get_taint_report` (TaintPathWire.status) |

## Primary File
- `crates/vb_ipc/src/server/handlers.rs` (1400+ lines, compiles clean as of HEAD)

## Related Type Definitions
- `crates/vb_ipc/src/payloads.rs` - defines `EdgeType`, `PassFail`, `GateKind`, `NodeKind`, `TaintPathStatus`, `CertificateWire`, `EdgeDescriptor`, `NodeDescriptor`, `TaintPathWire`
- `crates/vb_ipc/src/server/mod.rs` - imports handlers, defines `IpcResponse`, `WorkflowResolver`, `WorkflowResolutionError`
- `crates/vb_ipc/src/server/trace.rs` - provides `typed_events_response` used by `handle_list_events`
- `crates/vb_ipc/src/server/ticket.rs` - provides `action_ticket_from_wire`, `payload_len`, `step_from_ticket`

## Handler Functions in handlers.rs

### Session handlers
- `handle_ping() -> IpcResponse`
- `handle_health() -> IpcResponse`
- `handle_shutdown(runtime: &mut Runtime) -> IpcResponse`

### Query handlers
- `handle_submit_run(header, payload, runtime, resolver) -> IpcResponse`
- `handle_submit_run_inline(payload, runtime, resolver) -> IpcResponse`
- `handle_cancel_run(payload, runtime) -> IpcResponse`
- `handle_inspect_run(payload, runtime) -> IpcResponse`
- `handle_list_events(payload, runtime) -> IpcResponse`
- `handle_list_runs(payload, runtime) -> IpcResponse`
- `handle_get_metrics(runtime) -> IpcResponse`

### Command handlers
- `handle_answer_ask(payload, runtime) -> IpcResponse`
- `handle_complete_action(payload, runtime) -> IpcResponse`
- `handle_fail_action(payload, runtime) -> IpcResponse`

### Event / graph handlers
- `handle_verify_workflow(payload, resolver) -> IpcResponse`
- `handle_get_workflow_graph(payload, resolver) -> IpcResponse`
- `handle_get_taint_report(payload, resolver) -> IpcResponse`

### Helpers
- `decode_payload<T>(payload) -> Result<T, IpcResponse>`
- `sanitize_runtime_error(e) -> String`
- `sanitize_validation_detail(detail) -> String`
- `submit_resolved_workflow(command, submit, runtime, resolver) -> IpcResponse`
- `node_kind_label(kind) -> &'static str`
- `collect_edges_from_node(step, kind, edges)`
- `all_successors(kind) -> Vec<u16>`
- `enqueue_successors(node, node_count, visited, queue)`
- `bfs_forward(parts, start, node_count) -> Vec<u16>`

## Orphaned Files (NOT compiled)
The `crates/vb_ipc/src/server/handlers/` directory contains stale split files from the `d2ed2ba2` refactor that are no longer included in the module tree:
- `handlers/command.rs`
- `handlers/event.rs`
- `handlers/query.rs`
- `handlers/session.rs`

There is no `handlers/mod.rs`, so `pub mod handlers;` in `server/mod.rs` resolves to `handlers.rs` only.

## Public APIs Affected
None - all handler functions are internal to `vb_ipc`. The `IpcResponse` enum variants and wire types (`CertificateWire`, `EdgeDescriptor`, `NodeDescriptor`, `TaintPathWire`) are the public-facing shapes, and their field types were already correctly defined. The bug was purely in the handler implementation constructing these structs with wrong Rust types.

## Compile Status
- Current `cargo check -p vb_ipc`: clean (0 errors)
- The fix is already in place at commit `0ebc5270`
