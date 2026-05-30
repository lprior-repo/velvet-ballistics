# Architectural Drift Report: `vb_ipc/src/server/handlers.rs`

**File**: `crates/vb_ipc/src/server/handlers.rs`
**Analyzed**: 2026-05-29
**Enforcement Rule**: `<300 lines`, DDD cohesion, single-responsibility

---

## 1. LINE COUNT

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **3998** | 300 | ❌ FAIL (13.3× over budget) |
| Production code | ~1133 | 300 | ❌ FAIL (3.8× over budget) |
| Inline tests | ~2865 | — | ❌ 71.7% of file |

---

## 2. DDD COHESION ANALYSIS

**Filename reflects**: `handlers` — generic, does NOT bound a single domain concept.

### Domain Concepts Mixed in One File

| Concern | Lines | Domain |
|---------|-------|--------|
| IPC command dispatch | 1–155, 158–428 | **IPC Protocol** |
| Workflow graph construction | 580–879, 1019–1132 | **Workflow Graph Analysis** |
| Taint tracking BFS | 888–983, 987–1132 | **Taint Analysis** |
| Validation gate execution | 477–577 | **Workflow Validation** |
| Security sanitization | 68–104 | **Security** |
| Metrics assembly | 430–474 | **Observability** |
| Inline tests | 1134–3998 | **Tests** |

**Verdict**: ❌ **DDD SMELL DETECTED** — 7 distinct domain concepts crammed into one file.

---

## 3. VIOLATIONS

### 3.1 Critical: Oversized Functions (>30 lines)

| Function | Lines | LOC | Violation |
|----------|-------|-----|-----------|
| `collect_edges_from_node` | 623–803 | **180** | Massive match on all `CompiledNodeKind` variants |
| `handle_verify_workflow` | 477–577 | **100** | Gate execution loop, too many responsibilities |
| `handle_get_taint_report` | 888–983 | **95** | BFS + source/sink collection + response building |
| `all_successors` | 1019–1107 | **88** | Duplicates `collect_edges_from_node` logic |
| `handle_get_workflow_graph` | 807–879 | **72** | Node iteration + edge collection + response building |
| `handle_get_metrics` | 430–474 | **44** | Metrics snapshot → shard mapping → aggregate |
| `node_kind_label` | 580–620 | **40** | Pattern match on all node kinds |
| `submit_resolved_workflow` | 363–403 | **40** | Size check, resolver, workflow dispatch |
| `enqueue_successors` | 1110–1132 | **22** | BFS successor enqueue |
| `bfs_forward` | 987–1013 | **26** | BFS traversal core |

### 3.2 Inline Tests (Lines 1134–3998)

| Test Group | Lines | Count |
|------------|-------|-------|
| decode_payload roundtrips | 1143–1378 | ~20 tests |
| handler unit tests | 1382–1556 | ~15 tests |
| all_successors regression | 1467–1556 | ~6 tests |
| Security regression round 5–6 | 1558–2134 | ~30 tests |
| Runtime integration tests | 2151–3730 | ~80 tests |
| Mutation kill boundary tests | 3745–3998 | ~20 tests |
| **Total test lines** | **~2865** | **~160+ tests** |

**Violation**: Tests are **71.7%** of the file. Per architectural rules, tests belong in `tests/` or behind feature flags, not inline.

### 3.3 Constants Clutter (Lines 36–66)

```rust
const MAX_RUNTIME_ERROR_LEN: usize = 256;
const MAX_SUBMIT_INPUT_LEN: usize = 65536;
const MAX_ACTION_OUTPUT_LEN: usize = 65536;
const MAX_ACTION_ERROR_LEN: usize = 65536;
const MAX_TAINT_PATH_ENTRIES: usize = 65536;
const MAX_VALIDATION_DETAIL_LEN: usize = 512;
const MAX_LIST_RUNS_LIMIT: u32 = 4096;
const MAX_ANSWER_ASK_BYTES: usize = 65536;
const MAX_WORKFLOW_GRAPH_NODES: usize = 8192;
```

11 constants in module scope pollute the namespace. Should be grouped into a `limits` or `constants` submodule.

### 3.4 Missing Module Separation

| Concept | Location | Should Be |
|---------|----------|-----------|
| Graph analysis (`node_kind_label`, `collect_edges_from_node`, `all_successors`, `enqueue_successors`, `bfs_forward`) | Inline in handlers.rs | `server/graph.rs` or `graph/mod.rs` |
| Taint analysis | Inline in handlers.rs | `server/taint.rs` |
| Validation gates | Inline in handlers.rs | `server/validation.rs` |
| Security sanitization | Inline in handlers.rs | `server/sanitize.rs` |
| All inline tests | Lines 1134–3998 | `tests/handlers_tests.rs` or `workspace_tests/vb_ipc_handlers.rs` |

---

## 4. SPECIFIC LINE COUNTS

| Section | Lines | Content |
|---------|-------|---------|
| 1–21 | 21 | Module doc, `#![forbid]`, imports |
| 22–66 | 45 | Constants (11 total) |
| 68–104 | 36 | Security sanitization fns |
| 107–427 | 320 | Handler functions (14 handlers) |
| 430–577 | 147 | Metrics + verify-workflow |
| 580–1132 | 552 | Graph analysis utilities |
| 1134–3998 | 2865 | **Inline tests** |

---

## 5. REMEDIATION PRIORITY

| Priority | Action | Effort |
|----------|--------|--------|
| **P0** | Extract inline tests to `tests/handlers.rs` or `workspace_tests/vb_ipc_handlers.rs` | High |
| **P0** | Split production code into modules: `graph.rs`, `validation.rs`, `taint.rs`, `sanitize.rs` | High |
| **P1** | Move 11 constants into `limits.rs` or group by handler domain | Medium |
| **P1** | Reduce `collect_edges_from_node` (180 lines) — extract pattern arms to helper fns | Medium |
| **P2** | Extract `all_successors` reuse — consolidate with `collect_edges_from_node` | Medium |
| **P2** | Reduce `handle_verify_workflow` — extract gate_names array to module constant | Low |

---

## 6. SUMMARY

```
Lines:         3998 (limit: 300)      ❌ FAIL
DDD Cohesion:  NO - 7 concepts mixed  ❌ FAIL
Oversized Fn:  10 functions >30 lines  ❌ FAIL
Inline Tests:  2865 lines (71.7%)     ❌ FAIL
Module Sep:    NO - all in one file   ❌ FAIL
Constants:     11 at module scope     ⚠️  WARN
```

**Overall**: ❌ **SEVERE ARCHITECTURAL DRIFT** — File is 13.3× over budget and violates every DDD cohesion rule.

---

## 7. RECOMMENDED FILE STRUCTURE

```
vb_ipc/src/server/
├── mod.rs          (reexports)
├── handlers.rs     (~300 lines: dispatch + error helpers only)
├── graph.rs        (~250 lines: node_kind_label, collect_edges_from_node, all_successors, enqueue_successors, bfs_forward)
├── validation.rs   (~150 lines: handle_verify_workflow)
├── taint.rs        (~150 lines: handle_get_taint_report)
├── sanitize.rs     (~50 lines: sanitize_runtime_error, sanitize_validation_detail)
├── limits.rs       (~50 lines: all MAX_* constants)
└── tests/
    └── handlers_tests.rs  (moved inline tests)
```
