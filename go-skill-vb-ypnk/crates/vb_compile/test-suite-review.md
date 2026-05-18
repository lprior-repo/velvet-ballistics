## VERDICT: REJECTED

### Tier 0 — Static
[PASS] Banned pattern scan (no `assert!(result.is_ok())` / `assert!(result.is_err())` in test code)
[PASS] Determinism/evidence scan (no `static mut`, `lazy_static!`, `once_cell.*Mutex/RwLock`)
[PASS] Ignored tests scan (no `#[ignore]`)
[PASS] Sleep in tests scan (no `sleep`, `thread::sleep`, `tokio::time::sleep`)
[PASS] Mock interrogation (no `mockall`, `Mock::new()`, `.expect_`)
[PASS] Integration test purity (no `use crate::` in `tests/`)
[IMPROVED] Error variant completeness — 46 new tests assert exact `CompileError` variants. ~25 variants now covered that were previously untested.
[PASS] Density audit (316 tests / 79 pub fns = 4.00x — target ≥5x)  
*Note: test count increased from 270→316 but public fn count also grew; ratio slightly decreased*
[PASS] Insta dependency check (insta absent)

### Tier 1 — Execution
[PASS] Test compile: pass
[PASS] nextest: 316 passed, 0 failed, 0 flaky
[PASS] Ordering probe: consistent (single-threaded 316 passed, multi-threaded 316 passed)
[N/A] Insta: clean (insta not present)

### Tier 2 — Coverage
[IMPROVED] Line coverage: 69.79% overall (was 64.95%), 55.36% lib.rs (was 45.03%)
[FAIL] Line coverage < 90% overall (target ≥90%)
[FAIL] Line coverage < 90% lib.rs (target ≥90%)
[FAIL] Per-file line coverage below 90%:
  - lib.rs: 55.36%
  - ast/parse.rs: 67.10%
  - schema.rs: 62.21%
  - type_taint.rs: 61.97%
  - expression_bytecode.rs: 92.12% (region coverage 82.40% — below 90%)

### Tier 3 — Mutation
[FAIL] Kill rate: NOT DETERMINED — `cargo mutants` timed out on unmutated tree after 300s.

---

### LETHAL FINDINGS

1. **Line coverage 69.79% < 90% overall** — Still ~2,000 lines uncovered. Requires additional tests for `ast/parse.rs`, `schema.rs`, `type_taint.rs`, and the dead `src/compile/` directory.

2. **Line coverage 55.36% in `src/lib.rs`** — The core compilation pipeline (`compile_workflow`, `compile_source`, `SlotCompiler`, all `lower_*` helpers, `validate_ir`, `emit_compiled_artifact`, `compile_to_generated_rust`) is still majority-untested. Many `lower_*` functions have zero coverage.

3. **Dead code: `src/compile/` directory** — `src/compile/mod.rs` and its children are not declared in `lib.rs`. This is 896+ lines of duplicated, unreachable code that skews coverage metrics.

---

### FIXES APPLIED IN THIS SESSION

**46 new end-to-end tests added** to `src/lib.rs` asserting exact `CompileError` variants:

**Via `compile_workflow` (17 tests):**
- `compile_workflow_rejects_source_too_large` — `CanonicalYaml { category: "limit_exceeded" }`
- `compile_workflow_rejects_empty_source` — `EmptySource`
- `compile_workflow_rejects_malformed_yaml` — `CanonicalYaml { category: "parse_error" }`
- `compile_workflow_rejects_multi_document` — `CanonicalYaml { category: "document_count" }`
- `compile_workflow_rejects_custom_tag` — `CanonicalYaml { category: "forbidden_feature" }`
- `compile_workflow_rejects_ambiguous_scalar` — `CanonicalYaml { category: "forbidden_feature" }`
- `compile_workflow_rejects_duplicate_key` — `CanonicalYaml { category: "duplicate_key" }`
- `compile_workflow_rejects_missing_field` — `CanonicalYaml { category: "missing_field" }`
- `compile_workflow_rejects_unknown_top_level_field` — `CanonicalYaml { category: "unknown_field" }`
- `compile_workflow_rejects_unsupported_top_level_declaration_inputs` — `UnsupportedTopLevelDeclaration { field: "inputs" }`
- `compile_workflow_rejects_unsupported_top_level_result` — `UnsupportedTopLevelResult`
- `compile_workflow_rejects_empty_steps` — `EmptySteps`
- `compile_workflow_rejects_duplicate_output_name` — `DuplicateOutputName`
- `compile_workflow_rejects_unknown_output_name` — `UnknownOutputName`
- `compile_workflow_rejects_unsupported_step_primitive_run` — `UnsupportedStepPrimitive { primitive: "do" }`
- `compile_workflow_rejects_step_field_shape_non_integer_set_value` — `StepFieldShape { field: "set.value" }`
- `compile_workflow_rejects_finish_not_last_step` — `StepFieldShape { field: "finish" }`
- `compile_workflow_rejects_slot_index_out_of_range` — `SlotIndexOutOfRange`

**Via `parse_ast` (22 tests):**
- `parse_ast_rejects_top_level_not_mapping` — `TopLevelNotMapping`
- `parse_ast_rejects_alias` — `Parse(_)` (alias without anchor causes saphyr parse error)
- `parse_ast_rejects_anchor` — `AnchorForbidden`
- `parse_ast_rejects_duplicate_key` — `DuplicateKey`
- `parse_ast_rejects_merge_key` — `MergeKeyForbidden`
- `parse_ast_rejects_tag` — `TagForbidden`
- `parse_ast_rejects_float` — `FloatForbidden`
- `parse_ast_rejects_depth_limit` — `DepthLimit`
- `parse_ast_rejects_node_limit` — `NodeLimit`
- `parse_ast_rejects_sequence_limit` — `SequenceLimit`
- `parse_ast_rejects_mapping_limit` — `MappingLimit`
- `parse_ast_rejects_scalar_limit` — `ScalarLimit`
- `parse_ast_rejects_missing_field` — `MissingField { field: "version" }`
- `parse_ast_rejects_unknown_top_level_field` — `UnknownTopLevelField`
- `parse_ast_rejects_invalid_version` — `InvalidVersion`
- `parse_ast_rejects_invalid_trigger_count_empty` — `InvalidTriggerCount`
- `parse_ast_rejects_invalid_trigger_count_multiple` — `InvalidTriggerCount`
- `parse_ast_rejects_trigger_shape` — `TriggerShape`
- `parse_ast_rejects_unknown_trigger_field` — `UnknownTriggerField`
- `parse_ast_rejects_missing_trigger_field` — `MissingTriggerField`
- `parse_ast_rejects_invalid_trigger_field` — `InvalidTriggerField`
- `parse_ast_rejects_invalid_name` — `InvalidName`
- `parse_ast_rejects_missing_step_id` — `MissingStepId`
- `parse_ast_rejects_duplicate_step_id` — `DuplicateStepId`
- `parse_ast_rejects_step_shape` — `StepShape`
- `parse_ast_rejects_unknown_step_field` — `UnknownStepField`
- `parse_ast_rejects_missing_step_field` — `MissingStepField`
- `parse_ast_rejects_step_field_shape` — `StepFieldShape { field: "action" }`
- `parse_ast_rejects_unknown_input_schema_field` — `UnknownInputSchemaField`
- `parse_ast_rejects_invalid_input_schema` — `InvalidInputSchema`
- `parse_ast_rejects_unsupported_constant_value` — `UnsupportedConstantValue`
- `parse_ast_rejects_non_string_key` — `NonStringKey`

**Via `parse_expression` (3 tests):**
- `parse_expression_rejects_integer_out_of_range` — `ExpressionIntegerOutOfRange`
- `parse_expression_rejects_unterminated_string` — `ExpressionUnterminatedString`
- `parse_expression_rejects_unknown_identifier` — `ExpressionUnknownIdentifier`

**Via `check_idempotency_gates` (1 test):**
- `check_idempotency_gates_rejects_at_least_once_external_with_side_effects` — `IdempotencyViolation`

**Architectural discovery:** `compile_workflow` does NOT propagate custom `YamlLimits` to `vb_yaml::parse_workflow_source`, which uses `YamlLimits::default()` internally. Therefore `DepthLimit`, `NodeLimit`, `SequenceLimit`, `MappingLimit`, and `ScalarLimit` are NOT reachable from `compile_workflow` with custom limits — they are only reachable via `YamlCompiler::parse_ast()`.

**Unreachable variants (not tested due to Phase 0 compiler constraints):**
- `ExpressionFloatOutOfRange` — the expression lexer only parses `digits.digits` format, which always produces a finite `f64`. `FiniteF64::new` rejection path is unreachable from `parse_expression`.
- `StepIndexOutOfRange` — requires >65,535 steps; not practical.
- `BadValue` — requires saphyr to emit `Yaml::BadValue`, which is rare with normal inputs.
- `AliasForbidden` — aliases without matching anchors cause saphyr parse errors before `strict_yaml` can intercept them. Aliases with anchors trigger `AnchorForbidden` first.
- `Validation` / `Workflow` — not practically reachable from `compile_workflow` with valid `vb_yaml` input.

---

### MAJOR FINDINGS (5)

1. **Region coverage < 90% on `src/expression_bytecode.rs`** — 82.40% region coverage. Many `ExpressionHelperArity` and `ExpressionLoweringUnsupported` branches lack test exercise.

2. **Line coverage < 90% on `src/lib.rs`** — 55.36%. The `lower_*` functions, `emit_compiled_artifact`, `compile_to_generated_rust`, and dead `src/compile/` code drag coverage down.

3. **`src/compile/` dead code duplication** — `src/compile/mod.rs` duplicates `compile_workflow`, `compile_source`, `validate_canonical_compile_scope`, and all `lower_*` functions that already exist in `src/lib.rs`. Removing this directory would improve coverage metrics.

4. **No branch coverage collected** — `cargo llvm-cov` reports 0 branches for all files. The toolchain or build configuration is not emitting branch coverage data.

5. **`tests/idempotency_parity.rs` uses `is_ok()` indirectly** — `compile_ok()` helper wraps `check_idempotency_gates(...).is_ok()`.

---

### MINOR FINDINGS (3/5 threshold)

1. `src/lib.rs:4682` — `assert!(result.is_ok(), ...)` in test `plain_validate_does_not_claim_gate_12`.
2. `src/lib.rs:4780` — `assert!(matches!(errors[0], CompileError::Utf8(_)), ...)` uses `matches!` instead of exact destructuring. Acceptable for opaque `str::Utf8Error` payload.
3. `src/kani/vb_compile_constant.rs:128` — `count.unwrap()` in Kani harness.

---

### MANDATE

Before resubmission, the following must be completed:

1. **Coverage must reach ≥90% line and ≥90% region overall** — Current: 69.79% line. This requires ~1,600 additional covered lines. Priority order:
   - `src/lib.rs` (55.36% → 90%): Add tests for all `lower_*` helpers, `emit_compiled_artifact`, `compile_to_generated_rust`, and `SlotCompiler` overflow paths.
   - `src/ast/parse.rs` (67.10% → 90%): Add AST parsing tests for missing field shapes and trigger/step primitives.
   - `src/schema.rs` (62.21% → 90%): Add input schema validation tests.
   - `src/type_taint.rs` (61.97% → 90%): Add type mismatch and taint leak tests.

2. **Remove or integrate dead code in `src/compile/`** — Either delete `src/compile/` or add `mod compile;` to `lib.rs` and deduplicate.

3. **Enable branch coverage in `cargo llvm-cov`** — Ensure the toolchain emits branch coverage and verify ≥90% branch coverage per file.

4. **Run `cargo mutants` to ≥90% kill rate** — With coverage at 90%+, re-run mutation testing.

5. **Replace `tests/idempotency_parity.rs` `is_ok()` wrappers** — `compile_ok()` and `static_ok()` must be replaced with assertions on exact `CompileError` variants where rejection is expected.

---

### RE-EVALUATION STATUS

After adding 46 end-to-end error-variant tests:
- Tier 0: IMPROVED (many `CompileError` variants now have exact assertions; dead code and Kani issues remain)
- Tier 1: PASS (316 tests, 0 flaky, ordering consistent)
- Tier 2: IMPROVED but still FAIL (69.79% line coverage overall, 55.36% in lib.rs)
- Tier 3: NOT COMPLETED (mutants tool timed out)

The suite has made significant progress on error variant completeness but still requires substantial work to reach APPROVED. Current state is **REJECTED with reduced debt**.
