# vb_codegen/src/lib.rs Architectural Refactor

## Summary
Split monolithic 6378-line `vb_codegen/src/lib.rs` into focused modules ≤300 lines each.

## Original File
- `lib.rs` - 6378 lines (exceeded limit by 6078 lines)

## New Module Structure

| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | 118 | Thin facade with module declarations and public re-exports |
| `error.rs` | 47 | `CodegenError` enum and `CodegenResult` type alias |
| `validation.rs` | 142 | `validate_generated_subset` and IR validation helpers |
| `helpers.rs` | 157 | `write_header`, `write_next_or_error`, `emit_accessor_eval` |
| `emit_workflow.rs` | 180 | Main workflow emission: `emit_rust_workflow`, `emit_ids`, `emit_drive_function`, `emit_action_match_dispatch`, `emit_finish`, `emit_action_boundary`, `emit_trybuild_fixture` |
| `constants.rs` | 50 | `emit_constants` and `count_constants` |
| `resource.rs` | 107 | `emit_resource_contract` |
| `emit_steps.rs` | 47 | `emit_step_function` and `emit_step_body` dispatch |
| `emit_linear.rs` | 90 | Linear step emission: `emit_linear_step_body`, `emit_nop_step`, `emit_set_const_step`, `emit_copy_step`, `emit_eval_expr_step`, `emit_continue_step` |
| `emit_branch.rs` | 59 | Branch step emission: `emit_branch_step_body`, `emit_choose_step`, `emit_choose_slot_step`, `emit_choice_fallback` |
| `emit_boundary.rs` | 114 | Boundary step emission: `emit_boundary_step_body`, `emit_wait_until_step`, `emit_wait_event_step`, `emit_ask_step`, `emit_optional_timeout_read`, `emit_ask_resume_step`, `emit_error_handler_step` |
| `emit_unsupported.rs` | 47 | Unsupported node/expression emission: `emit_unsupported_node_step`, `emit_unsupported_step`, `emit_unsupported_expr` |
| `emit_expr.rs` | 119 | Expression emission: `emit_expr_function` |
| `semcheck.rs` | 141 | Semantic verification: `compare_generated_to_ir`, `reject_generated_pattern`, `require_generated_pattern` |
| `tests/mod.rs` | 5055 | All tests (exempt from line limit) |

## Verification

**Line Count Compliance:**
- All source files ≤300 lines: ✓
- Test files exempt per engineering rules: ✓

**File Count:**
- Total modules: 15 source files + 1 test file
- Original monolithic file eliminated

## Notes
- Workspace has a pre-existing vb_core compilation issue unrelated to this refactor
- All emit_* functions use `crate::*` paths for cross-module calls
- Public API re-exports maintained in `lib.rs` for backward compatibility
