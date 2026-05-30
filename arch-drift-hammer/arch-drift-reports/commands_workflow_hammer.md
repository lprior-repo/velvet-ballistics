# ARCHITECTURAL DRIFT REPORT: commands_workflow.rs

**File**: `crates/vb_cli/src/commands_workflow.rs`  
**Line Count**: 504 lines  
**Violation Status**: RED - EXCEEDS 300-LINE LIMIT BY 204 LINES

---

## EXECUTIVE SUMMARY

This file is a **PRIMITIVE OBSESSION HELLSCAPE** fused with **CONCENTRATED CONCERN BLINDNESS**. It should be split into at minimum 3 modules:
1. DOT graph generation
2. Workflow simulation  
3. Shared types/helpers

---

## VIOLATION 1: LINE COUNT (504 > 300)

**Severity**: CRITICAL

The file is 504 lines. The architectural contract mandates `<300` lines per file.

**Required Split**:
```
commands_workflow/          (new module directory)
├── mod.rs                  (~30 lines - re-exports)
├── dot_graph.rs            (~170 lines)
├── simulation.rs           (~180 lines)  
└── shared.rs               (~80 lines - helpers, SaturatingUsize, labels)
```

---

## VIOLATION 2: PRIMITIVE OBSESSION - TYPE CHAOS

### 2.1 Raw `usize` Everywhere

| Location | Primitive | Should Be |
|----------|-----------|-----------|
| `DotGraph.node_count` | `usize` | `NodeCount(usize)` |
| `DotGraph.edge_count` | `usize` | `EdgeCount(usize)` |
| `SimulationStep.index` | `usize` | `StepIndex(usize)` |
| `SimulationResult.total_steps` | `usize` | `StepCount(usize)` |
| `SimulationResult.action_count` | `usize` | `ActionCount(usize)` |
| `SimulationResult.branch_count` | `usize` | `BranchCount(usize)` |
| `saturating_add(a: usize, b: usize)` | raw | `SaturatingUsize::add(usize)` |

### 2.2 Raw `u16` Escaping

```rust
// Line 47-52: Raw u16 used as index
let next = node.next;
dot_lines.push(format!("    node_{i} -> node_{};", next.get()));
```

`StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX))` is repeated **4 times** in this file. This is a `Parse, don't validate` failure — the `unwrap_or(u16::MAX)` silently accepts invalid indices instead of failing fast.

### 2.3 Raw Tuples as Data Structures

```rust
// Line 240-241: RAW TUPLE INSTEAD OF TYPE
fn collect_kind_edges(node_idx: u16, kind: &CompiledNodeKind) -> Vec<(u16, u16, String)> {
```

This should be:
```rust
struct WorkflowEdge {
    from: StepIdx,
    to: StepIdx,
    label: EdgeLabel,
}
```

### 2.4 String Labels Everywhere

```rust
// Line 14: Raw String
pub dot: String,

// Line 29, 39, 61: Constant string formatting
format!("{i}: {name}")
label.replace('"', "\\\"")
```

No `DotString`, no `EscapedLabel`, no `NodeLabel` newtypes.

---

## VIOLATION 3: DDD BICYCLE PRINCIPLE

### 3.1 `node_kind_label()` - THE STRING DISPENSER

This function (lines 131-169) is a **pure string transformation** with no semantic meaning. It's a 38-line match arm that maps enum variants to `&'static str` labels.

**Scott Wlaschin says**: "Make illegal states unrepresentable."

Currently you can have ANY string as a label. You should have:
```rust
struct NodeKindLabel(&'static str);
impl NodeKindLabel {
    fn from_kind(kind: &CompiledNodeKind) -> Self { ... }
}
```

### 3.2 `describe_node_for_simulate()` - STRING BUILDING DISASTER

Lines 171-238: 67 lines of string formatting with mutation hidden in the call site.

```rust
fn describe_node_for_simulate(
    kind: &CompiledNodeKind,
    action_count: &mut usize,  // <-- MUTATION HIDDEN IN CALLER
    branch_count: &mut usize,
) -> String
```

This function has **side effects hidden in the return value** — the `action_count` and `branch_count` are mutated but not reflected in the return. This is a violation of the "explicit state transitions" principle.

**Better design**:
```rust
struct NodeDescription {
    label: String,
    stats: SimulationStats,
}
struct SimulationStats {
    actions_added: usize,
    branches_added: usize,
}
fn describe_node(kind: &CompiledNodeKind) -> NodeDescription { ... }
```

### 3.3 No Value Objects for Edge Labels

`"body"`, `"done"`, `"branch"`, `"join"`, `"otherwise"`, `"entry"` are all raw `&'static str` strings scattered throughout `collect_kind_edges`. These should be an enum:
```rust
enum EdgeRole {
    Body,
    Done,
    Branch,
    Join,
    Otherwise,
    Entry,
    Fallback,
}
```

---

## VIOLATION 4: CONCENTRATED CONCERN BLINDNESS

The file mixes **three distinct responsibilities** with no boundaries:

1. **DOT Graph Generation** (`generate_dot`, `DotGraph`)
2. **Workflow Simulation** (`simulate_workflow`, `SimulationStep`, `SimulationResult`)
3. **Shared Helpers** (`node_kind_label`, `describe_node_for_simulate`, `collect_kind_edges`, `saturating_add`)

These should be separate modules with clear boundaries.

---

## VIOLATION 5: `unwrap_or(u16::MAX)` SILENT FAILURE

Lines 27, 45, 56, 102:

```rust
let step = StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX));
```

This silently converts valid `usize` values > `u16::MAX` to `u16::MAX`. This is **data corruption**, not error handling. If `node_count > 65535`, the code will generate incorrect DOT output with silent corruption.

**Correct approach**:
```rust
let step = StepIdx::new(u16::try_from(i).ok().unwrap_or_else(|| {
    tracing::warn!("Step index {i} exceeds u16::MAX, truncating");
    u16::MAX
}));
```
Or simply: `u16::try_from(i).expect("workflow step count exceeds u16::MAX")`

---

## VIOLATION 6: TEST PRIMITIVE OBSESSION

The tests (lines 335-503) use raw values:
```rust
assert_eq!(saturating_add(3, 5), 8);  // Raw 3, 5, 8
assert_eq!(saturating_add(usize::MAX, 1), usize::MAX);
```

Tests should use named constants or newtypes to make the intent clear.

---

## SUMMARY TABLE

| Violation | Severity | Lines Affected |
|-----------|----------|-----------------|
| Line count 504 > 300 | CRITICAL | Entire file |
| Primitive obsession: usize fields | HIGH | Lines 12-15, 82-93 |
| Primitive obsession: u16 indices | HIGH | Lines 27, 45, 56, 102 |
| Raw tuple return type | HIGH | Line 240 |
| String labels without types | MEDIUM | Lines 131-169 |
| Side-effecting description | MEDIUM | Lines 171-238 |
| No edge label enum | MEDIUM | Lines 240-327 |
| unwrap_or(u16::MAX) silent corruption | CRITICAL | Lines 27, 45, 56, 102 |

---

## MANDATORY REFACTORING

1. **Split into 3 modules** under `commands_workflow/` directory
2. **Create newtypes**: `NodeCount`, `EdgeCount`, `StepIndex`, `EdgeLabel`, `EdgeRole`
3. **Replace `unwrap_or(u16::MAX)`** with proper error handling
4. **Extract `describe_node_for_simulate`** side effects into a proper return struct
5. **Add tests using named types**, not raw primitives

**STATUS**: REQUIRES REFACTORING
