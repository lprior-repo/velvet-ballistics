# Architectural Drift Report: `vb_core/src/workflow/mod.rs`

## File Overview
- **Path**: `crates/vb_core/src/workflow/mod.rs`
- **Total Lines**: 1909
- **Line Limit**: 300
- **Violation**: YES — **604% over limit**

---

## 1. Line Count Violations

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 1909 | 300 | **VIOLATION (+1609)** |
| Validation Logic | ~1150 | — | Embedded in single file |
| Type Definitions | ~400 | — | All in one file |
| Inline `mod tests` | ~5 | — | Line 1909 |

---

## 2. DDD Cohesion Analysis

### Filename vs. Content
- **Filename**: `workflow/mod.rs`
- **Expected**: Single domain concept for workflow IR
- **Actual**: Multiple domain concepts co-located

### Detected Domain Concepts (6 concepts in 1 file)
1. **Compiled Workflow IR** — `CompiledWorkflow`, `WorkflowParts`
2. **Resource Bounds** — `ResourceContract`
3. **Expression Bytecode** — `ExprProgram`, `ExprOp`
4. **Accessor Paths** — `AccessorProgram`, `PathSegment`
5. **Node State Machine** — `CompiledNode`, `CompiledNodeKind`
6. **Lifecycle State Machine** — `LifecycleState`, `LifecycleCommand`, `RunState`

### DDD Smell: **YES** — God module anti-pattern
The file violates Single Responsibility Principle. It should be split into:
```
workflow/
├── mod.rs          # Re-exports only (~50 lines)
├── compiled.rs     # CompiledWorkflow, WorkflowParts, ResourceContract
├── nodes.rs        # CompiledNode, CompiledNodeKind
├── expression.rs   # ExprProgram, ExprOp, StackEffect
├── accessor.rs     # AccessorProgram, PathSegment
├── lifecycle.rs    # LifecycleState, LifecycleCommand, RunState
├── validation.rs   # All validation functions (currently ~1150 lines)
└── tests.rs        # Test module (currently inline)
```

---

## 3. All Violations

### 3.1 Line Count Violation
- **Lines 1–1909**: 1909 total lines vs. 300 maximum
- **Severity**: CRITICAL

### 3.2 Oversized Functions
| Function | Lines | Issue |
|----------|-------|-------|
| `validate_parts` | 734–752 | Entry point; 18 validation helpers called inline |
| `validate_node_kind` | 936–1065 | 130-line match with 40+ arms — should dispatch to per-variant validators |
| `validate_reachability` | 1378–1447 | 70-line BFS algorithm embedded in workflow module |
| `validate_forward_edges` | 1554–1577 | Forward edge + loop span validation |
| `validate_kind_edges` | 1580–1641 | 62-line match mirroring `validate_node_kind` |

### 3.3 Missing Module Separation
**Current structure**:
```
workflow/
└── mod.rs  (1909 lines — EVERYTHING)
```

**Should be**:
```
workflow/
├── mod.rs           # Re-exports only
├── compiled.rs      # IR types (~200 lines)
├── validation.rs    # All validation logic (~1100 lines)
├── lifecycle.rs     # Lifecycle state machine (~80 lines)
└── tests.rs         # Tests (behind #[cfg(test)])
```

### 3.4 Inline Test Module
- **Line 1909**: `mod tests;`
- **Issue**: Tests should be `tests/` directory or `#[cfg(test)]` module with clear separation

### 3.5 Validation Logic Bloat
Lines 734–1902 (~1168 lines) are **pure validation** that should live in `validation.rs`:
- `validate_parts`, `validate_budget`, `validate_budget_result`, `budget_error_detail`
- `validate_node_id`, `validate_resource_contract`, `validate_transitions_per_tick`
- `validate_resource_counts`, `validate_primary_resource_counts`, `validate_expression_resource_counts`
- `validate_contract_limit`, `validate_expr_stack_contract`
- `validate_entry`, `validate_node`, `validate_node_common`, `validate_node_kind`
- `validate_optional_slot`, `validate_slots`, `validate_build_list`, `validate_build_object`
- `validate_for_each_start`, `validate_slot_and_steps`, `validate_two_steps`, `validate_together`
- `validate_nonzero_u16`, `validate_reduce_start`, `validate_reduce_next`, `validate_repeat_start`
- `validate_slot_choose`, `validate_expr_choose`, `validate_branch_route`
- `validate_optional_step`, `validate_expr`, `validate_step`, `validate_slot`, `validate_const`
- `validate_expressions`, `validate_expression_accessors`, `validate_accessors`, `validate_accessor`
- `validate_accessor_paths`, `validate_constants_symbols`, `validate_build_object_symbols`, `validate_symbol`
- `validate_reachability`, `collect_node_targets`
- `collect_choose_slot_targets`, `collect_choose_expr_targets`, `collect_together_start_targets`
- `validate_forward_edges`, `validate_kind_edges`
- `validate_choose_slot_edges`, `validate_choose_expr_edges`, `validate_loop_done_only`
- `validate_together_start_edges`, `validate_together_branch_edges`, `validate_forward_target`
- `push_loop_span`, `validate_expr_op_count`, `apply_expr_stack_effect`
- `validate_expr_stack_capacity`, `validate_expr_final_depth`, `expr_stack_effect`

---

## 4. Remediation Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0 (Critical)** | Split `mod.rs` — extract `validation.rs` (~1168 lines) | High |
| **P0 (Critical)** | Split `mod.rs` — extract `compiled.rs`, `nodes.rs`, `expression.rs`, `accessor.rs`, `lifecycle.rs` | High |
| **P1 (High)** | Replace `mod tests;` with `#[cfg(test)]` module or `tests/` directory | Medium |
| **P2 (Medium)** | Extract `validate_node_kind` match into per-variant validator functions | Medium |
| **P3 (Low)** | Move `StackEffect` and `expr_stack_effect` to `expression.rs` | Low |

---

## 5. Summary

| Metric | Result |
|--------|--------|
| Total Lines | **1909** (limit: 300) |
| DDD Smell Detected | **YES** |
| God Module | **YES** — 6 domain concepts in 1 file |
| Violations | 4 major categories |
| Remediation Priority | **P0 — Critical refactor required** |

---

## Recommendation

**Do not approve this file for production.** It must be split into:
1. `workflow/mod.rs` — thin re-export layer (~50 lines)
2. `workflow/validation.rs` — all validation logic
3. `workflow/compiled.rs` — `CompiledWorkflow`, `WorkflowParts`, `ResourceContract`
4. `workflow/nodes.rs` — `CompiledNode`, `CompiledNodeKind`
5. `workflow/expression.rs` — `ExprProgram`, `ExprOp`, `StackEffect`
6. `workflow/accessor.rs` — `AccessorProgram`, `PathSegment`
7. `workflow/lifecycle.rs` — `LifecycleState`, `LifecycleCommand`, `RunState`
8. `workflow/tests.rs` or `workflow/tests/` — test module

This is a **structural refactor** that requires go-skill gate before landing.
