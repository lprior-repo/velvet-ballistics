# Has parity contract clarification V5

Status: additive contract clarification only; no implementation, tests, tracker mutation, proof work, or public API change is authorized.

Base contract: `/home/lewis/src/isoloated/velvet-ballistics-masterplan-locator-luna/.beads/vb-eirmk.42/has-parity-contract-proposal-v3.md`, SHA-256 `2f3329582a76616ad9b02e83ea0b5036a66c616fa5b49e39afcd71b33339d860`.

Source boundary references below use the audited source pin `ff42abf7bd9f778d388274fb9a0c01d081715bb7`; V3's own source pin and all V3 ownership/status clauses remain unchanged.

## Additive clarification: malformed bytecode boundary

`ExprOp::Has` consumes two stack values and pushes one; it carries no source argument count. “0/1/3 arguments” in V3 bytecode clauses means respectively zero or one stack values before `Has`, or three values before `Has` (the latter leaves two values after the operation). It does not mean the slice-based `eval_helper_with_store` API.

For public `vb_expr` evaluator coverage, construct the real public `ExprProgram { ops, max_stack }` with a struct literal and call `eval_expr_program` or `eval_expr_program_with_store`. These are malformed-program diagnostic inputs: 0 and 1 stack values at `Has` return `ExprError::StackUnderflow`; 3 values before `Has` let the operation consume its top two, then whole-program finalization returns `ExprError::StackOverflow { max: MAX_EXPRESSION_STACK }`. The three-value fixture must use valid values for the consumed Object and Symbol operands and retain a lower stack value. The public validating `ExprProgram::try_from_ops`/`try_from_parts` path continues rejecting these programs: 0/1 with `CoreError::ExpressionStackUnderflow`, 3 with `CoreError::InvalidCompiledWorkflow { reason: "expression leaves non-single result" }` (`vb_core/src/workflow/mod.rs:884-908,976-986,2712-2721`). Do not weaken those constructors or substitute direct-helper arity tests for bytecode execution.

For Hot and Replay, malformed bytecode is rejected by the normal public workflow boundary before evaluation. `CompiledWorkflow::try_from_parts` returns `WorkflowError::Expression(CoreError::ExpressionStackUnderflow)` for 0/1, and `WorkflowError::Expression(CoreError::InvalidCompiledWorkflow { reason: "expression leaves non-single result" })` for 3 (`vb_core/src/workflow/mod.rs:42-60,712-714,2046-2053`). Public `eval_expr_with_store` and `ReplayEngine::replay_frame_through` acceptance must not promise these malformed programs reach their evaluators through a validated workflow.

Hot runtime malformed-program assertions are mandatory internal diagnostic tests: use the existing `test-util` `CompiledWorkflow::from_parts_unchecked` path and call public `eval_expr_with_store`. Expected results are `EngineError::ExpressionStackUnderflow` for 0/1 and `EngineError::InvalidCompiledWorkflow { reason: "expression leaves non-single result" }` for 3. They supplement, and do not replace, the separate public-constructor rejection tests.

Replay whole-expression malformed-program assertions are mandatory internal diagnostic tests: use the same sanctioned unchecked test construction and public `ReplayEngine::replay_frame_through` with a nonzero current PC. All three whole-expression failures are `ReplayError::ExpressionEvalFailed { step: that_pc }`. The separate public-constructor rejection tests remain mandatory. A direct `eval_replay_op` diagnostic is optional additional coverage only: 0/1 fails at the current PC, while 3 returns `Ok(())` and leaves stack length two; only whole-expression finalization reports the non-single result. Do not substitute the single-op test for whole-expression coverage or claim it checks final stack depth or exposes internal operand precedence.

## V3 clause disposition

Preserved without change: status and source pin; existing-owner deduplication; normative `has(Object, Symbol) -> Bool` membership and invalid-handle semantics; separate unresolved `contains`; all four Boolean cases; evaluator precedence and typed/handle errors; typechecker, parser, `parse_ast`, canonical compile, Any/type-taint, and exact field/error payload cases; existing test identities; bounded production surface; qualification-only ownership and no-change boundaries.

Clarified only: V3 “malformed bytecode” clauses under “Exact evaluator error, precedence, and arity contract” now mean the entry paths and diagnostic limits above. Keep V3's exact direct `vb_expr` runtime errors, mandatory Hot/Replay unchecked whole-expression diagnostic runtime errors, and Replay current-PC requirement, but label Hot/Replay unchecked execution as internal diagnostics and separately qualify normal constructor rejection. Keep V3's direct helper arity contract (`HelperArityMismatch { helper: "has", expected: 2, actual }`) separate and unchanged. The 3-value case remains a final-depth error after Has consumes two operands, not a Has arity error.

## Acceptance addendum

Retain V3's existing parity and regression acceptance in full. Add exact tests for the public validated-constructor rejections above and the raw-public-`ExprProgram` `vb_expr` diagnostic path. Require both Hot and Replay internal whole-expression malformed-program diagnostic suites using existing test utilities, including nonzero-PC Replay assertions; these do not imply reachability through validated public workflows. A direct single-op Replay diagnostic may be additional coverage only. Preserve the distinction between whole-expression finalization and single-op execution. No production behavior, API, feature wiring, contract ownership, or other V3 acceptance is removed or relaxed.
