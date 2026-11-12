# vb-te1i Baseline Report

## Build
- **cargo build --workspace**: PASS (0 errors, 2 warnings, 230 crates)

## Test
- **cargo test --workspace**: 3 FAILED / 1400+ PASSED
  - Failures are pre-existing in `crates/vb_compile/tests/v1_primitive_lowering.rs:1282`:
    - `compile_source_emits_supported_ir_when_each_scoped_primitive_is_valid`
    - `compile_workflow_emits_supported_ir_when_each_scoped_primitive_is_valid`
    - `yaml_compiler_compile_emits_supported_ir_when_each_scoped_primitive_is_valid`
  - All failures: assertion `left == right` failed: SetConst node 1 next target (left: Some(2), right: None)

## Clippy
- **cargo clippy --workspace --lib -- -D warnings**: FAIL
  - `dead_code` violations in `crates/vb_cli/src/lifecycle.rs`:
    - `get_state` method never used
    - `with_tracker` function never used
