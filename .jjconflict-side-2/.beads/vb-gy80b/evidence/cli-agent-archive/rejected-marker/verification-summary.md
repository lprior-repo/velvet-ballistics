# vb-gy80b Migration Path Guard Fix - Verification Summary

## Problem
The `use crate::expr_proofs as _;` approach in `proof_module_paths::expr_proofs_module_accessible` failed with `-D warnings` due to unused import warning. This was because:
1. `expr_proofs` module is cfg(kani) gated - empty in non-kani builds
2. `use ... as _` is still flagged as unused import under `-D warnings`
3. Module-level `use` and `pub use` also failed with unused/unreachable warnings

## Solution
Added a minimal public marker type `ExprProofsModule` to `crates/vb_expr/src/expr_proofs/mod.rs` that is always available (not cfg-gated). The migration test references this type via `std::any::type_name` to prove the module path resolves at compile time, following the same pattern as the existing `yaml_ast` test.

## Files Changed
1. `crates/vb_compile/src/migration_compat.rs` - Updated test to use marker type approach
2. `crates/vb_expr/src/expr_proofs/mod.rs` - Added `pub struct ExprProofsModule` marker type

## Test Results

### Positive Test (correct path)
```
running 9 tests
test migration_compat::expr_module_paths::expr_modules_resolve ... ok
test migration_compat::expr_module_paths::expr_full_pipeline_via_old_paths ... ok
test migration_compat::expr_module_paths::expr_lexer_behavioral_check ... ok
test migration_compat::yaml_module_paths::yaml_ast_module_accessible ... ok
test migration_compat::yaml_module_paths::yaml_modules_resolve ... ok
test migration_compat::proof_module_paths::expr_proofs_module_accessible ... ok
test migration_compat::yaml_module_paths::yaml_profile_rejects_multiple_documents ... ok
test migration_compat::yaml_module_paths::yaml_profile_rejects_anchors ... ok
test migration_compat::yaml_module_paths::yaml_profile_rejects_duplicate_keys ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 488 filtered out
```

### Negative Test (broken path)
```
error[E0425]: cannot find type `ExprProofsBroken` in module `expr_proofs`
   --> crates/vb_compile/src/migration_compat.rs:117:52
    |
117 |         let _ = std::any::type_name::<expr_proofs::ExprProofsBroken>();
    |                                                    ^^^^^^^^^^^^^^^^ not found in `expr_proofs`
```
Confirms the check is non-vacuous - a broken path fails to compile.

### Full vb_compile lib test suite
```
test result: ok. 497 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Pre-existing Issues (not caused by this change)
Integration tests in `crates/vb_compile/tests/` fail due to missing `WorkflowSourceParts`
export when not built with `test` or `test-util` features. This is a pre-existing cfg
gating issue unrelated to the migration path guard fix.
