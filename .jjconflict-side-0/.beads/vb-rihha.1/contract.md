# Contract: Reject known scalar operands for collection helpers

## Scope
This contract is limited to static operand validation for the existing `length`, `count`, and `empty` expression helpers; it does not claim whole-IR type correctness or change any other helper.

## Domain behavior
For each of `length`, `count`, and `empty`, a statically known Bool, I64, or F64 argument is rejected as a typed mismatch; the expected type label is `list`, and `found` is exactly `boolean`, `i64`, or `f64` according to the existing private expression type-name mapping.

Null and List arguments remain accepted for all three helpers; an Unknown argument remains deferred and accepted without false precision.

Existing Text and Object behavior is preserved without new rejection rules; their final accepted domain remains an open follow-up because master Section 46's signature table conflicts with its store-aware implementation-status note.

Successful helper result types remain `ExprType::I64` for `length` and `count`, and `ExprType::Bool` for `empty`.

No behavior changes are specified for `contains`, other helpers, expression operators, slot assignment, control flow, compiler gates, schemas, or public APIs.

## Public API and errors
The direct public expression API is `vb_expr::expr_typecheck::typecheck_expr(&ExprAst, &TypeContext) -> ExprResult<ExprType>`; invalid scalar operands return `vb_expr::ExprError::TypeMismatch { expected: "list", found }`.

The public compiler reproducer is `vb_compile::compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, CompileErrors>` using canonical Set `value` expressions such as `length(1)`; each invalid case returns `CompileErrors` containing `CompileError::TypeMismatch { field: "set.value", expected: "list", found }`.

For each helper, the public compiler rejection matrix is: argument `1` → `found: "i64"`; argument `true` → `found: "boolean"`; argument `1.0` → `found: "f64"`.

Direct expression-typecheck acceptance cases cover each helper with `null`, with a `$items` reference typed as List, and with a `$deferred` reference typed Unknown; assert the helper's existing result type in every accepted case.

Text and Object cases are not assigned new expected outcomes by this contract; preserve their existing behavior pending resolution of the master conflict.

## Compiler-path constraint
Use a helper call as the complete Set expression (`value: "length(1)"`, and corresponding calls for the other cases), not a bare scalar Set value; the inspected compiler parses the helper expression, infers its type, and only then emits bytecode, while the direct-I64 constant fast path applies only when the entire Set value parses as an integer.

## Existing test baseline and evidence
Existing `vb_expr::expr_typecheck::tests::helper_argument_types` covers rejected wrong types for other helpers and deferred Unknown cases for these three, but has no known Bool/I64/F64 rejection cases for `length`, `count`, or `empty`.

Existing public compiler fixtures call `vb_compile::compile_workflow(source.as_bytes())` and match `CompileError::TypeMismatch`; add the nine helper scalar cases there without weakening or replacing existing cases.

The repository CI test task is `moon run :test`; `.moon/tasks/all.yml` defines it to run pinned nightly Nextest with `--cargo-quiet --workspace --all-features`. This contract-writing task runs no tests or gates.
