# vb-eirmk.7.2.4 — Reduce arithmetic failures surface errors with correct PC

## Goal

Prove that two compiled SOURCE reduce body arithmetic failures — integer overflow from a near-max accumulator plus one, and integer division by zero — produce the expected public errors on the live engine (EngineError variants) and on the replay engine (ReplayError::ExpressionEvalFailed with the actual body expression StepIdx). The actual failing body expression StepIdx is derived from the compiled ReduceStart node's body field, never guessed. Both live and replay paths assert the captured frame PC equals the derived body expression StepIdx after the error.

## Scope

Write only a new test file:
`crates/workspace_tests/tests/vb_eirmk_reduce_arithmetic_failures.rs`.

## Scenarios

### Scenario 1: reduce body integer overflow, live path

**Given** a compiled SOURCE reduce with variable `total`, initial `9223372036854775807` (i64::MAX), input singleton list `[4]`, and body expression `$total + 1`.

**When** driven live with a list bound to slot 8 with DerivedFromSecret taint.

**Then** live execution returns `EngineError::InvalidCompiledWorkflow { reason: "integer arithmetic overflow" }`, the captured frame PC equals the derived body expression StepIdx, and the failing node is an EvalExpr.

### Scenario 2: reduce body integer division by zero, live path

**Given** a compiled SOURCE reduce with variable `total`, initial `10`, input singleton list `[4]`, and body expression `$total / 0`.

**When** driven live with a list bound to slot 8 with DerivedFromSecret taint.

**Then** live execution returns `EngineError::DivisionByZero`, the captured frame PC equals the derived body expression StepIdx, and the failing node is an EvalExpr.

### Scenario 3: reduce body integer overflow, replay path

**Given** the same compiled SOURCE reduce and the actual body expression StepIdx derived from the compiled workflow.

**When** replayed step-by-step for a single reduce iteration using the same list bound to slot 8 with DerivedFromSecret taint.

**Then** replay fails with `ReplayError::ExpressionEvalFailed { step: actual_body_expr_step }` using the exact actual body expression StepIdx, and the captured frame PC equals the derived body expression StepIdx.

### Scenario 4: reduce body integer division by zero, replay path

**Given** the same compiled SOURCE reduce and the actual body expression StepIdx.

**When** replayed step-by-step for a single reduce iteration using the same list bound to slot 8 with DerivedFromSecret taint.

**Then** replay fails with `ReplayError::ExpressionEvalFailed { step: actual_body_expr_step }` using the exact actual body expression StepIdx, and the captured frame PC equals the derived body expression StepIdx.

### Scenario 5: author slot ListId, contents, and taint preservation

**Given** the reduce input singleton list `[4]` bound to slot 8 with DerivedFromSecret taint on both the live and replay paths.

**When** the arithmetic failure occurs on either path.

**Then** the frame must still read the original ListId on slot 8, the ValueStore must still return the exact original list contents `[4]` from that ListId, and the slot taint must still be DerivedFromSecret. The author list is preserved untouched across both failure paths.

### Scenario 6: compiled IR preserved after failure

**Given** the original compiled workflow IR bytes.

**When** the arithmetic failure occurs on either path.

**Then** the compiled IR after failure matches the original IR bytes. No mutation of the compiled workflow.

## Registered tests

Two `#[test]` functions register four live/replay paths:
- `reduce_overflow_live_and_replay_error`
- `reduce_divide_by_zero_live_and_replay_error`

Each test derives the actual body expression StepIdx once, asserts the failing node is EvalExpr, runs both live and replay paths, and asserts the captured PC equals the derived StepIdx after each error.

## Non-goals

No production code edits, no config/gate/API changes, no new dependencies. No mutation of sibling tests or other crates. Only the new test file and this contract.

## Gates

The test must compile and pass under the pinned nightly-2026-04-28 toolchain with the exact cargo test command specified by the bead.
