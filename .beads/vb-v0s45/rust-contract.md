# vb-v0s45: Compiler Primitive Integration Repairs — Contract Rationale

## Five Test Failures — Two Root Causes

### Issue A: Together Primitive Constant Index Expectations (3 tests)

**Failing tests:**
- `compile_workflow_emits_supported_ir_when_each_scoped_primitive_is_valid`
- `compile_source_emits_supported_ir_when_each_scoped_primitive_is_valid`
- `yaml_compiler_compile_emits_supported_ir_when_each_scoped_primitive_is_valid`

**Symptom:** `assertion failed: SetConst node 4 const index: left: 1, right: 0`

**Root cause:** Stale test oracle. The together primitive test case has two branches with different constant values (`value: "1"` and `value: "2"`). These must map to different entries in the constant table. The test oracle incorrectly expected both SetConst nodes to reference const index 0.

**Evidence from master contract:** The compiler must resolve distinct literal values to distinct constant table entries. Two SetConst nodes with different values cannot share the same index.

**Fix:** Update `assert_exact_together` to expect const index 0 for the first SetConst (value 1) and const index 1 for the second SetConst (value 2).

### Issue B: Top-Level `inputs` Declaration Taxonomy (2 tests)

**Failing tests:**
- `compile_source_returns_exact_error_variants_for_contract_taxonomy`
- `public_compile_apis_return_exact_error_variants_for_contract_taxonomy`

**Symptom:** `case top_level_inputs unexpectedly compiled`

**Root cause:** Stale test oracle. The test expects `inputs:` to be an unsupported top-level declaration, but the master language specification (velvet-ballistics-MASTER.md §8) explicitly lists `inputs` as a valid optional top-level field for input schema declarations.

**Evidence from master contract:** velvet-ballistics-MASTER.md §8 "Language Specification" lists optional top-level fields: `inputs`, `vars`, `secrets`, `result`, `examples`.

**Fix:** Remove the `top_level_inputs` test case from both taxonomy tests. The compiler correctly accepts `inputs`.

## What Was NOT Changed

- No implementation changes to `vb_compile` source. The compiler behavior matches the master contract.
- No changes to `vb_runtime` or other crates (out of scope).
- No test weakening — both fixes align the test oracle with the documented contract.

## Verification Plan

1. All 47 tests in `v1_primitive_lowering` pass.
2. `vb_compile` lib tests (496) pass.
3. Source clippy and fmt pass with canonical production flags.
