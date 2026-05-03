# Coverage Gap Analysis Report

**Bead:** vb-cgw1  
**Date:** 2026-05-03  
**Overall Coverage:** 1.30% (56/4306 lines)

## Summary

Coverage is extremely low across all crates. The majority of lines in `vb_compile`, `vb_core`, `vb_storage`, `vb_expr`, `vb_validate`, and `vb_yaml` are untested.

## Tested/Total Lines by Crate

### vb_codegen
- `crates/vb_codegen/src/errors.rs`: 0/2 (0%)

### vb_compile (all 0%)
- `crates/vb_compile/src/ast/parse.rs`: 0/53
- `crates/vb_compile/src/compile_errors.rs`: 0/4
- `crates/vb_compile/src/compile_step_helpers.rs`: 0/6
- `crates/vb_compile/src/compile_step_primitives.rs`: 0/13
- `crates/vb_compile/src/control_flow.rs`: 0/10
- `crates/vb_compile/src/expression.rs`: 0/232
- `crates/vb_compile/src/lib.rs`: 0/130
- `crates/vb_compile/src/references.rs`: 0/5
- `crates/vb_compile/src/schema.rs`: 0/4
- `crates/vb_compile/src/strict_yaml.rs`: 0/5
- `crates/vb_compile/src/tests/helpers.rs`: 0/3
- `crates/vb_compile/src/type_taint.rs`: 0/25
- `crates/vb_compile/src/validate.rs`: 0/63
- `crates/vb_compile/src/workflow_compile.rs`: 0/13
- `crates/vb_compile/src/workflow_optional.rs`: 0/16
- `crates/vb_compile/src/workflow_trigger_validators.rs`: 0/16
- `crates/vb_compile/src/workflow_validators.rs`: 0/34
- `crates/vb_compile/src/yaml_parse.rs`: 0/20
- `crates/vb_compile/src/yaml_profile.rs`: 0/43

### vb_core (most 0%)
- `crates/vb_core/src/action.rs`: 0/59
- `crates/vb_core/src/budget.rs`: 0/328
- `crates/vb_core/src/capability.rs`: 0/26
- `crates/vb_core/src/diagnostic.rs`: 29/38 (76%) — **best coverage**
- `crates/vb_core/src/engine/error_routing.rs`: 0/59
- `crates/vb_core/src/engine/expr_eval/ops.rs`: 0/7
- `crates/vb_core/src/engine/expr_eval/ops_text_list.rs`: 0/24
- `crates/vb_core/src/engine/node_helpers.rs`: 0/25
- `crates/vb_core/src/engine/object_list.rs`: 0/4
- `crates/vb_core/src/engine/step.rs`: 0/44
- `crates/vb_core/src/errors.rs`: 5/54 (9%)
- `crates/vb_core/src/frame.rs`: 0/145
- `crates/vb_core/src/ids.rs`: 0/32
- `crates/vb_core/src/replay/choose.rs`: 0/37
- `crates/vb_core/src/replay/mod.rs`: 0/66
- `crates/vb_core/src/replay/ops.rs`: 0/134
- `crates/vb_core/src/replay/step.rs`: 0/119
- `crates/vb_core/src/span.rs`: 0/4
- `crates/vb_core/src/validation/graph.rs`: 0/29
- `crates/vb_core/src/value.rs`: 0/92
- `crates/vb_core/src/value_store.rs`: 0/153
- `crates/vb_core/src/workflow.rs`: 0/501

### vb_expr
- `crates/vb_expr/src/bytecode/mod.rs`: 0/14
- `crates/vb_expr/src/parser/mod.rs`: 0/88

### vb_ipc
- `crates/vb_ipc/src/error.rs`: 4/26 (15%)
- `crates/vb_ipc/src/frame.rs`: 0/26
- `crates/vb_ipc/src/server/handlers.rs`: 0/3
- `crates/vb_ipc/src/server/helpers.rs`: 0/3

### vb_runtime
- `crates/vb_runtime/src/journal.rs`: 0/4
- `crates/vb_runtime/src/journal_tests.rs`: 0/6
- `crates/vb_runtime/src/lib.rs`: 4/33 (12%)

### vb_storage (all 0%)
- `crates/vb_storage/src/admission.rs`: 0/48
- `crates/vb_storage/src/artifacts.rs`: 0/20
- `crates/vb_storage/src/batch.rs`: 0/76
- `crates/vb_storage/src/binary.rs`: 0/42
- `crates/vb_storage/src/blobs.rs`: 0/9
- `crates/vb_storage/src/codec.rs`: 0/132
- `crates/vb_storage/src/error.rs`: 4/27 (15%)
- `crates/vb_storage/src/events.rs`: 0/26
- `crates/vb_storage/src/headers.rs`: 0/18
- `crates/vb_storage/src/indexes.rs`: 0/12
- `crates/vb_storage/src/journal.rs`: 0/116
- `crates/vb_storage/src/keys.rs`: 0/85
- `crates/vb_storage/src/lib.rs`: 0/4
- `crates/vb_storage/src/queue.rs`: 0/89
- `crates/vb_storage/src/records.rs`: 0/22
- `crates/vb_storage/src/recovery/replay/core.rs`: 0/49
- `crates/vb_storage/src/recovery/types.rs`: 0/15
- `crates/vb_storage/src/snapshots.rs`: 0/10
- `crates/vb_storage/src/types.rs`: 0/10

### vb_ui
- `crates/vb_ui/src/layout.rs`: 0/32

### vb_validate (all 0%)
- `crates/vb_validate/src/control_flow.rs`: 0/64
- `crates/vb_validate/src/diagnostic.rs`: 10/149 (7%)
- `crates/vb_validate/src/gates.rs`: 0/1
- `crates/vb_validate/src/schema.rs`: 0/167
- `crates/vb_validate/src/type_taint.rs`: 0/172

### vb_yaml (all 0%)
- `crates/vb_yaml/src/ast/parse.rs`: 0/2
- `crates/vb_yaml/src/ast_helpers.rs`: 0/2
- `crates/vb_yaml/src/profile.rs`: 0/12
- `crates/vb_yaml/src/profile_dupkeys.rs`: 0/12

### benches
- `benches/velvet_ballastics.rs`: 0/3

## Key Observations

1. **Extremely low overall coverage** (1.30%) indicates the project is in early development with minimal test infrastructure.

2. **Best covered modules**:
   - `vb_core/src/diagnostic.rs`: 76% (29/38 lines)
   - `vb_core/src/errors.rs`: 9% (5/54 lines)
   - `vb_ipc/src/error.rs`: 15% (4/26 lines)
   - `vb_storage/src/error.rs`: 15% (4/27 lines)
   - `vb_validate/src/diagnostic.rs`: 7% (10/149 lines)
   - `vb_runtime/src/lib.rs`: 12% (4/33 lines)

3. **Uncovered critical paths**:
   - All of `vb_core/replay/*` (replay/choose.rs, replay/ops.rs, replay/step.rs, replay/mod.rs) — 0% each
   - All of `vb_storage` — 0% across all modules
   - All of `vb_compile/expression.rs` — 0% (232 lines)
   - All of `vb_core/workflow.rs` — 0% (501 lines)
   - All of `vb_core/budget.rs` — 0% (328 lines)

4. **Code marked as tested**: Only 56 lines total are covered.

## Recommendations

1. **High priority**: Write tests for `vb_core/replay/*` modules — these contain critical replay logic
2. **High priority**: Write tests for `vb_storage/*` modules — these handle persistence and recovery
3. **Medium priority**: Write tests for `vb_compile/expression.rs` (232 lines) and `vb_core/workflow.rs` (501 lines)
4. **Existing test infrastructure**: `crates/vb_compile/src/tests/helpers.rs` exists, suggesting tests are planned but not yet implemented
