# vb-8eocj.2 Source Type Identity Tests — v17

Status: tests written, NOT RUN
Workspace: /home/lewis/src/isoloated/vb-f1-contract-local3090-v1
Git root: /home/lewis/src/isoloated/vb-f1-contract-local3090-v1
JJ root: /home/lewis/src/isoloated/vb-f1-contract-local3090-v1

## Files

New test file: crates/workspace_tests/tests/type_identity.rs
Module registration: crates/workspace_tests/tests/vb_f1_canonical_scalar_do.rs (added `mod type_identity;`)
Evidence: .beads/vb-8eocj.2/source-type-tests-v17.md

## Fixture Regression Fix (v17)

V16 fixtures set `output: producer` on the `id: producer` step — the output slot name
was the same as the step ID. This masked the distinct-step-identity bug: bare
`$producer` could resolve via either path, so the fixture did not actually exercise
the compiler's step-identity reference resolution.

V17 corrects all helper fixtures:

- Every producer step now uses `output: flag` (distinct from step ID `producer`).
- Consumers still reference `$producer`, `$step.producer`, and `$steps.producer` —
  the compiler must resolve these to the step ID and forward the slot type.
- Transitive copier step retains `output: copied` (unchanged).
- Numeric producer fixtures also use `output: flag` (all numeric aliases flagged).
- Numeric finish steps now use named result `result: "result"` instead of index `1`.

The old 20 tests in the suite are unchanged. This is a fixture-only correction to the
new 7 test cases in type_identity.rs.

## Test Cases (7 total)

### Negative compile error tests (5)

All expect `Err(errors)` from `vb_compile::compile_workflow(yaml.as_bytes())` with an exact
`CompileError::TypeMismatch { field: "set.value", expected: "number", found: "boolean" }` variant
found by iterating `errors.0.iter()`. No string contains laundering.

1. test_bool_producer_step_id_producer — boolean producer, consumer arithmetic via bare alias `$producer + 2`
2. test_step_prefix_bool_producer — consumer arithmetic via `$step.producer + 2`
3. test_steps_prefix_bool_producer — consumer arithmetic via `$steps.producer + 2`
4. test_transitive_copy_bool_arithmetic — transitive copy of bool via `$step.producer` to `copied`, then `$copied + 2`
5. test_direct_literal_bool_arithmetic — direct boolean literal `true + 2` in set.value

### Positive runtime tests (2)

Both compile and run via `F1SourceRun::new` then `assert_finished_with_value` expecting `SlotValue::I64(12)`.

6. test_numeric_producer_alias — numeric producer value 10, consumer `$producer + 2` → 12
7. test_numeric_qualified_step_reference — numeric producer value 10, consumer `$step.producer + 2` → 12

## Design Notes

- Helper functions use `format!` to preserve literal `$` references; Set values are quoted with `"..."` matching existing test fixtures.
- Source names use underscores (producer, consumer, copied, finish) — no hyphens.
- `output: result` / `finish: { result: ... }` naming used, compatible with the bounded F1 compiler.
- No production edits, no Cargo/moon gates invoked. Tests expected to fail until root merges the new typing system.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, or arithmetic.
- File is 198 lines, under the 250-line budget.
