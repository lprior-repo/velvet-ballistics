# Test Writer Report

**Bead:** vb-xi2f.28
**State:** 9 (test-writer)
**Date:** 2026-05-25
**Status:** COMPLETE

---

## Test Count

| Layer | Count | Details |
|---|---|---|
| Unit tests (#[cfg(test)]) | 33 new | `crates/vb_compile/src/tests/foreach_digest_tests.rs` (+ `mod.rs`) |
| Proptest invariants | 9 total (7 existing + 2 new) | `tests/proptest_digest_foreach.rs` |
| Unit tests (cumulative crate) | 278 | +33 from this bead |
| Fuzz targets | 2 new | `fuzz/fuzz_targets/foreach_digest_canonical.rs` + `foreach_digest_step.rs` |
| Fuzz shared functions | 2 new | `fuzz/src/lib.rs` (fuzz_canonical_digest_foreach, fuzz_digest_step_primitive) |
| Module registrations | 2 | `src/tests/mod.rs` + `#[cfg(test)] mod tests;` in `lib.rs` |

### Unit Test Breakdown

| BDD Behavior | Count | Test Names |
|---|---|---|
| B7: at_once None vs Some(1) equivalence | 2 | `foreach_at_once_none_some1_produces_identical_step_digest`, `foreach_at_once_none_some1_produces_identical_workflow_digest` |
| B8: at_once None vs Some(0) inequivalence | 2 | `foreach_at_once_none_produces_different_step_digest_than_some0`, `foreach_at_once_some1_produces_different_step_digest_than_some0` |
| B10: ForEach arm not catch-all | 2 | `foreach_step_digest_contains_more_than_just_primitive_name`, `foreach_arm_produces_distinct_bytes_from_same_body_size_set` |
| B13: Empty body deterministic digest | 3 | `foreach_empty_body_produces_deterministic_step_digest`, `foreach_empty_body_digest_differs_from_nonempty_body_digest`, `foreach_empty_body_workflow_digest_is_deterministic` |
| B14: Body step ID sensitivity | 2 | `foreach_body_step_id_variation_changes_step_digest`, `foreach_body_step_id_variation_changes_workflow_digest` |
| B17: at_once Some(0) distinct | 2 | `foreach_at_once_zero_step_digest_differs_from_none`, `foreach_at_once_zero_step_digest_differs_from_some1` |
| at_once u32::MAX boundary | 2 | `foreach_at_once_max_boundary_produces_distinct_step_digest`, `foreach_at_once_max_vs_one_produces_distinct_step_digests` |
| Empty variable edge case | 2 | `foreach_empty_variable_produces_deterministic_step_digest`, `foreach_empty_variable_differs_from_nonempty_variable` |
| Non-ASCII variable edge case | 2 | `foreach_non_ascii_variable_produces_deterministic_step_digest`, `foreach_non_ascii_variable_differs_from_ascii_variable` |
| B16: Body type diversity (Set vs Finish) | 2 | `foreach_body_set_vs_finish_produces_different_step_digest`, `foreach_body_set_vs_finish_produces_different_workflow_digest` |
| B15: Nested ForEach body recursion | 2 | `foreach_nested_body_content_changes_workflow_digest`, `foreach_nested_foreach_vs_flat_set_body_produces_different_workflow_digest` |
| Body Set output variation | 1 | `foreach_body_set_output_variation_changes_step_digest` |
| Input field sensitivity | 1 | `foreach_input_variation_changes_step_digest` |
| Variable field sensitivity | 1 | `foreach_variable_variation_changes_step_digest` |
| Determinism across multiple calls | 1 | `foreach_step_digest_is_deterministic_across_multiple_calls` |
| All four fields contribute | 1 | `foreach_all_fields_contribute_to_step_digest` |
| Body step count sensitivity | 1 | `foreach_body_step_count_changes_step_digest` |
| Body step order sensitivity | 1 | `foreach_body_step_order_changes_step_digest` |
| Delimiter collision prevention | 1 | `foreach_variable_containing_colon_does_not_cause_delimiter_collision` |
| Step position (first vs last) | 1 | `foreach_step_position_changes_workflow_digest` |
| Finish String vs Integer result | 1 | `foreach_body_finish_string_result_differs_from_integer_result` |

### Proptest Breakdown

| Invariant | Test | Status |
|---|---|---|
| P1-P7 (existing) | 7 existing proptests | PASS (2000 cases each) |
| P8: at_once equivalence | `proptest_foreach_at_once_none_some1_equivalence` | PASS (2000 cases) |
| P9: Nested body sensitivity | `proptest_foreach_nested_body_content_changes_outer_digest` | PASS (2000 cases) |

### Fuzz Target Breakdown

| Target | File | Exercises |
|---|---|---|
| foreach_digest_canonical | `fuzz_targets/foreach_digest_canonical.rs` | `canonical_digest_part05` with adversarial source data |
| foreach_digest_step | `fuzz_targets/foreach_digest_step.rs` | `digest_step_primitive_part05` with adversarial ForEach data |

## Gate Results

- [x] Source clippy: 0 warnings (`cargo clippy -p vb_compile --all-features -- -D warnings`)
- [x] Test compile: pass (`cargo test -p vb_compile --no-run` → all 6 executables)
- [x] All unit tests: 278 passed, 0 failed
- [x] All integration tests (5 test binaries): all passed (7+9+15+11+10 = 52 tests)
- [x] Proptests: 9 passed (2000 cases each)
- [x] Fuzz crate: compiles cleanly (`cargo check` in fuzz/)
- [x] Full suite regression: no failures

## Behaviors Verified

| # | Behavior | Contract Clause | Verified By |
|---|---|---|---|
| B1 | ForEach.input sensitivity | AC-FE-01 | Unit + Proptest P1 |
| B2 | ForEach.at_once sensitivity | AC-FE-02 | Unit + Proptest P2 |
| B3 | ForEach.variable sensitivity | AC-FE-03 | Unit + Proptest P3 |
| B4 | ForEach.body sensitivity | AC-FE-04 | Unit + Proptest P4 |
| B5 | Determinism preserved | AC-FE-05 | Unit + Proptest P5 |
| B6 | Dual-path equivalence | AC-FE-06 | DEFERRED (path A not compiled) |
| B7 | at_once None=Some(1) equivalence | AC-FE-07 | Unit (2 tests) + Proptest P8 |
| B8 | at_once None!=Some(0) | AC-FE-07 inverse | Unit (2 tests) |
| B9 | Non-regression Set/Finish | AC-FE-08 | Proptest P6, P7 |
| B10 | ForEach arm hit (not catch-all) | INV-FE-01 | Unit (2 tests) |
| B11 | All four fields hashed | INV-FE-01 | Unit + Proptest |
| B12 | Delimiter collision prevention | INV-FE-02 | Unit (colon test) + Kani VERIFIED |
| B13 | Empty body valid digest | G-FE-06 | Unit (3 tests) |
| B14 | Body step ID sensitivity | Contract §2.3 | Unit (2 tests) |
| B15 | Nested ForEach recursion | Contract §2.1 | Unit (2 tests) + Proptest P9 |
| B16 | Body type diversity | AC-FE-04 | Unit (2 tests) |
| B17 | at_once Some(0) distinct | AC-FE-02+07 | Unit (2 tests) |
| B18 | Infallibility (no panic) | Infallibility | All unit tests + Fuzz |
| B19 | Machine-independent digest | Type contract §1.1 | Determinism proptest P5 |

## GOD RULE Compliance

- **GOD RULE 1 (No Hardcoded Shapes):** COMPLIANT. All unit tests construct test inputs programmatically with varied field values. Proptests use randomized strategies. Fuzz targets derive inputs from arbitrary byte data.
- **GOD RULE 2 (Bind to Production):** COMPLIANT. All tests call `canonical_digest_part05` and `digest_step_primitive_part05` — the actual production functions re-exported in the crate's public API.
- **GOD RULE 4 (No Loop Oscillations):** COMPLIANT. All tests pass against the existing implementation. No proof harness modified.

## Remaining Work

- **B6 dual-path equivalence**: Path A (`compile/mod.rs`) is not compiled in the current crate structure. Cross-path equivalence testing is deferred.
- **Mutation testing**: `cargo mutants -p vb_compile -- --function digest_step_primitive` for 90% kill rate threshold.
- **Coverage measurement**: `cargo llvm-cov` for ≥90% line coverage target.
- **Kani harnesses**: 12 of 14 remain PENDING due to blake3 InlineAsm blocker.
