# Architectural Drift Report: `event.rs`

**File**: `crates/vb_ipc/src/server/handlers/event.rs`
**Line Count**: 648 (MAX: 300 | OVER BUDGET: +116%)
**Status**: GUILTY

---

## EXECUTIVE SUMMARY

This file is a **graveyard of responsibilities**. It weighs 648 lines and does the work of at least **six distinct modules**. It violates every core Scott Wlaschin DDD principle and constitutes severe architectural drift.

---

## VIOLATION 1: LINE COUNT (+116% OVER BUDGET)

| Metric | Value |
|--------|-------|
| Actual | 648 |
| Max Allowed | 300 |
| Excess | +348 |
| % Over | 116% |

**Verdict**: GUILTY. This file MUST be split.

---

## VIOLATION 2: PRIMITIVE OBSESSION (CRITICAL)

### A. `u16` for Step Indices

```rust
// LINE 175: raw u16 instead of StepIdx newtype
fn collect_edges_from_node(
    step: u16,           // PRIMITIVE
    kind: &vb_core::workflow::CompiledNodeKind,
    edges: &mut Vec<EdgeDescriptor>,
)
```

```rust
// LINE 518: u16 in graph traversal
fn bfs_forward(
    parts: &vb_core::workflow::WorkflowParts,
    start: u16,          // PRIMITIVE
    node_count: usize,
) -> Vec<u16>           // RETURNS PRIMITIVE
```

**Should be**: `StepIdx` newtype wrapping `u16` with validation.

### B. `u16` in Edge Descriptors

```rust
// LINE 186-189: raw u16
edges.push(EdgeDescriptor {
    from: step,          // u16 PRIMITIVE
    to: branch.target.get(), // u16 extracted via .get()
```

**Should be**: `StepIdx` typed field, not raw `u16`.

### C. `u32` for Counts

```rust
// LINE 94-102
let total_checks = match u32::try_from(gate_results.len()) { ... };
let mut pass_count: u32 = 0;
let mut fail_count: u32 = 0;
```

**Should be**: `NonZeroU32` or a `VerificationCounts` value object.

### D. `usize` for Capacity Limits

```rust
// LINE 13-15
const MAX_TAINT_PATH_ENTRIES: usize = 65536;
const MAX_VALIDATION_DETAIL_LEN: usize = 512;
const MAX_WORKFLOW_GRAPH_NODES: usize = 8192;
```

**Should be**: Typed constants with `const MAX_TAINT_PATH_ENTRIES: u16 = 65536;` (since u16 can hold 65535, this is actually wrong too).

---

## VIOLATION 3: SIX RESPONSIBILITIES MINGLED

| Lines | Responsibility | DDD Violation |
|-------|---------------|---------------|
| 17-33 | `sanitize_validation_detail` | Process of sanitizing embedded in handler module |
| 35-133 | `handle_verify_workflow` | **ENTIRE VERIFICATION WORKFLOW** crammed here |
| 135-172 | `node_kind_label` | Pure function, but belongs in a `NodeKind` domain type |
| 174-349 | `collect_edges_from_node` | **GRAPH EDGE EXTRACTION** - a domain service |
| 351-420 | `handle_get_workflow_graph` | **GRAPH BUILDING** - another domain service |
| 422-514 | `handle_get_taint_report` | **TAINT ANALYSIS** - a security bounded context |
| 516-541 | `bfs_forward` | **GRAPH ALGORITHM** - algorithm in infrastructure |
| 543-626 | `all_successors` | **DUPLICATE** successor extraction logic |
| 628-648 | `enqueue_successors` | BFS queue management |

**This is ONE file doing the job of SIX modules.**

---

## VIOLATION 4: DUPLICATED SUCCESSOR EXTRACTION LOGIC

`collect_edges_from_node` (lines 174-349) and `all_successors` (lines 543-626) are **identical in intent** but diverge in naming and helper function structure.

```rust
// VERSION 1: collect_edges_from_node (lines 180-199)
match kind {
    vb_core::workflow::CompiledNodeKind::Choose { branches, otherwise } => {
        for (i, branch) in branches.iter().enumerate() {
            edges.push(EdgeDescriptor {
                from: step,
                to: branch.target.get(),
                label: Some(format!("branch_{i}")),
                edge_type: EdgeType::Branch,
            });
        }
    }
}

// VERSION 2: all_successors (lines 546-555)
match kind {
    vb_core::workflow::CompiledNodeKind::Choose { branches, otherwise } => {
        for branch in branches {
            succs.push(branch.target.get());
        }
        // No labels, no edge types - pure successor extraction
    }
}
```

**Scott Wlaschin Rule**: DUPLICATED CASE SPLITTING ON THE SAME DISCRIMINATED UNION IS A SMELL. This should be ONE function returning a graph structure.

---

## VIOLATION 5: RAW STRING SANITIZATION IN HANDLERS

```rust
// Lines 26-32: Inline path sanitization
truncated
    .replace("/home/", "<redacted>/")
    .replace("/etc/", "<redacted>/")
    // ... 6 more replaces
```

**Should be**: A `PathSanitizer` value object with a `sanitize(&self, path: &Path) -> String` method. This is classic "primitive obsession" - using `String` where a domain concept belongs.

---

## VIOLATION 6: SCATTERED BFS ACROSS THREE FUNCTIONS

```
bfs_forward (516-541)
  └── calls enqueue_successors (628-648)
  └── calls all_successors (543-626)
```

The BFS is split across THREE functions. This is **non-local reasoning** - to understand the algorithm you must jump between three locations.

**Scott Wlaschin Rule**: Algorithms should be in single coherent units unless decomposed by clear abstraction layers.

---

## VIOLATION 7: WORKFLOW VERIFICATION GATE SOUP

Lines 56-93 contain 8 sequential gate validations in a `vec![]` macro. This is **data-driven programming** where the gate registry should be declarative configuration, not inline code.

```rust
let gate_results: Vec<(crate::GateKind, Result<(), vb_validate::ValidationError>)> = vec![
    (crate::GateKind::Gate07ExpressionStackDepth,
     vb_validate::gates::validate_gate_07_expression_stack_depth(&parts)),
    // ... 7 more identical patterns
];
```

**Should be**: A `GateRegistry` that maps `GateKind -> fn(&WorkflowParts) -> Result` and iterates programmatically.

---

## PRESCRIPTION: REQUIRED REFACTORS

### Step 1: Extract `StepIdx` Newtype (New File)

```rust
// crates/vb_ipc/src/server/handlers/graph/step_idx.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StepIdx(u16);

impl StepIdx {
    pub fn new(idx: u16) -> Self { Self(idx) }
    pub fn get(self) -> u16 { self.0 }
    pub fn as_usize(self) -> usize { self.0 as usize }
}
```

### Step 2: Extract `WorkflowGraphService` (New File)

```rust
// crates/vb_ipc/src/server/handlers/graph/workflow_graph.rs
pub struct WorkflowGraphService;
impl WorkflowGraphService {
    pub fn build_graph(workflow: &dyn WorkflowHandle) -> WorkflowGraph { ... }
    pub fn collect_edges(step: StepIdx, kind: &CompiledNodeKind) -> Vec<Edge> { ... }
}
```

### Step 3: Extract `TaintAnalyzer` (New File)

```rust
// crates/vb_ipc/src/server/handlers/analysis/taint_analyzer.rs
pub struct TaintAnalyzer;
impl TaintAnalyzer {
    pub fn analyze(workflow: &Workflow) -> TaintReport { ... }
    fn bfs_forward(&self, start: StepIdx) -> Vec<StepIdx> { ... }
}
```

### Step 4: Extract `VerificationService` (New File)

```rust
// crates/vb_ipc/src/server/handlers/verification/verification_service.rs
pub struct VerificationService;
impl VerificationService {
    pub fn verify(workflow: &Workflow) -> VerificationResult { ... }
}
```

### Step 5: Extract `ValidationSanitizer` (New File)

```rust
// crates/vb_ipc/src/server/handlers/validation/sanitizer.rs
pub struct PathSanitizer { ... }
impl PathSanitizer {
    pub fn sanitize(&self, path: &str) -> String { ... }
}
```

### Step 6: Keep Only Request/Response Glue in `event.rs`

```rust
// event.rs should be ~50 lines
pub fn handle_verify_workflow(payload: &[u8], resolver: ...) -> IpcResponse {
    let IpcPayload::VerifyWorkflow { digest } = decode_payload(payload)? else {
        return IpcResponse::BadRequest;
    };
    VerificationService::new().verify(resolver.resolve_workflow(digest)?)
}
```

---

## SUMMARY

| Violation | Severity | Lines Affected |
|-----------|----------|----------------|
| Line count | CRITICAL | 648/300 (+116%) |
| Primitive obsession: `u16` | CRITICAL | 175, 186, 224, 300, 322, 401, 448, 461, 518, 543, 598, 602 |
| Primitive obsession: `u32` | HIGH | 94, 102, 103, 128, 129 |
| Primitive obsession: `usize` | HIGH | 13, 14, 15, 519, 636, 643 |
| Responsibility mingling | CRITICAL | ALL |
| Duplicated successor logic | HIGH | 174-349, 543-626 |
| Inline sanitization | MEDIUM | 17-33 |
| Scattered BFS | MEDIUM | 516, 628, 543 |

**Verdict**: `STATUS: GUILTY - MUST REFACTOR`

---

## RECOMMENDED TARGET STRUCTURE

```
handlers/
  mod.rs
  event.rs              # ~50 lines: request/response glue only
  graph/
    mod.rs
    step_idx.rs         # StepIdx newtype
    workflow_graph.rs   # WorkflowGraphService
    successors.rs       # all_successors + enqueue_successors + collect_edges
  analysis/
    mod.rs
    taint_analyzer.rs   # TaintAnalyzer + bfs_forward
  verification/
    mod.rs
    verification_service.rs  # VerificationService + gate registry
    sanitizer.rs        # PathSanitizer
```

**Expected final line count**: ~300 total across 6 files (5 new + 1 refactored event.rs).
