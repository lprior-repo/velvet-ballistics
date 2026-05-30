# Architectural Drift Report: vb_validate/src/gates.rs

## Line Count Violation

| Metric | Value |
|--------|-------|
| **Current lines** | 2894 |
| **Limit** | 300 |
| **Ratio** | 9.6x OVER |
| **Overflow** | 2594 lines |

---

## Gate Functions Requiring Extraction

### Public Gate API (9 gates)

| Gate | Function | Lines | Responsibility |
|------|-----------|-------|----------------|
| 7 | `validate_gate_07_expression_stack_depth` | 36-61 | Expression stack depth bounded |
| 8 | `validate_gate_08_accessor_path_segments` | 150-178 | Accessor path segment validity |
| 9 | `validate_gate_09_slot_references` | 233-242 | Slot references within bounds |
| 10 | `validate_gate_10_node_kind_specific` | 1120-1363 | Node-kind-specific constraints |
| 11 | `validate_gate_11_loop_body_graph` | 416-505 | Loop body graph well-formed |
| 12 | `validate_gate_12_action_contract_completeness` | 1432-1483 | Action contract bijection |
| 13 | `validate_gate_13_no_slot_cycles` | 869-883 | Slot dependency cycle detection |
| 14 | `validate_gate_14_slot_type_consistency` | 1586-1628 | Slot type consistency |
| 15 | `validate_gate_15_determinism_proof` | 1665-1699 | Determinism proof |

### Private Helpers (per-gate grouping)

| Gate | Helpers | Lines |
|------|---------|-------|
| 7 | `compute_stack_depth`, `pop_count`, `push_count`, `stack_effect` | 70-138 |
| 8 | `validate_field_symbol`, `validate_index_segment`, `validate_accessor_root` | 180-222 |
| 9 | `validate_node_slots`, `validate_expr_slots`, `check_slot` | 244-403 |
| 10 | `validate_expression_references`, `validate_load_const_reference`, `validate_load_accessor_reference` | 1365-1419 |
| 11 | `validate_loop_pairings`, `validate_node_pairing`, `require_matching_body_start`, `require_matching_done_start`, `require_matching_repeat_check`, `require_matching_together_branch`, `require_matching_together_join`, `has_prior_matching_start`, `step_in_loop_body`, `require_pairing`, `is_matching_for_each_start`, `is_matching_collect_start`, `is_matching_reduce_start`, `is_matching_repeat_start`, `is_foreach_start_done`, `is_collect_start_done`, `is_reduce_start_done`, `is_repeat_start_done`, `check_step_in_range`, `check_next_step_in_range`, `check_loop_span`, `check_together_span` | 507-854 |
| 13 | `build_slot_adjacency`, `append_node_edges`, `add_unique_edge`, `detect_cycle_dfs`, `node_reads` | 886-1101 |
| 12 | `validate_action_contract_capability_schema`, `validate_required_capability`, `validate_capability_name`, `is_capability_name_grammar_valid`, `validate_no_duplicate_capability_requirements` | 1485-1573 |
| 14 | `const_value_discriminant` | 1631-1642 |
| 15 | `is_non_deterministic` | 1703-1708 |

### Tests

| Section | Lines | Count |
|---------|-------|-------|
| Test helpers (`make_parts`, `nop_node`, `finish_node`, `copy_node`) | 1720-1772 | 4 |
| Gate 7 tests | 1774-1833 | 5 |
| Gate 8 tests | 1835-1878 | 4 |
| Gate 9 tests | 1880-1932 | 6 |
| Gate 11 tests | 1934-2131 | 8 |
| Gate 13 tests | 2133-2329 | 9 |
| Compute stack depth tests | 2331-2354 | 3 |
| Gate 7 adversarial tests | 2486-2508 | 3 |
| Gate 8 adversarial tests | 2510-2546 | 3 |
| Gate 9 adversarial tests | 2548-2598 | 3 |
| Gate 11 adversarial tests | 2600-2653 | 3 |
| BLACKHAT security tests | 2656-2893 | 8 |
| **Total tests** | **1714-2894** | **~1180 lines** |

---

## DDD Violations

### 1. **GATE BHAVIOR IS CROSS-CUTTING — VIOLATES COMPOSITION**

Gates are validation predicates over `WorkflowParts`. Per Scott Wlaschin's DDD, cross-cutting validations should compose via a **validation pipeline** or **specification pattern**, NOT be lumped into one god file.

**Current:**
```
gates.rs (2894 lines)
  ├── Gate 7  (expression stack)
  ├── Gate 8  (accessor paths)
  ├── Gate 9  (slot refs)
  ├── Gate 10 (node-kind specific)
  ├── Gate 11 (loop bodies)
  ├── Gate 12 (action contracts)
  ├── Gate 13 (slot cycles)
  ├── Gate 14 (slot types)
  ├── Gate 15 (determinism)
  └── ALL TESTS
```

**Correct DDD structure per phase:**
```
vb_validate/src/gates/
  ├── mod.rs           (re-exports, pipeline composer)
  ├── stack_depth.rs   (Gate 7 + helpers, ~60 lines)
  ├── accessor_paths.rs (Gate 8 + helpers, ~80 lines)
  ├── slot_refs.rs     (Gate 9 + helpers, ~100 lines)
  ├── node_kinds.rs    (Gate 10 + helpers, ~200 lines)
  ├── loop_graph.rs    (Gate 11 + helpers, ~250 lines)
  ├── action_contracts.rs (Gate 12 + helpers, ~100 lines)
  ├── slot_cycles.rs   (Gate 13 + helpers, ~150 lines)
  ├── slot_types.rs    (Gate 14 + helpers, ~50 lines)
  └── determinism.rs   (Gate 15 + helpers, ~50 lines)
```

### 2. **GATE 10 MATCHES ALL `CompiledNodeKind` VARIANTS — ANTI-PATTERN**

Lines 1120-1363: `validate_gate_10_node_kind_specific` has a single 243-line function with 50+ match arms over `CompiledNodeKind`. This is:

- **Not cohesive**: One function validates every node variant
- **Not extensible**: Adding a new `CompiledNodeKind` variant requires modifying this monolith
- **Not testable in isolation**: Cannot run gate 10 for only `Choose` nodes

Per DDD, each node kind should have its own **specification** or **validator**.

### 3. **`node_reads` duplications `validate_node_slots` — DUPLICATED DOMAIN LOGIC**

Lines 967-1101 (`node_reads`) and lines 244-372 (`validate_node_slots`) both pattern-match over `CompiledNodeKind` with nearly identical structure. If `CompiledNodeKind` changes, BOTH must change. This is the **Shotgun Surgery** smell.

### 4. **NO VALUE OBJECTS FOR VALIDATION CONTEXT**

Functions pass loose `(acc_index, seg_index, symbol, symbols_count)` tuples. These should be wrapped in a typed context:

```rust
// CURRENT (primitive obsession)
fn validate_field_symbol(acc_index: usize, seg_index: usize, symbol: SymbolId, symbols_count: u32) -> ValidationResult<()>;

// BETTER (typed context)
struct AccessorSegmentContext {
    accessor_index: AccessorIndex,
    segment_index: SegmentIndex,
    symbol: SymbolId,
    symbols_count: SymbolCount,
}
```

### 5. **LOOP PAIRING PREDICATES ARE UNTESTED IN ISOLATION**

Lines 689-759 define 8 predicate functions (`is_matching_for_each_start`, `is_foreach_start_done`, etc.) that are tested only indirectly through gate 11 integration tests. Each should have its own unit test.

---

## Primitive Obsession Violations

### 1. **Raw `usize` Indices Everywhere**

| Location | Type | Should Be |
|----------|------|-----------|
| `acc_index` (lines 151, 152, 163, etc.) | `usize` | `AccessorIndex` (newtype) |
| `seg_index` (lines 160, 183, 198) | `usize` | `SegmentIndex` (newtype) |
| `expr_index` (lines 44, 379, 1374) | `usize` | `ExpressionIndex` (newtype) |
| `node_index` (lines 235, 250, 364) | `usize` | `NodeIndex` (newtype) |
| `slot` (lines 395, 396, 398) | `usize` | `SlotIndex` (already exists: `SlotIdx`) |

**Evidence**: `SlotIdx` and `StepIdx` ARE proper newtypes (`AccessorIdx`, `ActionId`, `ConstIdx`, `ExprIdx`, `SlotIdx`, `StepIdx`, `SymbolId` all exist). But the validation code uses raw `usize` for context indices like `accessor_index`, `segment_index`, `expr_index`, `node_index`.

### 2. **Raw `Vec<Vec<usize>>` for Adjacency**

Lines 886-892: `build_slot_adjacency` returns `Vec<Vec<usize>>` with no domain type.

```rust
// CURRENT
fn build_slot_adjacency(parts: &WorkflowParts, slot_count: usize) -> Vec<Vec<usize>>

// SHOULD BE
type SlotAdjacency = Vec<Vec<SlotIndex>>;
fn build_slot_adjacency(parts: &WorkflowParts) -> SlotAdjacency
```

### 3. **Magic Color Numbers for DFS**

Lines 876-962: `detect_cycle_dfs` uses raw `u8` values 0/1/2 for white/gray/black.

```rust
// CURRENT
let mut visited: Vec<u8> = vec![0; slot_count]; // 0 = white, 1 = gray, 2 = black

// SHOULD BE
enum class VisitState { White, Gray, Black }
```

### 4. **`String` for Error Context**

Lines 384-385, 946-947, etc.:
```rust
context: format!("expression {expr_index}"),
chain: format!("slot {slot} -> slot {neighbor}"),
```
Should use a dedicated error context type, not `String` interpolation.

### 5. **Raw `u8` Discriminant**

Lines 1593-1594: `slot_const_kind: Vec<u8>` uses raw `u8` for type discriminants.
```rust
// CURRENT
let mut slot_const_kind: Vec<u8> = vec![0; slot_count]; // 0 = unset, 1 = Null, 2 = Bool...
```

Should be:
```rust
enum class ConstKind { Unset, Null, Bool, I64, F64, Symbol }
```

---

## Recommended Split by Validation Phase

### Phase 1: Expression & Accessor Gates (gates_stack_accessor.rs, ~150 lines)

**Gate 7**: Expression stack depth
- `validate_gate_07_expression_stack_depth`
- `compute_stack_depth`, `pop_count`, `push_count`, `stack_effect`

**Gate 8**: Accessor path segments
- `validate_gate_08_accessor_path_segments`
- `validate_field_symbol`, `validate_index_segment`, `validate_accessor_root`

### Phase 2: Slot Reference Gates (gates_slot_refs.rs, ~170 lines)

**Gate 9**: Slot references within bounds
- `validate_gate_09_slot_references`
- `validate_node_slots`, `validate_expr_slots`, `check_slot`

### Phase 3: Graph Structure Gates (gates_graph.rs, ~400 lines)

**Gate 11**: Loop body graph
- `validate_gate_11_loop_body_graph`
- All `require_matching_*`, `check_*`, `is_matching_*` helpers
- `validate_loop_pairings`, `validate_node_pairing`

**Gate 13**: Slot dependency cycles
- `validate_gate_13_no_slot_cycles`
- `build_slot_adjacency`, `append_node_edges`, `add_unique_edge`
- `detect_cycle_dfs`, `node_reads`

### Phase 4: Node Kind Gates (gates_node_kinds.rs, ~300 lines)

**Gate 10**: Node-kind-specific constraints
- `validate_gate_10_node_kind_specific`
- `validate_expression_references`, `validate_load_const_reference`, `validate_load_accessor_reference`

**Note**: Gate 10's 243-line match should be refactored into per-variant validators:
```rust
trait NodeKindValidator {
    fn validate(&self, node: &CompiledNode, ctx: &ValidationContext) -> ValidationResult<()>;
}
impl NodeKindValidator for FinishValidator { ... }
impl NodeKindValidator for ChooseValidator { ... }
// etc.
```

### Phase 5: Contract & Type Gates (gates_contracts.rs, ~200 lines)

**Gate 12**: Action contract completeness
- `validate_gate_12_action_contract_completeness`
- `validate_action_contract_capability_schema`, `validate_required_capability`
- `validate_capability_name`, `is_capability_name_grammar_valid`
- `validate_no_duplicate_capability_requirements`

**Gate 14**: Slot type consistency
- `validate_gate_14_slot_type_consistency`
- `const_value_discriminant`

### Phase 6: Determinism Gate (gates_determinism.rs, ~60 lines)

**Gate 15**: Determinism proof
- `validate_gate_15_determinism_proof`
- `is_non_deterministic`

### Phase 7: Test Extraction (gates/tests/, ~1180 lines → separate file)

Each gate module gets its own `#[cfg(test)]` sub-module OR tests move to `workspace_tests/`.

---

## Primitive Obsession: Summary Table

| # | Pattern | Location | Newtype Needed |
|---|---------|----------|---------------|
| 1 | `usize` for context indices | Throughout | `AccessorIndex`, `SegmentIndex`, `ExpressionIndex`, `NodeIndex` |
| 2 | `Vec<Vec<usize>>` adjacency | line 886 | `SlotAdjacency` |
| 3 | `u8` for color state | line 876 | `VisitState` enum |
| 4 | `String` for error context | lines 384, 946 | `ValidationContext` struct |
| 5 | `u8` for const kind | line 1593 | `ConstKind` enum |

---

## Refactoring Order (Safe Sequence)

1. **Create `gates/` directory** with `mod.rs` that re-exports all gate functions
2. **Extract Gate 7** into `gates/stack_depth.rs` — smallest, no dependencies on other gates
3. **Extract Gate 8** into `gates/accessor_paths.rs`
4. **Extract Gate 15** into `gates/determinism.rs` — depends only on `CompiledNodeKind`
5. **Extract Gate 14** into `gates/slot_types.rs`
6. **Extract Gate 9** into `gates/slot_refs.rs`
7. **Extract Gate 12** into `gates/action_contracts.rs`
8. **Extract Gate 11** into `gates/loop_graph.rs` (large, careful with pairings)
9. **Extract Gate 13** into `gates/slot_cycles.rs` (node_reads must be extracted cleanly)
10. **Extract Gate 10** into `gates/node_kinds.rs` (consider per-variant validators)
11. **Move tests** into `#[cfg(test)]` sub-modules per gate OR to `workspace_tests/`
12. **Replace primitive `usize` indices** with newtype wrappers (can be done incrementally)

---

## Files After Refactoring

```
vb_validate/src/gates/
├── mod.rs              (~50 lines: re-exports + pipeline)
├── stack_depth.rs      (~140 lines)
├── accessor_paths.rs   (~100 lines)
├── slot_refs.rs        (~170 lines)
├── loop_graph.rs       (~400 lines)
├── slot_cycles.rs      (~250 lines)
├── node_kinds.rs       (~300 lines, refactor match arms)
├── action_contracts.rs (~200 lines)
├── slot_types.rs       (~60 lines)
├── determinism.rs      (~60 lines)
└── tests/              (~1180 lines in #[cfg(test)] submodules)
```

**Estimated total**: ~1800 lines (still over 300 per file limit — each gate file may still exceed limit)

**Further action required**: Some gate files still exceed 300 lines. Gate 10 (~243 main fn + helpers), Gate 11 (~350 helpers), Gate 13 (~250 helpers) need sub-splitting into per-concern helpers.

---

## DDD Principles Violated Summary

| Principle | Violation | Severity |
|-----------|-----------|----------|
| **One responsibility per file** | gates.rs holds 9 gate responsibilities | CRITICAL |
| **Composition over monolithic validation** | Single `validate_gate_X` functions do everything | HIGH |
| **Newtypes for indices** | Raw `usize` for context indices despite `SlotIdx`/`StepIdx` existing | MEDIUM |
| **Shotgun surgery** | `node_reads` and `validate_node_slots` duplicate same match structure | MEDIUM |
| **Untestable in isolation** | Pairing predicates only tested via gate 11 integration | MEDIUM |
| **Primitive obsession** | `String`, `u8`, `Vec<Vec<usize>>` used where domain types exist | MEDIUM |
| **Non-exhaustive enum handling** | Gate 10 `_ => {}` silently ignores unknown variants | HIGH |

---

## STATUS: CRITICAL DRIFT — MANDATORY REFACTOR

**gates.rs at 2894 lines is 9.6x the 300-line limit. This is not a style issue — it is an architectural failure that prevents comprehension, testing, and maintainability.**
