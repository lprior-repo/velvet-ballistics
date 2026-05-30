# Architectural Drift Report: vb_ipc/src/server/handlers.rs

## Line Count Violation

| Metric | Value |
|--------|-------|
| **Current lines** | 3998 |
| **Limit** | 300 |
| **Ratio** | 13.3x OVER |
| **Overage** | 3698 lines |

---

## File Layout Analysis

```
Lines 1-62:    Constants, imports
Lines 64-124:  Utility functions (sanitize_*)
Lines 127-577: IPC handlers (handle_*) + submit_resolved_workflow
Lines 580-620: node_kind_label
Lines 623-803: collect_edges_from_node
Lines 807-879: handle_get_workflow_graph
Lines 885-983: handle_get_taint_report
Lines 987-1013: bfs_forward
Lines 1019-1107: all_successors
Lines 1110-1132: enqueue_successors
Lines 1134-3998: TEST MODULE (2865 lines)
```

---

## Functions Requiring Extraction (with line estimates)

### Tier 1: Workflow Graph Analysis Module (~700 lines)

| Function | Lines | Recommendation |
|----------|-------|----------------|
| `node_kind_label` | 580-620 (41L) | Move to `graph/node_label.rs` |
| `collect_edges_from_node` | 623-803 (181L) | Move to `graph/edge_collector.rs` |
| `all_successors` | 1019-1107 (89L) | Move to `graph/successors.rs` |
| `enqueue_successors` | 1110-1132 (23L) | Move to `graph/successors.rs` |
| `bfs_forward` | 987-1013 (27L) | Move to `graph/bfs.rs` |

**Subtotal: ~361 lines**

### Tier 2: Taint Analysis Handler (~150 lines)

| Function | Lines | Recommendation |
|----------|-------|----------------|
| `handle_get_taint_report` | 888-983 (96L) | Move to `handlers/taint.rs` |

### Tier 3: Workflow Graph Handler (~80 lines)

| Function | Lines | Recommendation |
|----------|-------|----------------|
| `handle_get_workflow_graph` | 807-879 (73L) | Move to `handlers/workflow_graph.rs` |

### Tier 4: Verification Handler (~110 lines)

| Function | Lines | Recommendation |
|----------|-------|----------------|
| `handle_verify_workflow` | 477-577 (101L) | Move to `handlers/verify.rs` |

### Tier 5: Metrics Handler (~50 lines)

| Function | Lines | Recommendation |
|----------|-------|----------------|
| `handle_get_metrics` | 430-474 (45L) | Move to `handlers/metrics.rs` |

### Tier 6: Submit/Run Lifecycle (~200 lines)

| Function | Lines | Recommendation |
|----------|-------|----------------|
| `handle_submit_run` | 127-145 (19L) | Move to `handlers/submit.rs` |
| `handle_submit_run_inline` | 148-155 (8L) | Move to `handlers/submit.rs` |
| `submit_resolved_workflow` | 363-403 (41L) | Move to `handlers/submit.rs` |
| `handle_cancel_run` | 158-172 (15L) | Move to `handlers/submit.rs` |
| `handle_inspect_run` | 175-197 (23L) | Move to `handlers/submit.rs` |
| `handle_list_events` | 200-215 (16L) | Move to `handlers/submit.rs` |

### Tier 7: Action Handlers (~145 lines)

| Function | Lines | Recommendation |
|----------|-------|----------------|
| `handle_answer_ask` | 218-282 (65L) | Move to `handlers/action.rs` |
| `handle_complete_action` | 285-321 (37L) | Move to `handlers/action.rs` |
| `handle_fail_action` | 324-361 (38L) | Move to `handlers/action.rs` |

### Tier 8: List/Misc Handlers (~50 lines)

| Function | Lines | Recommendation |
|----------|-------|----------------|
| `handle_list_runs` | 406-427 (22L) | Move to `handlers/list.rs` |
| `handle_ping` | 107-109 (3L) | Keep in main (trivial) |
| `handle_health` | 112-114 (3L) | Keep in main (trivial) |
| `handle_shutdown` | 117-124 (8L) | Move to `handlers/control.rs` |

### Remaining Utilities (~100 lines)

| Function | Lines | Recommendation |
|----------|-------|----------------|
| `decode_payload` | 23-26 (4L) | Keep in main or move to `codec.rs` |
| `ipc_error_response` | 28-33 (6L) | Keep in main |
| `sanitize_runtime_error` | 74-82 (9L) | Move to `sanitize.rs` |
| `sanitize_validation_detail` | 87-104 (18L) | Move to `sanitize.rs` |
| Constants | 36-66 (31L) | Move to `constants.rs` |

---

## Primitive Obsession Violations

### 1. `ticket: u64` in `handle_answer_ask` (line 220)
```rust
// VIOLATION: Raw u64 used where a TicketId type should exist
pub fn handle_answer_ask(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::AnswerAsk {
        run_id,
        ticket,  // <-- raw u64, no validation wrapper
        answer,
        taint,
    }) = ...
```
**FIX**: Create `TicketId(u64)` newtype with `TryFrom<u64>` bounds validation.

### 2. `ticket: u64` in `handle_complete_action` (line 288)
Same violation as above.

### 3. `ticket: u64` in `handle_fail_action` (line 327)
Same violation as above.

### 4. `run_id: vb_core::RunId` — Actually OK
This one uses proper `RunId` wrapper, so it's the exception that proves the rule.

### 5. `limit: u32` in `handle_list_runs` (line 407)
```rust
let Ok(IpcPayload::ListRuns { limit, workflow }) = ...
let capped_limit = limit.min(MAX_LIST_RUNS_LIMIT);
```
**FIX**: `limit` should be `NonZeroU32` or a bounded `ListLimit` type.

### 6. `from_sequence: u32` in `handle_list_events` (line 202)
```rust
from_sequence,  // <-- raw u32, no domain meaning
```
**FIX**: Create `SequenceOffset(u32)` or similar.

### 7. `message: String` for errors throughout
```rust
IpcResponse::RuntimeError {
    message: sanitize_runtime_error(&e),
}
```
**FIX**: Error messages should be `ErrorMessage(String)` newtype.

### 8. `title: String` in workflow graph nodes (line 872)
```rust
title,  // constructed from String operations, not a domain type
```
**FIX**: Create `NodeTitle(String)` or use `Display` impl.

### 9. `label: Option<String>` in `EdgeDescriptor` (line 637)
```rust
label: Some(format!("branch_{i}")),  // String interpolation at call site
```
**FIX**: `EdgeLabel(String)` newtype.

---

## Missing Domain Types (should exist but raw primitives used)

| Location | Raw Type | Missing Domain Type |
|----------|----------|---------------------|
| `handle_answer_ask` | `u64` ticket | `TicketId` |
| `handle_complete_action` | `u64` ticket | `TicketId` |
| `handle_fail_action` | `u64` ticket | `TicketId` |
| `handle_list_events` | `u32` sequence | `SequenceOffset` |
| `handle_list_runs` | `u32` limit | `RunListLimit` |
| Throughout | `String` message | `ErrorMessage` |
| `handle_verify_workflow` | `String` detail | `ValidationDetail` |
| `collect_edges_from_node` | `String` label | `EdgeLabel` |
| `handle_get_workflow_graph` | `String` title | `NodeTitle` |
| All handlers | `&[u8]` payload | `WirePayload` |
| `decode_payload` | `&[u8]` | `EncodedPayload` |

---

## Parse Don't Validate Violations

### 1. Ticket bounds check is validation, not parsing (line 237-239)
```rust
let Some(ask_step) = step_from_ticket(ticket) else {
    return IpcResponse::BadRequest;
};
```
The `step_from_ticket` extracts meaning but the ticket is still raw `u64` from wire. The **parse** should produce a `TicketId` that carries the validated step index internally. Currently validation happens after parsing into raw `u64`.

### 2. Digest mismatch check happens after resolution (line 387-388)
```rust
let workflow = match resolver.resolve_workflow(submit.workflow) {
    ...
};
if workflow.digest() != submit.workflow {  // Post-parse validation, not parse
    return IpcResponse::WorkflowDigestMismatch;
}
```
**FIX**: The resolver should return an already-validated `ResolvedWorkflow` that cannot exist with wrong digest.

### 3. Answer bytes decoded as postcard THEN validated (line 254-260)
```rust
let value = match postcard::from_bytes::<SlotValue>(&answer) {
    Ok(v) => v,
    Err(_) => {
        return IpcResponse::RuntimeError { ... };  // Validation after parse
    }
};
```
**FIX**: `AnswerBytes` should be a wrapper that validates on construction.

### 4. Action output decoded then checked for length (line 307-310)
```rust
let decoded_output = match decode_payload::<crate::IpcActionOutputPayload>(&output) {
    Ok(d) => d,
    Err(response) => return response,
};
if output.len() > MAX_ACTION_OUTPUT_LEN {  // Length check after decode
```
**FIX**: `ActionOutputBytes` should carry its own length guarantee.

### 5. Input length check AFTER decode (line 369-376)
```rust
pub fn submit_resolved_workflow(...) -> IpcResponse {
    if submit.input.len() > MAX_SUBMIT_INPUT_LEN {  // Validation AFTER deserialization
        return IpcResponse::PayloadError { ... };
    }
```
**CRITICAL**: Postcard has already allocated `submit.input` before this check. The allocation attack is already done.

**FIX**: `SubmitRunPayload.input` should be a `BoundedInput<N>` that limits deserialization size. The length check is useless as a security control post-decode.

---

## Recommended Module Split

```
vb_ipc/src/server/handlers.rs  (TARGET: ~300 lines)
├── Utility re-exports
├── Constants re-exports  
└── Dispenser functions (route to submodules)

vb_ipc/src/server/handlers/
├── mod.rs                    (re-exports, ~50 lines)
├── submit.rs                 (~120 lines)      handle_submit_run, handle_cancel_run, etc.
├── action.rs                 (~140 lines)      handle_answer_ask, handle_complete_action, handle_fail_action
├── list.rs                   (~30 lines)       handle_list_runs
├── verify.rs                 (~110 lines)      handle_verify_workflow
├── workflow_graph.rs         (~80 lines)       handle_get_workflow_graph
├── taint.rs                  (~100 lines)      handle_get_taint_report
├── metrics.rs                (~50 lines)       handle_get_metrics
├── control.rs                (~15 lines)       handle_shutdown, handle_ping, handle_health
└── codec.rs                  (~20 lines)       decode_payload, ipc_error_response

vb_ipc/src/server/handlers/graph/
├── mod.rs
├── node_label.rs             (~45 lines)       node_kind_label
├── edge_collector.rs         (~185 lines)      collect_edges_from_node
├── successors.rs             (~115 lines)      all_successors, enqueue_successors
└── bfs.rs                    (~30 lines)       bfs_forward

vb_ipc/src/server/handlers/sanitize.rs
├── mod.rs
├── runtime_error.rs          (~15 lines)       sanitize_runtime_error
└── validation_detail.rs      (~20 lines)       sanitize_validation_detail
```

---

## Priority Refactoring Order

1. **IMMEDIATE (Security)**: Fix `BoundedInput` deserialization — the current length check after decode is theater, not security
2. **Phase 1**: Extract graph analysis to `handlers/graph/` module
3. **Phase 2**: Extract taint analysis to `handlers/taint.rs`
4. **Phase 3**: Extract handlers to individual files by domain
5. **Phase 4**: Create domain newtypes for `TicketId`, `SequenceOffset`, `ErrorMessage`
6. **Phase 5**: Move tests alongside implementation (per crate)

---

## Evidence

- File: `crates/vb_ipc/src/server/handlers.rs`
- Total lines: 3998
- Test module alone: 2865 lines (71.6% of file)
- Non-test production code: ~1133 lines
- After splitting: target ~300 lines main + ~1100 lines in submodules

**SEVERITY: CRITICAL** — File is 13.3x over the 300-line architectural limit and contains multiple primitive obsession violations that leak raw types across the IPC boundary.
