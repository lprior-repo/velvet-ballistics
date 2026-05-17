# Test Repair Guide: vb-f04l State 9 Rejection

## Route

- Current gate: State 9 test-reviewer.
- Result: rejected suite, approved plan.
- Exact route: return to State 8 test-writer attempt 2.
- Do not route to State 7 unless State 8 proves a contract/test-plan clause is impossible to test through available public APIs; if so, stop and route to State 7 plan/contract clarification before writing replacement tests.
- Do not enter State 10 implementation until State 9 approves both `test-plan-review.md` and `test-suite-review.md`.

## Required State 8 Repair

1. Keep all work inside `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`.
2. Do not edit production implementation code.
3. Add executable tests for all contracted public paths:
   - `compile_source(&WorkflowSource)` canonical AST path.
   - `compile_workflow(&[u8])` YAML bytes path.
   - `YamlCompiler::compile(&[u8])` parser/admission mapping path.
4. Add exact error-variant tests for every contract error in `contract.md` lines 78-88:
   - `EmptySteps`.
   - `UnsupportedTopLevelDeclaration`.
   - `UnsupportedTopLevelResult`.
   - `UnsupportedStepControlField`.
   - `DuplicateStepId`.
   - `DuplicateOutputName`.
   - `UnknownOutputName`.
   - `StepFieldShape`.
   - `StepIndexOutOfRange`.
   - `SlotIndexOutOfRange`.
   - `PrimitiveLoweringLimitExceeded`.
   - `Workflow` with preserved underlying `WorkflowError`.
   - `CanonicalYaml`.
   - `UnsupportedStepPrimitive` for exactly `Save`, `Do`, and `Choose`, or route to State 7 if `Save` is contractually untestable.
5. Strengthen primitive positive assertions:
   - Exact body/done/join/resume/exhausted target indexes.
   - Exact join count for `Together`.
   - Exact max attempts and exhaustion route for `Repeat`.
   - Exact collector/accumulator/prompt/answer/timeout slot values.
   - Exact `slot_count`, not `slot_count >= minimum`.
6. Add Set/Finish regression tests for duplicate output and unknown output behavior.
7. Identify or add the bead-specific `YamlCompiler::compile` fuzz target/corpus evidence required by the approved plan.
8. Update `test-writer-report.md` with raw command evidence for compile, focused red run, property run, and fuzz compile/target coverage.

## Resubmission Rule

After repair, rerun State 9 from the beginning. Do not ask the reviewer to inspect only the delta.
