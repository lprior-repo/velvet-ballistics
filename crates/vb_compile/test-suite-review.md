## VERDICT: REJECTED

### Tier 0 — Static
[PASS] Banned pattern scan (no `assert!(result.is_ok())` / `assert!(result.is_err())` in test code)
[PASS] Determinism/evidence scan (no `static mut`, `lazy_static!`, `once_cell.*Mutex/RwLock`)
[PASS] Ignored tests scan (no `#[ignore]`)
[PASS] Sleep in tests scan (no `sleep`, `thread::sleep`, `tokio::time::sleep`)
[PASS] Mock interrogation (no `mockall`, `Mock::new()`, `.expect_` — `.expect_err()` at `src/lib.rs:4218` is legitimate `Result` method, not a mock)
[PASS] Integration test purity (no `use crate::` in `tests/`)
[FAIL] Error variant completeness — numerous `CompileError` variants lack exact test assertions
[PASS] Density audit (410 tests / 79 pub fns = 5.19x — target ≥5x)
[PASS] Insta dependency check (insta absent)

### Tier 1 — Execution
[PASS] Test compile: pass
[PASS] nextest: 270 passed, 0 failed, 0 flaky
[PASS] Ordering probe: consistent (single-threaded 270 passed, multi-threaded 270 passed)
[N/A] Insta: clean (insta not present)

### Tier 2 — Coverage
[FAIL] Line coverage: 64.95% overall (target ≥90%), 45.03% lib.rs (Calc layer effectively <95%)
[FAIL] Branch coverage: not collected by current toolchain configuration; region coverage 64.95% overall
[FAIL] Per-file line coverage below 90%:
  - lib.rs: 45.03%
  - ast/parse.rs: 67.10%
  - schema.rs: 62.21%
  - type_taint.rs: 61.97%
  - expression.rs: 87.61%
  - control_flow.rs: 89.52%
  - expression_bytecode.rs: 92.12% (region coverage 82.40% — below 90%)

### Tier 3 — Mutation
[FAIL] Kill rate: NOT DETERMINED — `cargo mutants` timed out on unmutated tree after 300s. Likely poor kill rate given 64.95% line coverage.

---

### LETHAL FINDINGS

1. **Line coverage 64.95% < 90% overall** — The suite covers fewer than 2/3 of executable lines. 3,731 lines are never executed by any test.

2. **Line coverage 45.03% in `src/lib.rs`** — The core compilation pipeline (`compile_workflow`, `compile_source`, `SlotCompiler`, all `lower_*` helpers, `validate_ir`, `emit_compiled_artifact`, `compile_to_generated_rust`) is majority-untested.

3. **`CompileError::Utf8` untested** — `src/lib.rs:1199` defines `Utf8(#[from] str::Utf8Error)`, reachable via `compile_workflow(&[invalid_utf8])`. No test asserted this exact variant. **FIXED during review** — test `compile_workflow_rejects_invalid_utf8` added at `src/lib.rs:4776`.

4. **`CompileError::SourceTooLarge` untested** — `src/lib.rs:1191`, reachable via `compile_workflow` with oversized input. No exact test assertion.

5. **`CompileError::EmptySource` untested** — `src/lib.rs:1202`, reachable via `compile_workflow(b"")`. No exact test assertion.

6. **`CompileError::Parse` untested** — `src/lib.rs:1205`, reachable via `compile_workflow` with malformed YAML. No exact test assertion.

7. **`CompileError::DocumentCount` untested** — `src/lib.rs:1216`, reachable via multi-document YAML streams. No exact test assertion.

8. **`CompileError::TopLevelNotMapping` untested** — `src/lib.rs:1222`, reachable via non-mapping top-level YAML. No exact test assertion.

9. **`CompileError::NonStringKey` untested** — `src/lib.rs:1225`, reachable via numeric YAML mapping keys. No exact test assertion.

10. **`CompileError::DuplicateKey` untested** — `src/lib.rs:1231`, reachable via duplicate YAML keys. No exact test assertion.

11. **`CompileError::MergeKeyForbidden` untested** — `src/lib.rs:1251`, reachable via YAML `<<` merge keys. No exact test assertion.

12. **`CompileError::TagForbidden` untested** — `src/lib.rs:1257`, reachable via YAML custom tags. No exact test assertion.

13. **`CompileError::BadValue` untested** — `src/lib.rs:1263`, reachable via saphyr bad scalars. No exact test assertion.

14. **`CompileError::FloatForbidden` untested** — `src/lib.rs:1266`, reachable via floating-point YAML scalars. No exact test assertion.

15. **`CompileError::DepthLimit` untested** — `src/lib.rs:1269`, reachable via deeply nested YAML. No exact test assertion.

16. **`CompileError::NodeLimit` untested** — `src/lib.rs:1277`, reachable via YAML with too many nodes. No exact test assertion.

17. **`CompileError::SequenceLimit` untested** — `src/lib.rs:1283`, reachable via overly long YAML sequences. No exact test assertion.

18. **`CompileError::MappingLimit` untested** — `src/lib.rs:1291`, reachable via overly large YAML mappings. No exact test assertion.

19. **`CompileError::ScalarLimit` untested** — `src/lib.rs:1299`, reachable via overly long YAML scalars. No exact test assertion.

20. **`CompileError::Validation` untested from public API** — `src/lib.rs:1310`, produced by `vb_validate`. While `lib.rs` tests assert `Validation` in `test_validate_error_parity`, no `#[test]` in `lib.rs` drives a public `compile_workflow` call into a `Validation` error path.

21. **`CompileError::Workflow` untested from public API** — `src/lib.rs:1307`, produced by `WorkflowError`. No public-API test asserts this exact variant.

22. **`CompileError::MissingField` untested** — `src/lib.rs:1313`. No exact test assertion.

23. **`CompileError::UnknownTopLevelField` untested** — `src/lib.rs:1319`. No exact test assertion.

24. **`CompileError::InvalidVersion` untested** — `src/lib.rs:1325`. No exact test assertion.

25. **`CompileError::InvalidTriggerCount` untested from end-to-end** — `src/lib.rs:1331`. Asserted in `src/ast/tests.rs` at AST-parse level, but no `compile_workflow` end-to-end test asserts it.

26. **`CompileError::TriggerShape` untested** — `src/lib.rs:1343`. No exact test assertion.

27. **`CompileError::UnknownTriggerField` untested** — `src/lib.rs:1351`. No exact test assertion.

28. **`CompileError::MissingTriggerField` untested** — `src/lib.rs:1359`. No exact test assertion.

29. **`CompileError::InvalidTriggerField` untested** — `src/lib.rs:1367`. No exact test assertion.

30. **`CompileError::UnknownInputSchemaField` untested** — `src/lib.rs:1385`. No exact test assertion.

31. **`CompileError::InvalidInputSchema` untested** — `src/lib.rs:1391`. No exact test assertion.

32. **`CompileError::UnsupportedTopLevelResult` untested** — `src/lib.rs:1399`. No exact test assertion.

33. **`CompileError::UnsupportedTopLevelDeclaration` untested from end-to-end** — `src/lib.rs:1402`. Asserted in `src/lib.rs` via `compile_source`, but not via `compile_workflow`.

34. **`CompileError::DuplicateOutputName` untested from end-to-end** — `src/lib.rs:1405`. Asserted in `src/type_taint/tests.rs` at AST level, but not via `compile_workflow`.

35. **`CompileError::UnknownOutputName` untested from end-to-end** — `src/lib.rs:1408`. Asserted in `src/type_taint/tests.rs` at AST level, but not via `compile_workflow`.

36. **`CompileError::EmptySteps` untested from end-to-end** — `src/lib.rs:1411`. Asserted in `src/lib.rs` via `compile_source`, but not via `compile_workflow`.

37. **`CompileError::InvalidName` untested** — `src/lib.rs:1414`. No exact test assertion.

38. **`CompileError::MissingStepId` untested** — `src/lib.rs:1422`. No exact test assertion.

39. **`CompileError::DuplicateStepId` untested from end-to-end** — `src/lib.rs:1428`. Asserted in `src/control_flow/tests.rs` at AST level, but not via `compile_workflow`.

40. **`CompileError::StepShape` untested** — `src/lib.rs:1434`. No exact test assertion.

41. **`CompileError::UnknownStepField` untested** — `src/lib.rs:1440`. No exact test assertion.

42. **`CompileError::UnknownStepPrimitiveField` untested** — `src/lib.rs:1448`. No exact test assertion.

43. **`CompileError::MissingStepPrimitive` untested** — `src/lib.rs:1458`. No exact test assertion.

44. **`CompileError::MultipleStepPrimitives` untested from end-to-end** — `src/lib.rs:1464`. Asserted in `src/ast/tests.rs` at AST level, but not via `compile_workflow`.

45. **`CompileError::UnsupportedStepPrimitive` untested from end-to-end** — `src/lib.rs:1470`. Asserted in `src/compile/mod.rs` (dead code), but not via `compile_workflow`.

46. **`CompileError::UnsupportedStepControlField` untested from end-to-end** — `src/lib.rs:1478`. Asserted in `src/control_flow/tests.rs` at AST level and `src/lib.rs` via `compile_source`, but not via `compile_workflow`.

47. **`CompileError::MissingStepField` untested** — `src/lib.rs:1486`. No exact test assertion.

48. **`CompileError::StepFieldShape` untested from end-to-end** — `src/lib.rs:1494`. Asserted in `src/ast/tests.rs` at AST level, but not via `compile_workflow`.

49. **`CompileError::StepIndexOutOfRange` untested** — `src/lib.rs:1504`. No exact test assertion.

50. **`CompileError::SlotIndexOutOfRange` untested from end-to-end** — `src/lib.rs:1510`. Asserted in `src/ast/tests.rs` and `src/type_taint/tests.rs` at AST level, but not via `compile_workflow`.

51. **`CompileError::BranchTargetOutOfRange` untested from end-to-end** — `src/lib.rs:1516`. Asserted in `src/ast/tests.rs` at AST level, but not via `compile_workflow`.

52. **`CompileError::BackwardBranchTarget` untested from end-to-end** — `src/lib.rs:1522`. Asserted in `src/control_flow/tests.rs` at AST level, but not via `compile_workflow`.

53. **`CompileError::PrimitiveLoweringLimitExceeded` untested from end-to-end** — `src/lib.rs:1530`. Asserted in `src/ast/tests.rs` at AST level, but not via `compile_workflow`.

54. **`CompileError::LastStepMustFinish` untested from end-to-end** — `src/lib.rs:1542`. Asserted in `src/control_flow/tests.rs` at AST level, but not via `compile_workflow`.

55. **`CompileError::UnsupportedConstantValue` untested** — `src/lib.rs:1545`. No exact test assertion.

56. **`CompileError::ExpressionFloatOutOfRange` untested** — `src/lib.rs:1649`. Asserted in `src/expression.rs` production code but not in any `#[test]`.

57. **`CompileError::ExpressionIntegerOutOfRange` untested** — `src/lib.rs:1641`. Asserted in `src/expression.rs` production code but not in any `#[test]`.

58. **`CompileError::ExpressionUnterminatedString` untested** — `src/lib.rs:1633`. Asserted in `src/expression.rs` production code but not in any `#[test]`.

59. **`CompileError::ExpressionUnknownIdentifier` untested** — `src/lib.rs:1677`. Asserted in `src/expression.rs` production code but not in any `#[test]`.

60. **`CompileError::ExpressionLoweringUnsupported` untested from end-to-end** — `src/lib.rs:1687`. Asserted in `src/expression_bytecode.rs` tests at bytecode level, but not via `compile_workflow`.

61. **`CompileError::ExpressionHelperArity` untested from end-to-end** — `src/lib.rs:1693`. Asserted in `src/expression_bytecode.rs` tests at bytecode level, but not via `compile_workflow`.

62. **`CompileError::IdempotencyViolation` untested from end-to-end** — `src/lib.rs:1703`. Asserted in `tests/idempotency_parity.rs` via `check_idempotency_gates` helper returning `bool`, but not as exact variant assertion.

63. **Dead code: `src/compile/` directory** — `src/compile/mod.rs` and its children (`bytecode.rs`, `expression.rs`, `schema.rs`, `type_taint.rs`) are not declared in `lib.rs` (`grep -rn "mod compile" src/` returns nothing). This is 896+ lines of duplicated, unreachable code.

---

### MAJOR FINDINGS (9)

1. **Region coverage < 90% on `src/expression_bytecode.rs`** — 82.40% region coverage. Many `ExpressionHelperArity` and `ExpressionLoweringUnsupported` branches lack test exercise.

2. **Line coverage < 90% on `src/expression.rs`** — 87.61%. `ExpressionFloatOutOfRange`, `ExpressionIntegerOutOfRange`, `ExpressionUnterminatedString`, `ExpressionUnknownIdentifier` have no test assertions.

3. **Line coverage < 90% on `src/control_flow.rs`** — 89.52%. Close but below threshold.

4. **`tests/idempotency_parity.rs` uses `is_ok()` indirectly** — `compile_ok()` helper wraps `check_idempotency_gates(...).is_ok()`. While not the literal banned pattern `assert!(result.is_ok())`, it hides the exact error variant from assertion.

5. **No branch coverage collected** — `cargo llvm-cov` reports 0 branches for all files. The toolchain or build configuration is not emitting branch coverage data. Without branch coverage, the suite cannot prove it exercises conditional boundaries.

6. **Kani harnesses use `let _ = ` for result suppression** — `src/kani/vb_compile_constant.rs:123-125,140` silently discards `push_constant` results. While these are proof harnesses, not tests, the pattern is still present in the codebase.

7. **`src/compile/` dead code duplication** — `src/compile/mod.rs` duplicates `compile_workflow`, `compile_source`, `validate_canonical_compile_scope`, and all `lower_*` functions that already exist in `src/lib.rs`. This creates maintenance liability and coverage confusion.

8. **Missing end-to-end compilation tests** — The vast majority of tests exercise AST parsing, control flow, references, and type taint in isolation. Very few tests call `compile_workflow` with invalid inputs to assert exact `CompileError` variants.

9. **`SlotCompiler::push_expression`, `push_accessor` overflow paths untested** — `src/lib.rs:807-828` — `u16::try_from` overflow branches never executed.

---

### MINOR FINDINGS (4/5 threshold)

1. `src/lib.rs:4682` — `assert!(result.is_ok(), ...)` in test `plain_validate_does_not_claim_gate_12`. This is `vb_validate::shared::validate` result, not a `CompileError`, so not a banned pattern per se, but still a boolean assertion.
2. `src/lib.rs:4780` — `assert!(matches!(errors[0], CompileError::Utf8(_)), ...)` uses `matches!` instead of exact destructuring. Acceptable for opaque `str::Utf8Error` payload.
3. `src/kani/vb_compile_constant.rs:128` — `count.unwrap()` in Kani harness (not a test, but still an unwrap).
4. `src/kani/vb_compile_constant.rs:142` — `final_count.unwrap()` in Kani harness.

---

### MANDATE

Before resubmission, the following must be completed:

1. **Coverage must reach ≥90% line and ≥90% region overall** — Current: 64.95% line, 64.95% region. This requires ~2,500 additional covered lines. Priority order:
   - `src/lib.rs` (45.03% → 90%): Add end-to-end `compile_workflow` tests for every `CompileError` variant reachable from `&[u8]` input.
   - `src/ast/parse.rs` (67.10% → 90%): Add AST parsing tests for missing field shapes and trigger/step primitives.
   - `src/schema.rs` (62.21% → 90%): Add input schema validation tests.
   - `src/type_taint.rs` (61.97% → 90%): Add type mismatch and taint leak tests.

2. **Every `CompileError` variant must have an exact test assertion** — At minimum, every variant must be asserted via `matches!`, `assert_eq!`, or `match` in a `#[test]` or `#[kani::proof]`. The following categories need tests:
   - YAML strict-profile limits: `SourceTooLarge`, `DepthLimit`, `NodeLimit`, `SequenceLimit`, `MappingLimit`, `ScalarLimit`
   - YAML forbidden features: `EmptySource`, `DocumentCount`, `TopLevelNotMapping`, `NonStringKey`, `DuplicateKey`, `AliasForbidden`, `AnchorForbidden`, `MergeKeyForbidden`, `TagForbidden`, `BadValue`, `FloatForbidden`
   - YAML parse errors: `Parse`, `CanonicalYaml`
   - AST shape errors: `MissingField`, `UnknownTopLevelField`, `InvalidVersion`, `InvalidTriggerCount`, `TriggerShape`, `UnknownTriggerField`, `MissingTriggerField`, `InvalidTriggerField`, `InvalidInputSchema`, `UnknownInputSchemaField`, `UnsupportedTopLevelResult`, `UnsupportedTopLevelDeclaration`, `InvalidName`, `MissingStepId`, `StepShape`, `UnknownStepField`, `UnknownStepPrimitiveField`, `MissingStepPrimitive`, `MissingStepField`, `StepFieldShape`, `StepIndexOutOfRange`, `UnsupportedConstantValue`
   - End-to-end lowering errors: `UnsupportedStepPrimitive`, `DuplicateOutputName`, `UnknownOutputName`, `EmptySteps`, `SlotIndexOutOfRange`, `BranchTargetOutOfRange`, `PrimitiveLoweringLimitExceeded`, `LastStepMustFinish`
   - Expression errors: `ExpressionFloatOutOfRange`, `ExpressionIntegerOutOfRange`, `ExpressionUnterminatedString`, `ExpressionUnknownIdentifier`
   - Idempotency: `IdempotencyViolation` must be asserted exactly, not via `is_ok()` wrapper.

3. **Enable branch coverage in `cargo llvm-cov`** — Ensure the toolchain emits branch coverage and verify ≥90% branch coverage per file.

4. **Remove or integrate dead code in `src/compile/`** — Either delete `src/compile/` (if truly dead) or add `mod compile;` to `lib.rs` and deduplicate with `lib.rs`. Dead code skews coverage metrics and creates maintenance debt.

5. **Run `cargo mutants` to ≥90% kill rate** — With coverage at 90%+, re-run mutation testing. Every surviving mutant must have a named test written to kill it.

6. **Replace `tests/idempotency_parity.rs` `is_ok()` wrappers** — `compile_ok()` and `static_ok()` must be replaced with assertions on exact `CompileError` variants where rejection is expected.

---

### FIXES APPLIED DURING REVIEW

- **Added `compile_workflow_rejects_invalid_utf8`** at `src/lib.rs:4776` — asserts `CompileError::Utf8(_)` exactly when `compile_workflow` receives invalid UTF-8 bytes. This resolves one LETHAL finding (`Utf8` untested) and adds 1 line of covered code.

---

### RE-EVALUATION STATUS

After applying the `Utf8` test:
- Tier 0: Still FAIL (62 additional untested `CompileError` variants, dead code, `let _ = ` in Kani)
- Tier 1: PASS (270 tests, 0 flaky, ordering consistent)
- Tier 2: FAIL (64.95% line coverage, 45.03% in lib.rs)
- Tier 3: NOT COMPLETED (mutants tool timed out; insufficient coverage to expect viable kill rate)

The suite requires substantial test expansion (estimated 150–200 additional end-to-end tests) to reach APPROVED. Current state is **REJECTED with known debt**.
