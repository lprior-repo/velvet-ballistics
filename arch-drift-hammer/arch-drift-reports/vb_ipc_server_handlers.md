# Architectural Drift Report: `vb_ipc/src/server/handlers.rs`

**File**: `crates/vb_ipc/src/server/handlers.rs`
**Total Lines**: 3998
**Limit**: 300
**Status**: `CRITICAL DRIFT`

---

## 1. Line Count Violation

| Metric | Value |
|--------|-------|
| Actual Lines | 3998 |
| Maximum Allowed | 300 |
| Violation | **3698 lines over limit (1233%)** |

This file is **13.3x larger** than the architectural maximum.

---

## 2. DDD Cohesion Analysis

### 2.1 Cohesion Score: `LOW`

This file violates the Single Responsibility Principle by conflating **four distinct architectural concerns**:

| Concern | Functions | Belongs In |
|---------|-----------|------------|
| **IPC Protocol** | `decode_payload`, `ipc_error_response`, `sanitize_runtime_error`, `sanitize_validation_detail` | IPC transport module |
| **Handler Dispatch** | `handle_ping`, `handle_health`, `handle_shutdown`, `handle_submit_run`, `handle_cancel_run`, `handle_inspect_run`, `handle_list_events`, `handle_answer_ask`, `handle_complete_action`, `handle_fail_action`, `handle_list_runs`, `handle_get_metrics`, `handle_verify_workflow`, `handle_get_workflow_graph`, `handle_get_taint_report`, `handle_submit_run_inline`, `submit_resolved_workflow` | IPC application service |
| **Graph Analysis (DOMAIN)** | `node_kind_label`, `collect_edges_from_node`, `bfs_forward`, `all_successors`, `enqueue_successors` | `vb_core::workflow::graph` or `vb_ipc::analysis` |
| **Validation Gate Execution (DOMAIN)** | `handle_verify_workflow` gate execution | `vb_validate` crate |

### 2.2 God Module Smell

The file has grown into a "god module" that:
1. Decodes IPC wire protocol
2. Dispatches commands to runtime
3. Performs **BFS graph traversal** for taint analysis
4. Extracts **control-flow edges** from compiled workflows
5. Executes **validation gates** (Gate07–Gate15)
6. Contains **1500+ lines of tests** embedded in the module

### 2.3 Feature Envy Violations

These functions exhibit **Feature Envy** toward `vb_core::workflow`:

```rust
// VIOLATION: Belongs in vb_core workflow domain, not IPC handlers
fn node_kind_label(kind: &vb_core::workflow::CompiledNodeKind) -> &'static str

fn collect_edges_from_node(
    step: u16,
    kind: &vb_core::workflow::CompiledNodeKind,
    edges: &mut Vec<crate::EdgeDescriptor>,
)

fn bfs_forward(
    parts: &vb_core::workflow::WorkflowParts,
    start: u16,
    node_count: usize,
) -> Vec<u16>

fn all_successors(kind: &vb_core::workflow::CompiledNodeKind) -> Vec<u16>

fn enqueue_successors(
    node: &vb_core::workflow::CompiledNode,
    node_count: usize,
    visited: &mut std::collections::HashSet<u16>,
    queue: &mut std::collections::VecDeque<u16>,
)
```

These functions:
- Operate on `CompiledNodeKind`, `WorkflowParts`, `CompiledNode` — domain types
- Perform graph algorithms (BFS, edge extraction) — domain logic
- Have **no dependency on IPC types** whatsoever
- Are imported from `vb_core::workflow::*` but live in `vb_ipc::server::handlers`

### 2.4 Leaky Abstraction

`handle_get_taint_report` (lines 888–983) and `handle_get_workflow_graph` (lines 807–879) leak domain logic into the IPC layer:
- They iterate `WorkflowParts.nodes` directly
- They pattern-match on `CompiledNodeKind` variants
- They construct `EdgeDescriptor` types for IPC wire format

This is **infrastructure inversion** — the IPC layer should not know the internal structure of `CompiledNodeKind`.

---

## 3. Violations Catalog

### 3.1 File Size Violation (CRITICAL)

```
3998 lines > 300 lines (LIMIT)
```

### 3.2 DDD Boundary Violations (HIGH)

| ID | Violation | Location | Fix |
|----|-----------|----------|-----|
| DDD-01 | Graph analysis functions (`bfs_forward`, `all_successors`, `enqueue_successors`, `collect_edges_from_node`, `node_kind_label`) live in IPC layer | Lines 580–1132 | Extract to `vb_ipc::analysis` or `vb_core::workflow::graph` |
| DDD-02 | `handle_verify_workflow` executes validation gates in IPC layer | Lines 477–577 | Move gate execution to `vb_validate` crate; IPC should only decode request and return result |
| DDD-03 | `handle_get_workflow_graph` builds graph descriptors from domain types | Lines 807–879 | Extract graph construction to domain service; handler should only format response |
| DDD-04 | `handle_get_taint_report` performs BFS traversal in IPC layer | Lines 888–983 | Extract BFS to domain service; IPC handler should invoke domain analyzer |

### 3.3 Primitive Obsession (MEDIUM)

| ID | Location | Issue |
|----|----------|-------|
| PO-01 | Lines 240–249 | `u32::try_from(answer.len())` with manual bounds check; could be `EncodedLen::try_from(answer.len())` |
| PO-02 | Lines 36–66 | Multiple `const MAX_*` values hardcoded as raw `usize`; should use domain-typed constants |

### 3.4 Test Cohesion (HIGH)

The embedded `#[cfg(test)]` module (lines 1134–3998) is **~2864 lines** — 71% of the file. Tests for:
- `node_kind_label` (graph domain)
- `collect_edges_from_node` (graph domain)
- `all_successors` (graph domain)
- `bfs_forward` (graph domain)
- `enqueue_successors` (graph domain)

These tests validate **domain graph algorithms**, not IPC behavior. They should migrate with the domain functions.

---

## 4. Recommended Refactoring

### Phase 1: Extract Graph Analysis Module

Create `crates/vb_ipc/src/server/analysis.rs`:

```rust
// Extract from handlers.rs:
pub mod analysis {
    pub fn node_kind_label(...) { ... }
    pub fn collect_edges_from_node(...) { ... }
    pub fn bfs_forward(...) { ... }
    pub fn all_successors(...) { ... }
    pub fn enqueue_successors(...) { ... }
}
```

### Phase 2: Extract Workflow Graph Service

Create `crates/vb_ipc/src/server/workflow_service.rs`:
- Move `handle_verify_workflow` gate execution here
- Move `handle_get_workflow_graph` graph construction here
- Move `handle_get_taint_report` BFS analysis here

### Phase 3: Slim handlers.rs

Target: ~400 lines (within limit)

```rust
// Should remain in handlers.rs:
pub fn handle_ping() -> IpcResponse { ... }
pub fn handle_health() -> IpcResponse { ... }
pub fn handle_shutdown(...) -> IpcResponse { ... }
pub fn handle_submit_run(...) -> IpcResponse { ... }
pub fn handle_cancel_run(...) -> IpcResponse { ... }
// ... remaining handlers

// Protocol utilities only:
pub fn decode_payload<T: serde::de::DeserializeOwned>(...) -> Result<T, IpcResponse> { ... }
pub(crate) fn sanitize_runtime_error(e: &dyn std::fmt::Display) -> String { ... }
```

---

## 5. Priority Assessment

| Priority | Category | Reason |
|----------|----------|--------|
| **P0** | File Size | 3998 lines is 13.3x over limit; blocks code health |
| **P0** | Domain Leakage | Graph algorithms in IPC layer is architectural error |
| **P1** | Test Location | 71% of file is tests for domain logic |
| **P2** | Primitive Obsession | Minor; defensive code is at least explicit |

---

## 6. Summary

**DDD Smell**: `LOW` — the file conflates IPC transport, application service, and domain logic into one god module.

**Priority**: `P0` — This file requires immediate structural refactoring to extract graph analysis and validation logic into proper domain modules. The IPC layer should be a thin transport wrapper, not a domain analysis engine.

**Effort Estimate**: ~3–4 hours to extract graph analysis module; ~2–3 hours to move validation/taint services.
