# vb-8eocj.3 Test Review — Frozen19 Suite (local3090 independent)

**Scope:** crates/workspace_tests/tests/vb_f2_source_slot_preservation.rs
**Frozen revision:** 34fd304e7214ced5527d28768bb98aa240045b0b
**Test file SHA256:** 49bcff41f94eaed46dab343ea2b771e7b5aa70cb2151b2f91fc99792915d708e (verified)
**Evidence run:** .beads/vb-8eocj.3/native-integration-v10/ROOT_F2_OWNED_REENTRY_V10_NATIVE (exit=101, elapsed=5.520s)
**Reviewer:** local3090 independent test reviewer

## Measured Results from Raw Logs

- **Tests run:** 19 (compiled, executed, no ignore/filtered)
- **Passed:** 12
- **Failed:** 7
- **Exit code:** 101 (correctly indicates test failures)

### Failed tests (V10, 5.520s run)
1. `declared_scalar_list_object_inputs_survive_repeat` — Core(InternalInvariantViolation { reason: "invalid_state_transition" })
2. `reduce_empty_list_preserves_source_values` — EvidenceGapsRecorded { count: 1, last_slot: Some(SlotIdx(11)) }
3. `reduce_single_item_preserves_source_values` — EvidenceGapsRecorded { count: 1, last_slot: Some(SlotIdx(10)) }
4. `repeat_finishes_with_prior_answer_42` — Core(InternalInvariantViolation { reason: "invalid_state_transition" })
5. `repeat_finishes_with_prior_sentinel_99` — Core(InternalInvariantViolation { reason: "invalid_state_transition" })
6. `sequential_repeat_together_preserve_prior_source_values` — Core(InternalInvariantViolation { reason: "invalid_state_transition" })
7. `sequential_together_repeat_preserve_prior_source_values` — Core(InternalInvariantViolation { reason: "invalid_state_transition" })

### Passed tests (12)
- together_entry_preserves_prior_source_values
- together_finishes_with_prior_answer_42
- together_finishes_with_prior_sentinel_99
- for_each_empty_list_preserves_source_values
- for_each_single_item_preserves_source_values
- for_each_two_items_preserve_source_values
- generated_composite_roles_do_not_alias_source_outputs
- numeric_source_for_each_preserves_42_99_and_list_taint
- numeric_source_reduce_preserves_42_99_and_list_taint
- reduce_compiles_with_four_distinct_private_roles_and_no_nop
- reduce_two_items_preserve_source_values
- declared_scalar_list_object_inputs_survive_together

## Test Suite Design Assessment

### Strengths
1. **Uses compile_workflow on real source:** Every workflow is compiled from velvet-ballistics/v1 source, not handbuilt IR. Public source execution uses unchanged IR as required.
2. **Concrete sentinel assertions:** Tests assert specific values (42, 99) rather than generic `is_ok()`/`is_err()`.
3. **Body execution count verification:** `assert_body_counts` uses EvidenceEvent::StepSucceeded to verify exact body execution counts, not just terminal values.
4. **Taint preservation checks:** Multiple tests verify Clean/DerivedFromSecret/Secret taints are preserved.
5. **Slot isolation verification:** `assert_role_isolation` and `implicit_state_writes` check that generated composite slots do not alias protected source slots.
6. **IR stability check:** `assert_eq!(run.workflow.to_parts(), original)` verifies compilation artifacts are not mutated during execution.

### Exact test enumeration (19 tests)
| # | Test name | Composite | List size | Finish target | Status |
|---|-----------|-----------|-----------|---------------|--------|
| 1 | together_entry_preserves_prior_source_values | Together | - | entry | PASS |
| 2 | together_finishes_with_prior_answer_42 | Together | - | answer(42) | PASS |
| 3 | together_finishes_with_prior_sentinel_99 | Together | - | sentinel(99) | PASS |
| 4 | repeat_finishes_with_prior_answer_42 | Repeat(max=3) | - | answer(42) | FAIL |
| 5 | repeat_finishes_with_prior_sentinel_99 | Repeat(max=3) | - | sentinel(99) | FAIL |
| 6 | sequential_together_repeat_preserve_prior_source_values | Together→Repeat | - | answer(42) | FAIL |
| 7 | sequential_repeat_together_preserve_prior_source_values | Repeat→Together | - | sentinel(99) | FAIL |
| 8 | generated_composite_roles_do_not_alias_source_outputs | Together,Repeat | - | compile | PASS |
| 9 | numeric_source_for_each_preserves_42_99_and_list_taint | ForEach | 3 | answer(42) | PASS |
| 10 | numeric_source_reduce_preserves_42_99_and_list_taint | Reduce | 3 | answer(42) | PASS |
| 11 | for_each_empty_list_preserves_source_values | ForEach | 0 | answer(42) | PASS |
| 12 | for_each_single_item_preserves_source_values | ForEach | 1 | answer(42) | PASS |
| 13 | for_each_two_items_preserve_source_values | ForEach | 2 | answer(42) | PASS |
| 14 | reduce_empty_list_preserves_source_values | Reduce | 0 | answer(42) | FAIL |
| 15 | reduce_single_item_preserves_source_values | Reduce | 1 | answer(42) | FAIL |
| 16 | reduce_two_items_preserve_source_values | Reduce | 2 | answer(42) | PASS |
| 17 | reduce_compiles_with_four_distinct_private_roles_and_no_nop | Reduce | - | compile | PASS |
| 18 | declared_scalar_list_object_inputs_survive_together | Together | - | $input.payload | PASS |
| 19 | declared_scalar_list_object_inputs_survive_repeat | Repeat(max=3) | - | $input.payload | FAIL |

## Missing Coverage for F1 Parent Acceptance

The frozen19 suite provides strong bounded-fixture coverage for Together and ForEach, but the following coverage gaps remain for full F1 parent acceptance:

### Repeat coverage gaps
- **Repeat max_attempts=1:** Only max_attempts=3 is tested. The minimal repeat case (single iteration) is not covered.
- **Repeat body execution count 1:** No test verifies a body executed exactly once.
- **Counter slot journal updates:** No test verifies that Repeat's counter slot is written to the journal/evidence after each iteration, only that the terminal count is correct.
- **Repeat suspension/rearm:** No test drives Repeat partway, suspends, and resumes. All Repeat tests drive to completion in one call.

### Reduce coverage gaps
- **Reduce empty/singleton actual write evidence:** The two failing Reduce tests (empty, singleton) fail with `EvidenceGapsRecorded`, indicating the tests expect Reduce to write evidence for empty/single-item cases but the implementation does not. This is a test-oracle vs implementation mismatch — the test is correct to expect it, but the implementation is incomplete.
- **Reduce accumulator journal updates:** No test verifies accumulator slot writes between iterations.

### Control flow edge coverage (all composites)
- **Wrong nearest owner:** No test verifies error when a body step references an incorrect nearest loop owner.
- **Wrong counter slot:** No test verifies error when body writes to a different counter than expected.
- **Wrong done slot:** No test verifies error when body writes to a different done slot.
- **Wrong body slot:** No test verifies error when body writes to a different body slot.
- **Self-edge:** No test for a control edge from a node to itself.
- **Crossing nested owner:** No test for control edges that cross nested loop boundaries.
- **Unrelated backward edge:** No test for control edges that jump backward to an unrelated (non-loop) target.

### Handbuilt IR negative tests
- The suite does not include any handbuilt-IR negative validation tests for the validator. All tests use compile_workflow and expect success. Per user specification, handbuilt IR tests are allowed for validator probe only and are not required for this suite's acceptance.

## Assertion Strength Analysis

### Concrete vs generic
- All sentinel checks use specific values (42, 99) — strong.
- Body count assertions use exact counts — strong.
- Taint checks use specific Taint variants — strong.
- Slot isolation checks use exact slot equality/inequality — strong.

### What would still pass if behavior were deleted (mutation thought experiment)
- If Repeat's counter increment logic were deleted but the terminal check hardcoded to 3, `repeat_finishes_with_prior_answer_42` would still pass if the body executed 3 times for another reason. However, `assert_body_counts` mitigates this by checking body execution count independently.
- If Reduce's accumulator write were deleted but the initial value propagated, `reduce_empty_list_preserves_source_values` would still pass (initial=10, result=10). The EvidenceGapsRecorded failure suggests this is being caught by the evidence mechanism.
- If slot aliasing protection were deleted, `generated_composite_roles_do_not_alias_source_outputs` would catch it — good.

## Bounded Fixture vs Full Parent Acceptance

The frozen19 suite is a **bounded fixture** test suite:
- Tests use specific, small inputs (list sizes 0-3, specific sentinel values)
- Tests drive to completion in single calls (no suspension/rearm)
- Tests use only sequential composition (Together→Repeat, Repeat→Together), not arbitrary nesting

This suite provides **strong evidence of correctness for the tested cases** but does not constitute **full F1 parent acceptance** because:
1. Repeat behavior is not fully covered (max_attempts=1, counter journal updates)
2. Reduce evidence gaps are not resolved (empty/single-item cases)
3. Control flow edge cases are not tested
4. Suspension/rearm behavior is not tested
5. Arbitrary nested/multistep acceptance is not tested (sequential only)

## Verdict

**STATUS: WIP — Suite is internally consistent but incomplete for F1 parent acceptance.**

The frozen19 suite is well-designed with strong, concrete assertions that correctly catch implementation gaps (the 7 failures are real, not test bugs). The test architecture (compile_workflow + unchanged IR + evidence-based body counts + sentinel/taint preservation) is sound.

However, the suite has known, documented coverage gaps for Repeat and Reduce that must be addressed before F1 parent acceptance. The remaining work is implementation-focused (fix Repeat state transitions, fix Reduce evidence for empty/single-item cases), not test-design focused.

**No fabricated PASS.** All 12 passing tests executed successfully. All 7 failing tests failed for genuine implementation reasons documented in the raw logs. No tests were ignored or filtered. No acceptance from names only — each test's actual behavior was verified against the raw execution logs.

**Oracle strength:** The suite's oracles (sentinel values, body execution counts, taint preservation, slot isolation, IR stability) are strong and not relaxed. The suite correctly fails when the implementation does not meet its expectations.

**Reviewer note:** Previous test reviews overclaimed loop counts, taint coverage, and nesting coverage. This review enumerates exact test names, maps assertions to actual execution behavior, and distinguishes bounded-fixture strength from full parent acceptance explicitly.
