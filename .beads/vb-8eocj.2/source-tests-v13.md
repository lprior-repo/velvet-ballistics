# F1 Bounded Slice Source Test Report (v13)

**Bead:** vb-8eocj.2 (bounded first F1 slice)
**Parent:** F1 (full canonical scalar + Do action language)
**Author:** local RTX3090 Codex CLI agent
**Date:** 2026-10-22
**Workspace:** /home/lewis/src/isoloated/vb-f1-contract-local3090-v1
**Status:** Tests executed. 19 passed, 1 failed (bool+i64 preserved).

## Test Execution Summary

```bash
env RUSTFLAGS=-Dwarnings RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 cargo test -p velvet-ballistics-workspace-tests --test vb_f1_canonical_scalar_do -- --nocapture
```

**Exit code:** 101 (test failure, as expected for bool+i64)

**Result:** 19 passed; 1 failed; 0 ignored; 0 measured

## Passed Tests (19)

1. test_i64_set_unchanged_to_finish
2. test_bool_set_to_finish
3. test_null_set_to_finish
4. test_f64_set_to_finish
5. test_arithmetic_through_prior_output_alias
6. test_step_reference_by_id
7. test_step_reference_qualified
8. test_step_reference_steps_prefix
9. test_do_action_suspension_and_completion
10. test_duplicate_output_name_error
11. test_undefined_name_reference_error
12. test_forward_reference_error
13. test_self_reference_error
14. test_ambiguous_bare_name_error
15. test_step_prefix_disambiguates_to_step
16. test_invalid_action_id_negative
17. test_invalid_action_id_too_large
18. test_invalid_input_slot_negative
19. test_finish_expression_not_supported

## Failed Test (1 — preserved)

1. test_scalar_type_error_bool_plus_i64
   - Error: "expected compile error for bool + i64, got success"
   - This test expects the compiler to reject incompatible type operations (bool + i64),
     but the current production compiler accepts it. This is a known gap in production
     that must be repaired before this test can pass. The test is intentionally preserved
     as failing to document this gap.

## Review Defects Fixed (v12 -> v13)

1. **Unwrap removal:** Removed all `unwrap()` calls from both test files.
   - `f1_action_contract` now returns `Result<ActionContract, String>`
   - `f1_contracts_through` now returns `Result<Box<[ActionContract]>, String>`
   - `action_output` now returns `Result<ActionOutputReady, String>`
   - Used `.map_err().unwrap()` pattern replaced with proper `?` propagation

2. **Dense contract loop replaced:** Replaced saturating while loop with
   `(0..=target.get()).map().collect::<Result<Vec<_>, _>>().?` for bounded
   contract generation.

3. **Ambiguity oracle strengthened:** Negative typed tests now call
   `vb_compile::compile_workflow` directly and match actual `CompileErrors`
   contents/`CompileError` variants:
   - `test_duplicate_output_name_error`: matches `CompileError::DuplicateOutputName { name: "x" }`
   - `test_undefined_name_reference_error`: matches `CompileError::UnknownOutputName { name: "missing" }`
   - `test_forward_reference_error`: matches `CompileError::UnknownReferenceName { name: "future", reference: "$future" }`
   - `test_self_reference_error`: matches `CompileError::UnknownReferenceName { name: "x", reference: "$x" }`
   - `test_ambiguous_bare_name_error`: matches `CompileError::ExpressionLoweringUnsupported { feature }` where feature contains "ambiguous"
   - `test_invalid_action_id_negative`: matches `CompileError::StepFieldShape { field: "action" }`
   - `test_invalid_action_id_too_large`: matches `CompileError::StepFieldShape { field: "action" }`
   - `test_invalid_input_slot_negative`: matches `CompileError::SlotIndexOutOfRange { value: -1 }`

4. **Forward reference fixture corrected:** The forward reference test now
   actually defines `future` (renamed to `later_step` to avoid YAML 1.1
   ambiguity) as a later step, so the error is truly a forward reference
   rather than a non-existent reference.

5. **Disambiguation test added:** New test `test_step_prefix_disambiguates_to_step`
   creates a fixture with a step ID `conflict` and a distinct output name `conflict`,
   then uses `$step.conflict` to resolve the ambiguity to the step's primary output
   and asserts the exact value (41 vs 43).

6. **Do test strengthened:** The `test_do_action_suspension_and_completion` test
   now:
   - Submits with empty input assignments (source Set step owns input_val)
   - Uses `runtime.snapshot_run` actual API to verify suspension at step 1 (Do)
   - Verifies run is no longer active after completion via snapshot
   - Inspects actual `InspectResponse` types (Found/NotFound variants)
   - Preserves ticket inspection for action ID and step
   - Notes: InspectSnapshot only exposes pc and executed, not slot values.
     Value evidence comes from the action completion flow and run completion counters.

## Raw Test Output

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.57s
     Running tests/vb_f1_canonical_scalar_do.rs (target/debug/deps/vb_f1_canonical_scalar_do-ddd4184b48b00c6d)

running 20 tests
test test_duplicate_output_name_error ... ok
test test_arithmetic_through_prior_output_alias ... ok
test test_bool_set_to_finish ... ok
test test_f64_set_to_finish ... ok
test test_do_action_suspension_and_completion ... ok
test test_i64_set_unchanged_to_finish ... ok
Error: "expected compile error for bool + i64, got success"
test test_invalid_action_id_too_large ... ok
test test_scalar_type_error_bool_plus_i64 ... FAILED
test test_step_reference_steps_prefix ... ok
test test_step_prefix_disambiguates_to_step ... ok
test test_invalid_input_slot_negative ... ok
test test_step_reference_qualified ... ok
test test_undefined_name_reference_error ... ok
test test_step_reference_by_id ... ok
test test_self_reference_error ... ok
test test_finish_expression_not_supported ... ok
test test_ambiguous_bare_name_error ... ok
test test_invalid_action_id_negative ... ok
test test_null_set_to_finish ... ok
test test_forward_reference_error ... ok

failures:

failures:
    test_scalar_type_error_bool_plus_i64

test result: FAILED. 19 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p velvet-ballistics-workspace-tests --test vb_f1_canonical_scalar_do`
```

## Files Modified

1. `crates/workspace_tests/tests/vb_f1_canonical_scalar_do.rs` — 20 test cases, all review defects fixed
2. `crates/workspace_tests/tests/vb_f1_scalar_do_support/mod.rs` — Unwrap removal, proper error propagation

## Notes

- The bool+i64 type error test remains failing as required. This documents the
  production gap where incompatible type operations are not caught at compile time.
- All error matching uses the actual `CompileError` enum variants from
  `vb_compile::errors::kind::CompileError`.
- The snapshot_run API returns `InspectSnapshot` with `pc` (program counter) and
  `executed` (transition count) but not slot values. Slot value evidence in the
  Do test comes from the action completion flow and run completion counters.
