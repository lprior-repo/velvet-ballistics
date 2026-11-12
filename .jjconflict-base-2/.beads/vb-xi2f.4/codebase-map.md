# Codebase Map: vb-xi2f.4

## Production Code
- crates/vb_compile/src/mod_compile_lowering/part_01.rs — unchecked emission site
- crates/vb_compile/src/mod_compile_core.rs — public compile APIs
- crates/vb_compile/Cargo.toml — dependency features

## Core Types
- crates/vb_core/src/workflow/mod.rs — CompiledWorkflow, try_from_parts
- crates/vb_core/src/compiled_workflow.rs — re-export

## Validation
- crates/vb_core/src/workflow/validate.rs — validate_parts, validate_budget
- crates/vb_validate/src/shared.rs — shared validation gates

## Tests (existing)
- crates/workspace_tests/tests/integration_compile_codegen_pipeline.rs
- crates/workspace_tests/tests/integration_compile_codegen_runtime_e2e.rs
- crates/workspace_tests/tests/integration_storage_runtime_validate_pipeline.rs
