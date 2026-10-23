# VB-8EOCJ2 Scalar Type Test Review (Local RTX5090 v20)

Workspace: /home/lewis/src/isoloated/vb-8eocj2-scalar-test-review-local5090-v19
Reviewer: Local RTX5090 (functions.exec_command reads/writes, no commit/push)
Prior: v19 raw review preserved unchanged; this is separate corrective review.
Status: REVIEW INCOMPLETE / changes requested — material factual errors identified.

## Verified Test Inventory (actual counts from files)

crates/workspace_tests/tests/positive.rs: 8 tests (81 lines)
  test_i64_set_unchanged_to_finish
  test_bool_set_to_finish
  test_null_set_to_finish
  test_f64_set_to_finish
  test_arithmetic_through_prior_output_alias
  test_step_reference_by_id
  test_step_reference_qualified
  test_step_reference_steps_prefix

crates/workspace_tests/tests/negative.rs: 11 tests (216 lines)
  test_duplicate_output_name_error
  test_undefined_name_reference_error
  test_forward_reference_error
  test_self_reference_error
  test_ambiguous_bare_name_error
  test_step_prefix_disambiguates_to_step
  test_invalid_action_id_negative
  test_invalid_action_id_too_large
  test_invalid_input_slot_negative
  test_finish_expression_not_supported
  test_scalar_type_error_bool_plus_i64

crates/workspace_tests/tests/do_action.rs: 1 test (250 lines)
  test_do_action_suspension_and_completion

Total: 20 #[test] functions. No invented names. Line lengths verified by wc -l.

## Material Factual Errors in v19

1. "Typed Error Assertions NOT String Contains" — WRONG.
   negative.rs lines 103, 205-209 use .contains() for error classification:
   - feature.contains("ambiguous") || feature.contains("Ambiguous")
   - msg.contains("type") || msg.contains("Type") || ...
   Qualification via contains() is a POSITIVE runtime test, not a typed error check.
   The "not string contains" claim is contradicted by actual source.

2. "Forward fixture $future while later step id later_step/output result_y proves
   undefined reference, NOT true forward reference to eventual actual binding."
   The test uses a name ("future") never defined anywhere. A true forward reference
   test would reference result_y from an earlier step where result_y is defined later.
   This proves unknown-name detection, not forward-reference semantics.

3. "Mutation resistance verified PASS" — WRONG. Reviewer did not independently
   execute mutation probes. Mutation resistance is static inference only.

4. "Cargo autodiscovery may recognize files..." — Overly cautious. Root cargo
   metadata CONFIRMS test target registration. The 'may' phrasing understates
   verified build-system state.

5. "Import breaks proper module move" — WRONG. Root .rs helper files auto-target
   regardless of #[test]; module relocation subdirectory with #[path] correctly
   retains crate:: support.

6. "7 newtype fixture coverage claim" — Invented. No such claim in actual tests.

7. Actual Do input 10/output 999 real journal verified by actual source lines:
   - Line 13: value "10" in workflow YAML
   - Line 141: action_output(SlotValue::I64(999))
   - Line 194: decoded != SlotValue::I64(999) verification
   This is genuine end-to-end slot value verification.

## Findings vs Acceptance

While the test suite contains 20 real tests with proper names and the do_action
journal verification is genuine, the review cannot grant full TEST acceptance
while material factual errors remain unresolved. The v19 errors — particularly
the typed-error vs contains() confusion and the undefined-reference vs forward-
reference conflation — undermine the accuracy of the prior acceptance rationale.

## Bounded Corrective Needs

- Correct the error-assertion classification: tests use contains() for
  disambiguation, not typed error variants.
- Rename or restructure the forward-reference test to actually test a reference
  to a later-defined binding, or rename it to "unknown_name_error".
- Remove "verified mutation resistance" claim unless mutation probes are
  actually executed and their results included.
- Tighten cargo autodiscovery language from "may" to "confirmed" based on
  root cargo metadata.
- Remove all invented claims (7-newtype coverage, import-breaks-module-move).

This review is bounded to the stated corrective needs. No source/test/gate
changes were made. Reviewer preserved v19 raw report.
