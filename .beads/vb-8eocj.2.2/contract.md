# vb-8eocj.2.2: Named Reference Resolution Order Tests

## Objective

Verify that the compiler resolves named reference errors (forward and self)
BEFORE arithmetic type errors. When a name is not in scope, the error must
be `CompileError::UnknownReferenceName`, not `CompileError::TypeMismatch`.

## Scope

- New standalone test file: `tests/vb_f1_named_reference_order.rs`
- Uses existing `vb_f1_scalar_do_support` helper module (no changes)
- No production code changes
- No changes to existing tests

## Test Cases

### 1. Forward Alias Reference (output kind)
Consumer references `$future` before step `future_id` defines output `future`.
- Expect: `CompileError::UnknownReferenceName`
- `kind: "output"`, `reference: "$future"`, `name: "future"`
- Must NOT be `CompileError::TypeMismatch`

### 2. Forward Explicit Step Reference (step kind)
Consumer references `$step.future_id` before step `future_id` is defined.
- Expect: `CompileError::UnknownReferenceName`
- `kind: "step"`, `reference: "$step.future_id"`, `name: "future_id"`
- Must NOT be `CompileError::TypeMismatch`

### 3. Self Alias Reference (output kind)
Step defines output `self_value` with expression `$self_value + true`.
- Expect: `CompileError::UnknownReferenceName`
- `kind: "output"`, `reference: "$self_value"`, `name: "self_value"`
- Must NOT be `CompileError::TypeMismatch`

### 4. Positive: Producer Before Consumer
Producer step `producer` sets output `future` to 10.
Consumer step references `$future + 2`.
- Expect: Success, finished with value 12
- Verifies name resolution succeeds when producer comes first

## Evidence

- `.beads/vb-8eocj.2.2/source-name-tests-local3090-v1.md` (4 test cases, not run)
- `tests/vb_f1_named_reference_order.rs` (4 tests, run on local RTX3090)
