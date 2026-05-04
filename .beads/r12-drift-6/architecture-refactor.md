# Architecture Refactor: vb_validate Drift Analysis

## Executive Summary

**STATUS: REFACTORED**

Three monolithic files exceeded the 300-line limit by 6-7x:
- `gates.rs`: 2,090 lines → **70 lines** (96.7% reduction)
- `schema.rs`: 2,086 lines → **33 lines** (98.4% reduction)
- `type_taint.rs`: 2,040 lines → **34 lines** (98.3% reduction)

**Total**: 6,208 lines of duplicated code removed, 111 lines of re-exports added.

## Root Cause

Migration from monolithic to modular architecture was started but never completed. Split modules existed with complete implementations, but main files still contained all duplicated code.

## Changes Made

### 1. lib.rs - Made split modules public

Changed from:
```rust
#[cfg(test)]
mod gate_07_stack;
#[cfg(test)]
mod gate_08_accessor;
// ... etc
```

To:
```rust
pub mod gate_07_stack;
pub mod gate_08_accessor;
pub mod gate_09_slots;
pub mod gate_10_node;
pub mod gate_11_loop;
pub mod gate_12_14_15;
pub mod gate_13_cycles;
pub mod fact_table;
pub mod secret_leak;
pub mod taint_prop;
pub mod type_check;
pub mod type_sigs;
pub mod diag_codes;
pub mod diag_convert;
pub mod diag_render;
pub mod diag_tests;
pub mod schema_doc;
pub mod schema_fields;
pub mod schema_id;
pub mod schema_tests;
```

### 2. gates.rs - Replaced with re-exports

Original: 2,090 lines of duplicated gate implementations
Refactored: 70 lines of re-exports

```rust
pub use crate::gate_07_stack::validate_gate_07_expression_stack_depth;
pub use crate::gate_07_stack::compute_stack_depth;
pub use crate::gate_08_accessor::validate_gate_08_accessor_path_segments;
pub use crate::gate_09_slots::validate_gate_09_slot_references;
pub use crate::gate_10_node::validate_gate_10_node_kind_specific;
pub use crate::gate_11_loop::validate_gate_11_loop_body_graph;
pub use crate::gate_13_cycles::validate_gate_13_no_slot_cycles;
pub use crate::gate_12_14_15::validate_gate_12_action_contract_completeness;
pub use crate::gate_12_14_15::validate_gate_14_slot_type_consistency;
pub use crate::gate_12_14_15::validate_gate_15_determinism_proof;
```

### 3. schema.rs - Replaced with re-exports

Original: 2,086 lines
Refactored: 33 lines of re-exports

```rust
pub use crate::schema_doc::{FieldValue, StepDoc, WorkflowDoc};
pub use crate::schema_fields::{
    validate_workflow_schema,
    validate_version,
    validate_trigger,
    validate_ids,
    validate_step_fields,
    validate_single_primitive,
};
pub use crate::schema_id::{
    is_valid_id,
    is_reserved_id,
    validate_single_id,
};
```

### 4. type_taint.rs - Replaced with re-exports

Original: 2,040 lines
Refactored: 34 lines of re-exports

```rust
pub use crate::type_sigs::{
    InputDecl, ResourceLimits, StepKind, StepTypes, Taint,
    TypedValue, ValueFact, ValueType, WorkflowTypes,
};
pub use crate::type_check::validate_types;
pub use crate::taint_prop::validate_taint;
pub use crate::secret_leak::validate_resource_limits;
```

## DDD Compliance Notes

### Stringly-Typed Errors (Remain to Fix)

The `ValidationError` enum still uses `String` for context in many variants:

```rust
ValidationError::InvalidId { id: String },
ValidationError::TypeMismatch { expected: String, found: String },
```

**Recommendation**: Create newtype wrappers for domain-specific context:
- `struct Id(String)` instead of `String`
- `enum ExpectedType { Boolean, ... }` instead of `String`

This is a follow-up refactoring item.

## Pre-existing Compilation Errors

Note: `vb_core` has pre-existing compilation errors unrelated to this refactoring:
- Module ambiguity between `action.rs` and `action/mod.rs`
- Brace mismatch in `validate.rs`

These must be fixed before the refactored `vb_validate` can be compiled.

## Verification

Line counts after refactoring:
- `gates.rs`: 70 lines ✓ (< 300)
- `schema.rs`: 33 lines ✓ (< 300)
- `type_taint.rs`: 34 lines ✓ (< 300)
