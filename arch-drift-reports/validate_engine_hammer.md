# Architectural Drift Report: `validate.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_core/src/engine/validate.rs`
**Lines:** 1164 (VIOLATION: 3.88x the 300-line limit)
**Date:** 2026-05-29
**Severity:** CRITICAL

---

## Executive Summary

This file is in a state of severe architectural drift. It violates the `<300 line` rule by **864 lines** and **actively duplicates validation logic that already exists** in the properly-structured `validation/` module.

---

## Violation #1: File Size (CRITICAL)

| Metric | Value |
|--------|-------|
| Actual lines | 1164 |
| Limit | 300 |
| Over by | 864 lines |
| Ratio | 3.88x |

### Breakdown

| Section | Lines | Content |
|---------|-------|---------|
| 1-180 | 180 | Validation functions |
| 182-1164 | 983 | **Inline tests** (should not exist in this file) |

---

## Violation #2: Duplicated Validation Logic (CRITICAL)

### Evidence: Massive Parallel Structure

The `validation/` module already provides proper validation:

| `validation/resource.rs` | `engine/validate.rs` |
|--------------------------|----------------------|
| `validate_step(step, node_count)` | `validate_node_bounds()` — manual `step.as_usize() >= node_count` |
| `validate_slot(slot, slot_count)` | No equivalent, but bounds checks scattered |
| `validate_entry(entry, node_count)` | `validate_transition_target` checks entry indirectly |
| `validate_expr(expr, count)` | None |
| `validate_const(const, count)` | None |

### Evidence: Target Collection Duplication

**`validation/targets.rs`** (89 lines) provides:
```rust
pub(crate) fn collect_node_targets(kind: &CompiledNodeKind, targets: &mut Vec<StepIdx>)
```

**`validate_transition_target`** in `engine/validate.rs` (lines 68-126) reimplementsexactly this logic in a 58-line match statement with **no reuse** of `collect_node_targets`.

### Evidence: Resource Validation Duplication

**`validation/resource.rs`** (248 lines) provides comprehensive resource validation:
- `validate_resource_contract()` — validates all contract limits
- `validate_contract_limit()` — generic limit checker
- `validate_nonzero_u32/u64` — nonzero validators
- `validate_expr_stack_contract()` — expression stack validation

**`validate_resource_contract()`** in `engine/validate.rs` (lines 16-49) reimplements only a **subset** with hardcoded repeated patterns:
```rust
if usize::from(contract.max_steps) > crate::limits::MAX_STEPS_PER_WORKFLOW {
    return Err(WorkflowError::ResourceContractTooLarge { resource: "max_steps" });
}
// ... repeated 6 more times
```

---

## Violation #3: Primitive Obsession

### Pattern: Repeated Index Casting

The file contains **12 instances** of:
```rust
target.as_usize() >= node_count
```

This pattern should be encapsulated as:
```rust
impl StepIdx {
    pub fn is_valid_for(self, count: usize) -> bool {
        self.as_usize() < count
    }
}
```

### Pattern: Massive Enum Match

`validate_transition_target()` (lines 68-126) has a 58-line match on `CompiledNodeKind` that:
1. Duplicates `collect_node_targets()` from `validation/targets.rs`
2. Uses **raw `as_usize()` calls** instead of `StepIdx::is_valid_for()`
3. Has 12 branches with identical structure (`validate_two_step_targets`)

---

## Violation #4: Inline Tests in Production Module

Lines 182-1164 (983 lines) are **inline tests** in a production source file.

Per workspace structure rules:
- Tests belong in `crates/workspace_tests/`
- This file should contain **0 test code**

---

## Scott Wlaschin DDD Violations

### 1. Feature Envy (Anti-pattern)

`validate_transition_target()` envies `CompiledNodeKind`'s internals. It should be:

```rust
// CURRENT (envy):
match &node.kind {
    CompiledNodeKind::Jump { target } if target.as_usize() >= node_count => ...
}

// SHOULD BE (DDD-aligned):
impl CompiledNodeKind {
    fn validate_targets(&self, node_count: usize) -> Result<(), WorkflowError> {
        // encapsulated
    }
}
```

### 2. Primitive Obsession (Anti-pattern)

`usize::from(contract.max_steps) > MAX_STEPS_PER_WORKFLOW` is scattered 7 times.

Should be `ResourceContract::is_within_bounds()` or similar.

### 3. Data Clump (Anti-pattern)

The repeated `(step, node_count)` tuples should be a context/validator struct:

```rust
struct BoundsValidator {
    node_count: usize,
}
impl BoundsValidator {
    fn check_step(&self, step: StepIdx) -> Result<(), WorkflowError> { ... }
}
```

---

## Required Refactors

### 1. Extract Inline Tests (983 lines to remove)

Move all tests in `#[cfg(test)] mod tests` to `crates/workspace_tests/`.

### 2. Delete `validate_resource_contract` (Duplicated)

The proper version exists in `validation/resource.rs:validate_resource_contract()`.

### 3. Replace `validate_transition_target` with Target Collection

Replace the manual match with:
```rust
pub fn validate_transition_target(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    let mut targets = Vec::new();
    for node in &parts.nodes {
        collect_node_targets(&node.kind, &mut targets);
    }
    for target in targets {
        validate_step(target, node_count)?;
    }
    Ok(())
}
```

### 4. Encapsulate Index Validation

Add to `StepIdx` or create `validation::validate_step()` and call it instead of raw `as_usize()`.

### 5. Resulting Line Count

| After Refactor | Lines |
|----------------|-------|
| Validation logic (3 functions) | ~50 |
| Inline tests removed | -983 |
| **Total** | **~50** |

---

## Evidence of Known Good Structure

The `validation/` directory already has proper architecture:

```
src/validation/
├── graph.rs      (8.2K)
├── nodes.rs      (9.8K)
├── resource.rs   (8.3K)  ← properly abstracted resource validation
└── targets.rs    (3.1K)  ← target collection (DUPLICATED in validate.rs!)
```

The `validate.rs` file should **delegate** to these modules, not duplicate them.

---

## Conclusion

**Status:** CRITICAL DRIFT
**Action Required:** Immediate refactor
**Priority:** Blocks further development until resolved

The file must be reduced to <300 lines by:
1. Removing all inline tests (983 lines)
2. Delegating to `validation/resource.rs` for resource contract validation
3. Delegating to `validation/targets.rs` for target collection
4. Using `validation/resource.rs::validate_step()` instead of raw `as_usize()` comparisons
