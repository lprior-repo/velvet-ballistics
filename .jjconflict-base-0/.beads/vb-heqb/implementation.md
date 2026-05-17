# Implementation: vb-heqb - On_error/then Handler Routing (Phase 21)

## Summary
Implemented proper error handler slot passing through the `ErrorHandler` node kind by adding an `error_slot` field.

## Changes Made

### Core Data Structure Changes

**crates/vb_core/src/workflow/node.rs**
- Added `error_slot: Option<SlotIdx>` field to `CompiledNodeKind::ErrorHandler` variant
- Updated doc comments to clarify the new field's purpose

**crates/vb_core/src/nodes.rs**
- Mirrored the `ErrorHandler` change from node.rs

**crates/vb_core/src/workflow.rs**
- Updated `ErrorHandler` variant definition to include `error_slot`
- Updated all pattern matches to use `..` for the new field

### Error Handler Lookup Enhancement

**crates/vb_runtime/src/shard/helpers.rs**
- Modified `find_error_handler_for_failure` to return `Option<(StepIdx, Option<SlotIdx>)>` instead of `Option<StepIdx>`
- Now returns both the handler step index AND the error slot (if configured)
- Updated `error_handler_on_node` helper to extract and return both values

### Shard Lifecycle Fix

**crates/vb_runtime/src/shard/lifecycle.rs**
- Updated `handle_action_failure` to handle the new tuple return type
- When routing to an error handler, now writes the failed step index to the configured `error_slot` as an `I64` value
- If slot write fails, continues without failing (graceful degradation)

### Pattern Match Updates (across multiple files)

Updated all pattern matches on `ErrorHandler` to include `..` to ignore the new `error_slot` field:
- `crates/vb_core/src/budget.rs`
- `crates/vb_core/src/workflow/validation/edges.rs`
- `crates/vb_core/src/workflow/validation/reachability.rs`
- `crates/vb_core/src/workflow/validation/kind.rs`
- `crates/vb_core/src/engine/validate.rs`
- `crates/vb_core/src/workflow.rs` (3 locations)
- `crates/vb_core/src/validation/targets.rs`
- `crates/vb_core/src/validation/nodes.rs`
- `crates/vb_codegen/src/emit/step.rs`
- `crates/vb_codegen/src/lib.rs`
- `crates/vb_validate/src/gates.rs`
- `crates/vb_ui/src/graph_builder.rs` (2 locations)
- `crates/vb_ui/src/graph_renderer.rs`
- `crates/vb_ui/src/verify/certificates.rs`
- `crates/vb_runtime/src/engine/execute.rs`
- `crates/vb_runtime/src/engine/step_engine.rs`

### Test Updates

Updated test workflows that construct `ErrorHandler` nodes:
- `crates/vb_runtime/src/shard/tests.rs`
- `crates/vb_core/tests/section36_mandatory_coverage.rs`
- `crates/vb_ui/src/graph_builder.rs`
- `crates/vb_ui/src/graph_renderer.rs`
- `crates/vb_codegen/src/tests.rs`

## Constraint Adherence

All changes follow the Big 6 constraints:
1. **Data → Calc → Actions**: Error routing logic remains in pure calculation functions; actions (slot writes) are minimal and at the shell boundary
2. **Zero Mutability**: No `mut` keywords added in core logic
3. **Zero Panics/Unwraps**: Added explicit `if let` for slot write instead of `expect()`
4. **Make Illegal States Unrepresentable**: The `Option<SlotIdx>` for error_slot makes missing slot explicit
5. **Expression-Based**: Uses expression-based returns
6. **Clippy Flawless**: Passes with `0 errors`

## Verification

- `cargo test`: 81 tests passed
- `cargo fmt --check`: Passed
- `cargo clippy`: 0 errors, 2 warnings (pre-existing)