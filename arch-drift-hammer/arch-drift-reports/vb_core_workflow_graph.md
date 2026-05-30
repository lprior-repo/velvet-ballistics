# Architectural Drift Report: `vb_core::validation::graph`

**File**: `crates/vb_core/src/validation/graph.rs`  
**Lines**: 254 (✓ under 300 limit)  
**DDD Cohesion**: HIGH - Single responsibility for graph structural validation  
**Priority**: MEDIUM

---

## 1. Line Count

| Metric | Value |
|--------|-------|
| Total lines | 254 |
| Limit | 300 |
| Status | ✓ PASS |

---

## 2. DDD Cohesion Analysis

**Cohesion Score**: HIGH

**Domain Role**: This module provides graph structural validation for compiled workflows:
- `validate_reachability()` - BFS reachability from entry node
- `validate_forward_edges()` - All edges point forward (except loop back-edges)
- `validate_kind_edges()` - Dispatches to kind-specific edge validators
- `push_loop_span()` - Loop span tracking for nesting validation

**Cohesion Assessment**: 
The module has a single, well-defined responsibility. All public functions operate on `WorkflowParts` and produce `WorkflowError`. No mixed concerns.

---

## 3. Violations

### V1: Unnecessary Re-export Module (DDD Smell)
**Severity**: MINOR  
**Lines**: 9-11

```rust
pub(crate) mod targets {
    pub(crate) use super::super::targets::collect_node_targets;
}
```

**Problem**: This pass-through re-export creates an artificial module boundary. The `targets` function is already accessible via `super::super::targets::collect_node_targets` or could be imported directly at the validation package level. The re-export adds indirection without value.

**Recommendation**: Import directly from `crate::validation::targets` or remove the re-export and use the full path.

---

### V2: Semantic Error Type Mismatch  
**Severity**: MEDIUM  
**Lines**: 26-28

```rust
let Some(entry_flag) = visited.get_mut(entry_usize) else {
    return Err(WorkflowError::EntryOutOfBounds { entry: parts.entry });
};
```

**Problem**: The error case `visited.get_mut(entry_usize)` returning `None` is **dead code**. On lines 22-25, we already check `entry_usize >= node_count` and return `Ok(())` in that case. Since `visited` has exactly `node_count` elements (line 19), `entry_usize` accessing `visited` can never be out of bounds at line 26.

Additionally, `WorkflowError::ResourceContractExceeded` (used in lines 71, 88-89, 232-242) is semantically incorrect for index conversion failures. That error variant is meant for "resource contract does not cover the compiled artifact" (per validation.rs line 69-74).

**Recommendation**: 
1. Remove the dead code path (lines 26-28) since the bounds check already handles this case
2. Introduce a new error variant like `StepIndexConversionError` for `u16::try_from` failures, or use a different existing variant

---

### V3: Overly Defensive Index Arithmetic
**Severity**: MINOR  
**Lines**: 38-41

```rust
head = match head.checked_add(1) {
    Some(v) => v,
    None => break,
};
```

**Problem**: `head` is initialized from `queue.get(head)` (line 34) which is guaranteed to succeed because `head < queue.len()` (line 33). Therefore `head.checked_add(1)` is unnecessary defensive programming that obscures intent.

**Recommendation**: Use simple `head += 1` with a comment explaining why it's safe, or assert `head + 1 <= queue.len()`.

---

### V4: Dead Code in BFS Loop
**Severity**: MINOR  
**Lines**: 44-47

```rust
let node = match parts.nodes.get(current) {
    Some(n) => n,
    None => break,
};
```

**Problem**: `current` is always a valid index because:
- `queue` only contains `usize` values from `entry_usize` or pushed from `target_usize` values (line 61)
- `target_usize < node_count` is checked before pushing (line 55)
- `current = queue[head]` where `head < queue.len()`

So `parts.nodes.get(current)` can never return `None`. This is dead code that makes the logic harder to reason about.

---

## 4. Summary

| Category | Count | Severity |
|----------|-------|----------|
| DDD violations | 1 | Minor |
| Error semantics | 2 | Medium |
| Dead code | 2 | Minor |
| Over-defensive | 1 | Minor |

**Overall Assessment**: The module is architecturally sound with high cohesion and correct algorithmic logic. The violations are primarily around error semantics (wrong error types) and unnecessary defensive code rather than fundamental architectural problems.

**Priority**: MEDIUM - Error semantics should be fixed to prevent incorrect error classification in production logs/traces.

---

## 5. Recommended Fixes

1. **V1**: Remove the `pub(crate) mod targets` re-export; use direct imports
2. **V2**: Add `StepIdxConversionError` variant or reuse `StepOutOfBounds`; remove dead code path
3. **V3**: Replace `checked_add` with simple increment + assert
4. **V4**: Remove unreachable `None` match arm with explanatory comment
