# F1 Scalar Do Source Tests - v14

## Execution Result

```
running 20 tests
test negative::test_invalid_action_id_too_large ... ok
test negative::test_invalid_action_id_negative ... ok
test negative::test_duplicate_output_name_error ... ok
test negative::test_invalid_input_slot_negative ... ok
test negative::test_self_reference_error ... ok
test negative::test_forward_reference_error ... ok
test negative::test_finish_expression_not_supported ... ok
test negative::test_ambiguous_bare_name_error ... ok
test negative::test_scalar_type_error_bool_plus_i64 ... FAILED
test negative::test_undefined_name_reference_error ... ok
test positive::test_i64_set_unchanged_to_finish ... ok
test positive::test_arithmetic_through_prior_output_alias ... ok
test positive::test_f64_set_to_finish ... ok
test positive::test_bool_set_to_finish ... ok
test positive::test_step_reference_qualified ... ok
test positive::test_null_set_to_finish ... ok
test negative::test_step_prefix_disambiguates_to_step ... ok
test positive::test_step_reference_steps_prefix ... ok
test positive::test_step_reference_by_id ... ok
test do_action::test_do_action_suspension_and_completion ... ok

test result: FAILED. 19 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

## Known Failure

`negative::test_scalar_type_error_bool_plus_i64` remains the single known failure,
consistent with the original test suite. This is the bool + i64 type inference
issue that requires the 5090 source fix to resolve. Not weakened.

## Do Test Value Evidence

The Do action test (`do_action::test_do_action_suspension_and_completion`) now
proves actual slot values through journal events rather than counters:

1. **Compiled IR verification**: Asserts Do node at step 1 has input slot 0
   (the Set step's output slot) before running the workflow.

2. **Input slot value evidence**: Verifies `SlotWritten` journal event for
   slot 0 contains postcard-encoded `I64(10)` - the Set step's actual output
   value. Decodes and compares the raw bytes.

3. **Action completion output evidence**: Verifies `ActionCompletedEnvelope`
   journal event for output slot 1 contains postcard-encoded `I64(999)` - the
   actual value returned by the completed action. Decodes and compares the
   raw bytes.

4. **RunFinished result slot evidence**: Verifies `RunFinished` journal event
   records result slot 1, confirming the terminal finish node received the
   action's output.

All value assertions use actual enum field inspection of journal events, not
counters or NotFound checks.
