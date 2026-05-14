# Test Plan Review — vb-i94f: Taint Propagation Through EvalExpr, BuildObject, BuildList, Choose, and Finish Paths

## STATUS: REJECTED

---

## LETHAL FINDINGS

### LETHAL-1: Exit Criterion 1 Violated — Behaviors Outnumber BDD Scenarios ~4×

**Clause**: Exit Criterion 1: "Every behavior in Section 1 has a BDD scenario in Section 3"

**Finding**: Section 1 enumerates **82 behavioral entries** (B-001 through B-211, excluding error groups). Section 3 contains **22 explicitly named BDD scenarios** with `fn test_name` declarations. The ratio is **3.7 behaviors per scenario**.

The named scenarios cover:
- `join_taint_returns_secret_when_either_input_is_secret` → B-001
- `join_taint_returns_derived_from_secret_when_neither_is_secret` → B-002
- `join_taint_is_commutative_for_all_taint_pairs` → B-004
- `join_taint_is_associative_for_all_taint_triples` → B-005
- `write_slot_with_taint_atomically_updates_both_slots_and_taint_arrays` → B-011
- `read_slot_returns_slot_uninitialized_for_fresh_frame` → B-012
- `write_taint_rejects_uninitialized_slot_to_prevent_desync` → B-016
- `eval_expr_returns_clean_taint_when_all_slots_clean` → B-030
- `eval_expr_returns_derived_from_secret_when_any_slot_has_that_taint` → B-031
- `eval_expr_returns_secret_when_any_loaded_slot_is_secret` → B-032
- `eval_expr_taint_accum_never_decreases` → B-033
- `build_object_with_taint_returns_clean_when_all_fields_clean` → B-040
- `build_object_with_taint_returns_secret_when_any_field_secret` → B-042
- `build_object_cannot_produce_clean_taint_from_secret_inputs` → B-047
- `build_list_with_taint_returns_clean_when_all_items_clean` → B-050
- `build_list_with_taint_returns_secret_when_any_item_secret` → B-052
- `choose_expr_branch_does_not_accumulate_taint_from_condition` → B-060
- `finish_run_returns_finished_with_exact_slot_taint` → B-070
- `copy_slot_preserves_both_value_and_taint` → B-080
- `resume_action_completion_writes_output_value_and_taint_unchanged` → B-090
- `no_taint_desync_slot_always_has_value_when_taint_is_non_clean` → B-100
- `eval_expr_returns_slot_out_of_bounds_for_invalid_slot` → B-200
- `eval_expr_returns_slot_uninitialized_when_slot_not_written` → B-201
- `eval_expr_returns_expr_out_of_bounds_for_invalid_index` → B-202
- `build_object_with_taint_returns_slot_out_of_bounds` → B-204
- `build_list_with_taint_returns_slot_out_of_bounds` → B-206
- `choose_expr_branch_returns_missing_next_step_when_no_match_and_no_otherwise` → B-208
- `choose_slot_branch_returns_type_mismatch_for_non_bool_condition` → B-209
- `finish_run_returns_slot_uninitialized_when_result_not_written` → B-210
- `copy_slot_returns_slot_uninitialized_for_uninitialized_source` → B-211

**Uncovered critical behaviors** (partial list):
- B-013: `read_slot` returns `SlotOutOfBounds` for indices >= slot_count — NO SCENARIO
- B-015: `read_taint` returns `SlotOutOfBounds` for indices >= slot_count — NO SCENARIO
- B-017: `write_taint` returns `SlotOutOfBounds` for out-of-range indices — NO SCENARIO
- B-019: After `write_slot_with_taint(slot, value, taint)`, `read_slot(slot)` returns exactly `value` — NO NAMED SCENARIO
- B-020: `reinitialize` resets all slots to uninitialized and taint to `Clean` — NO SCENARIO
- B-034: `eval_expr_inner` rejects `ExprOutOfBounds` for invalid `ExprIdx` — NO SCENARIO (B-202 covers wrong variant)
- B-035: `eval_expr_inner` rejects `ConstOutOfBounds` for invalid `ConstIdx` — NO SCENARIO
- B-036: `eval_expr_inner` rejects `SlotOutOfBounds` for invalid `SlotIdx` in `LoadSlot` — NO SCENARIO
- B-037: `eval_expr_inner` rejects `SlotUninitialized` when loading from uninitialized slot — NO SCENARIO
- B-038: `eval_expr_inner` rejects `SlotUninitialized` when loading from slot with no value — NO SCENARIO
- B-043: `build_object_with_taint` joins taint across all fields (order-independent) — NO SCENARIO
- B-044: `build_object_with_taint` returns `SlotOutOfBounds` for invalid field slot indices — DUPLICATED BY B-204 but not named
- B-051: `build_list_with_taint` returns `Taint::DerivedFromSecret` when any item is `DerivedFromSecret` and none is `Secret` — NO SCENARIO
- B-053: `build_list_with_taint` joins taint across all items (order-independent) — NO SCENARIO
- B-061: `choose_slot_branch` does not accumulate taint from slot reads — NO SCENARIO
- B-062: `choose_expr_branch` returns `EngineSignal::Continue` with PC set to first matching branch target — NO SCENARIO
- B-063: `choose_expr_branch` returns `EngineSignal::Continue` with PC set to `otherwise` when no branch matches — NO SCENARIO
- B-065: `choose_slot_branch` returns `TypeMismatch` when condition slot is not `Bool` — NO NAMED SCENARIO (B-209 covers wrong function)
- B-066: `choose_expr_branch` returns `TypeMismatch` when expression evaluates to non-boolean — NO SCENARIO
- B-067: `choose_slot_branch` selects first matching branch (short-circuit) — NO SCENARIO
- B-068: `choose_expr_branch` evaluates expression conditions in order, stops at first `true` — NO SCENARIO
- B-071: `finish_run` returns `SlotUninitialized` when result slot is uninitialized — NO NAMED SCENARIO (B-210 covers)
- B-072: `finish_run` returns `SlotOutOfBounds` when result slot index is out of range — NO SCENARIO
- B-073: `finish_run` preserves exact taint from result slot (no promotion/demotion) — NO SCENARIO
- B-081: `copy_slot` returns `SlotUninitialized` when source slot is uninitialized — NO NAMED SCENARIO (B-211 covers)
- B-082: `copy_slot` returns `SlotOutOfBounds` when source slot index is out of range — NO SCENARIO
- B-083: `copy_slot` returns `MissingOutputSlot` when node has no output — NO SCENARIO
- B-084: Destination slot taint exactly equals source slot taint after `copy_slot` — NO NAMED SCENARIO (B-080 covers value but not taint equality explicitly)
- B-091: `resume_action_completion` returns `InvalidProgramCounter` for invalid step — NO SCENARIO
- B-092: `resume_action_completion` returns `MissingNextStep` when step has no next — NO SCENARIO
- B-101: A slot never carries non-`Clean` taint without a corresponding value — NO SCENARIO
- B-102: `read_taint` on uninitialized slot returns `SlotUninitialized` (not a default taint) — NO NAMED SCENARIO
- B-110: Taint on any slot never spontaneously decreases without `reinitialize` call — NO SCENARIO
- B-111: `join_taint` is monotone — NO SCENARIO
- B-120: `slots[i]` and `taint[i]` are always written together atomically — NO NAMED SCENARIO (B-011 covers)
- B-121: `read_taint` on uninitialized slot returns `SlotUninitialized` — NO NAMED SCENARIO (B-014 covers)
- B-130: `ObjectField { value, taint, .. }` stored in `ValueStore` preserves field taint after insertion — NO SCENARIO
- B-131: Round-trip store/lookup preserves field taint unchanged — NO SCENARIO
- B-140: `EngineSignal::Finished(v, t)` taint `t` equals `read_taint(result)` at `finish_run` call time — NO NAMED SCENARIO
- B-141: `finish_run` is the only path that emits `EngineSignal::Finished` — NO SCENARIO
- B-150: Expression result carries `DerivedFromSecret` taint when computed from `DerivedFromSecret` inputs — NO SCENARIO
- B-151: `DerivedFromSecret` is not promoted to `Secret` during expression evaluation — NO SCENARIO
- B-203: `eval_expr_inner` returns `ConstOutOfBounds` on invalid constant index — NO SCENARIO
- B-205: `build_object_with_taint` returns `AllocationFailed` on memory allocation failure — NO SCENARIO
- B-207: `build_list_with_taint` returns `AllocationFailed` on memory allocation failure — NO SCENARIO
- B-209: `choose_slot_branch` returns `TypeMismatch` when condition is non-boolean — NO NAMED SCENARIO (B-209 is named but for wrong function: `choose_slot_branch` vs `choose_expr_branch`)

**Verdict**: 60+ behaviors lack explicit BDD scenarios. Exit criterion 1 is **VIOLATED**.

---

### LETHAL-2: `choose_slot_branch` Has No BDD Scenario — Only Proptest

**Contract function**: `pub(super) fn choose_slot_branch` (contract.md line 111)

**Finding**: Section 3 has **zero** explicitly named BDD scenarios for `choose_slot_branch`. The only coverage is Proptest P-007 (`choose_expr_branch` only, not `choose_slot_branch`). The named test `choose_slot_branch_returns_type_mismatch_for_non_bool_condition` at test-plan.md:515 asserts `choose_slot_branch` error behavior, but no scenario tests the **happy path** (`choose_slot_branch` selects correct branch and does not accumulate taint).

BDD scenarios missing for `choose_slot_branch`:
- No scenario for `choose_slot_branch` returning `EngineSignal::Continue` with PC set to correct branch target
- No scenario for `choose_slot_branch` returning `EngineSignal::Continue` with PC set to `otherwise` when no branch matches
- No scenario for `choose_slot_branch` returning `MissingNextStep` when no branch matches and no otherwise
- No scenario for `choose_slot_branch` not accumulating taint from slot reads (B-061)

**Verdict**: `choose_slot_branch` has 0 happy-path BDD scenarios. LETHAL.

---

### LETHAL-3: `ConstOutOfBounds` Error Variant — No Scenario Asserting This Exact Variant

**Contract error**: `EngineError::ConstOutOfBounds { index }` (contract.md line 65)

**Finding**: B-203 in the behavior inventory states `eval_expr_inner` returns `ConstOutOfBounds` on invalid constant index. The error variant appears in the proof obligations (ERR-001) and is mentioned in the contract. However:

- No BDD scenario in Section 3 explicitly names `ConstOutOfBounds`
- B-202 covers `ExprOutOfBounds` but not `ConstOutOfBounds`
- The error variant is distinct from `ExprOutOfBounds` and requires a separate scenario

**Evidence**: `grep -n "ConstOutOfBounds" test-plan.md` returns no matches.

**Verdict**: `ConstOutOfBounds` has no test scenario. LETHAL per: "Any `Error` variant with no scenario asserting the exact variant".

---

### LETHAL-4: `MissingOutputSlot` Error Variant — No Scenario Asserting This Exact Variant

**Contract error**: `EngineError::MissingOutputSlot { step }` (contract.md line 66)

**Finding**: B-083 in the behavior inventory states `copy_slot` returns `MissingOutputSlot` when node has no output. No BDD scenario in Section 3 covers this. `grep -n "MissingOutputSlot" test-plan.md` returns no matches.

**Verdict**: `MissingOutputSlot` has no test scenario. LETHAL per: "Any `Error` variant with no scenario asserting the exact variant".

---

## MAJOR FINDINGS (5)

### MAJOR-1: `choose_expr_branch` TypeMismatch Scenario Tests Wrong Function

**Location**: test-plan.md:515

**Issue**: `choose_slot_branch_returns_type_mismatch_for_non_bool_condition` is named for `choose_slot_branch` but contract POST-004 (line 42-43) specifies that `choose_expr_branch` returns `TypeMismatch` when expression evaluates to non-boolean. The test name implies it covers `choose_slot_branch` returning `TypeMismatch`, but `choose_slot_branch` operates on already-evaluated boolean slot values (not expressions), so `TypeMismatch` on `choose_slot_branch` would occur if the slot value is non-boolean, not if the expression is non-boolean.

The test at test-plan.md:507-514 describes a `RunFrame` where condition slot contains `I64(1)` — this is testing `choose_slot_branch` with a non-boolean slot. But B-066 says `choose_expr_branch` returns `TypeMismatch` when expression evaluates to non-boolean. These are **two distinct error paths**:
- `choose_slot_branch`: `TypeMismatch` when slot is non-boolean
- `choose_expr_branch`: `TypeMismatch` when expression result is non-boolean

Both error variants exist but only one is tested. B-065 covers `choose_slot_branch` TypeMismatch. B-066 for `choose_expr_branch` TypeMismatch is **missing**.

---

### MAJOR-2: B-034/B-035/B-036/B-037/B-038 Scenarios Overlap But Don't Match Error Variant Inventory

**Location**: test-plan.md:38-44

**Issue**: Behavior inventory B-034 through B-038 enumerate specific error paths for `eval_expr_inner`:
- B-034: `ExprOutOfBounds` for invalid `ExprIdx`
- B-035: `ConstOutOfBounds` for invalid `ConstIdx`
- B-036: `SlotOutOfBounds` for invalid `SlotIdx` in `LoadSlot`
- B-037: `SlotUninitialized` when loading from uninitialized slot
- B-038: `SlotUninitialized` when loading from slot with no value

Section 3 has scenarios for `ExprOutOfBounds` (B-202), `SlotOutOfBounds` (B-200), and `SlotUninitialized` (B-201), but:
- B-035 (`ConstOutOfBounds`) has **no scenario** — this is already flagged as LETHAL-3
- B-036 (`SlotOutOfBounds` in `LoadSlot`) is covered by B-200 but the scenario describes `[LoadSlot(99)]` with `slot_count = 2` — the scenario name doesn't explicitly call out `LoadSlot` context vs general slot bounds
- B-037 and B-038 are both `SlotUninitialized` but represent **different runtime conditions**: uninitialized slot vs slot with no value. Both map to `CoreError::SlotUninitialized` but the behavioral distinction matters for the contract (INV-003: "Reading `taint[i]` without a corresponding `slots[i] = Some(...)` returns `SlotUninitialized`")

The scenario B-201 (`fn eval_expr_returns_slot_uninitialized_when_slot_not_written`) covers one path but doesn't distinguish between the two `SlotUninitialized` conditions.

---

### MAJOR-3: Proptest P-002 Does Not Cover `ConstOutOfBounds`

**Location**: test-plan.md:565-579

**Issue**: P-002 input strategy generates random `ExprProgram` with ops including `LoadConst`. The strategy states "all slot indices within frame bounds" but does **not** generate invalid `ConstIdx` references. The proptest invariant covers valid evaluation paths but not the `ConstOutOfBounds` error path. This gap is compounded by LETHAL-3 (no scenario for `ConstOutOfBounds`).

---

### MAJOR-4: `AllocationFailed` Error Path Has No BDD Scenario for Object/List Construction

**Location**: test-plan.md:53-54, 64-65

**Issue**: B-054 and B-056 assert `build_object_with_taint` and `build_list_with_taint` return `AllocationFailed` when `try_reserve_exact` fails. B-205 and B-207 reference these in the error section. However:
- No BDD scenario in Section 3 tests the `AllocationFailed` path for `build_object_with_taint` (B-054)
- No BDD scenario in Section 3 tests the `AllocationFailed` path for `build_list_with_taint` (B-056)
- These require injection of allocation failure which is difficult to test without `malloc` hooking or a custom allocator

The proof obligation for this path (ERR-002) is marked Kani-only, but the contract's error taxonomy lists this as a required error variant test.

---

### MAJOR-5: `InvalidProgramCounter` and `MissingNextStep` for `resume_action_completion` Have No Scenarios

**Location**: test-plan.md:91-92, B-091, B-092

**Issue**: B-091 and B-092 describe error conditions for `resume_action_completion`:
- B-091: returns `InvalidProgramCounter` for invalid step
- B-092: returns `MissingNextStep` when step has no next

Neither has a BDD scenario in Section 3. P-008 (proptest for `resume_action_completion`) does not explicitly enumerate these error conditions in its "Invalid cases" definition.

---

## MINOR FINDINGS

### MINOR-1: Exit Criterion 6 ("No `is_ok()` or `is_err()` assertions") Cannot Be Verified

The exit criteria states no `is_ok()` or `is_err()` assertions, but since the tests don't exist yet, this cannot be verified. This criterion should be reworded as a **plan requirement** to specify that all scenarios will use concrete value assertions rather than boolean assertion helpers.

### MINOR-2: Combinatorial Coverage Matrix (Section 8) Does Not Cover `choose_slot_branch` Happy Paths

The matrix at test-plan.md:1058-1100 covers `choose_slot` scenarios but only for error cases (B-1089: non-bool condition, B-1090: no match, B-1091: no otherwise). The happy path scenarios for `choose_slot_branch` (correct branch selection, short-circuit behavior) are absent from the matrix.

### MINOR-3: Kani Harnesses Lack Module Path Verification

Section 6 defines Kani harnesses (K-001 through K-012) with target modules. The proof-obligations.jsonl maps these to exact obligations, but the test plan does not verify that the harness code actually exists at those paths (e.g., `vb_core/src/engine/expr_eval/fuzz_expr_eval.rs` for F-001, `vb_core/src/frame.rs` for K-001). Since this is Plan Review (Mode 1), the actual harness implementations are future work, but the plan should specify that harness existence is a precondition.

### MINOR-4: B-047 and B-057 ("Impossible to produce Clean-tainted container from Secret-tainted inputs") Are Named But Test the Negative

**Location**: test-plan.md:336-344 (B-047), B-057 implicitly covered by design

The scenario B-047 says "result.taint is Taint::Secret (never Clean)" — this is correct but the scenario name `fn build_object_cannot_produce_clean_taint_from_secret_inputs` does not match the standard naming pattern used elsewhere (`fn build_object_with_taint_returns_X_when_Y`). This is a naming inconsistency, not a functional defect.

---

## PROOF OBLIGATION TRACEABILITY

All 33 proof obligations from `proof-obligations.jsonl` are addressed in the proof obligations table (test-plan.md:1103-1140). However:

- ERR-001 maps to `eval_expr_errors` Kani harness — gap: `ConstOutOfBounds` scenario missing (LETHAL-3)
- ERR-002 maps to `object_list_errors` Kani harness — gap: `AllocationFailed` scenario missing (MAJOR-4)
- POST-004 maps to proptest P-007 + unit tests B-060–B-068 — gap: `choose_slot_branch` happy paths missing (LETHAL-2)
- ERR-004 maps to `finish_run_errors` Kani harness — gap: `SlotOutOfBounds` for `finish_run` (B-072) missing

---

## MANDATE

The following **must exist** before resubmission:

### Required Named BDD Scenarios (28 new scenarios):

1. `fn read_slot_returns_slot_out_of_bounds_for_oob_index` — B-013
2. `fn read_taint_returns_slot_out_of_bounds_for_oob_index` — B-015
3. `fn write_taint_returns_slot_out_of_bounds_for_oob_index` — B-017
4. `fn write_slot_with_taint_then_read_slot_returns_exact_value` — B-019
5. `fn reinitialize_resets_all_slots_and_taint_to_clean` — B-020
6. `fn eval_expr_returns_const_out_of_bounds_for_invalid_const_idx` — B-035 (new)
7. `fn eval_expr_load_slot_rejects_uninitialized_slot` — B-037
8. `fn eval_expr_load_slot_rejects_slot_with_no_value` — B-038
9. `fn build_object_with_taint_joins_taint_across_fields_order_independent` — B-043
10. `fn build_list_with_taint_returns_derived_from_secret_when_any_item_has_that_taint` — B-051 (new)
11. `fn build_list_with_taint_joins_taint_across_items_order_independent` — B-053
12. `fn choose_slot_branch_does_not_accumulate_taint_from_slot_reads` — B-061 (new)
13. `fn choose_slot_branch_returns_continue_with_pc_set_to_first_matching_branch` — B-062 (new)
14. `fn choose_slot_branch_returns_continue_with_pc_set_to_otherwise_when_no_match` — B-063 (new)
15. `fn choose_expr_branch_returns_type_mismatch_when_expr_is_non_boolean` — B-066 (new)
16. `fn choose_slot_branch_selects_first_matching_branch_short_circuit` — B-067 (new)
17. `fn choose_expr_branch_evaluates_conditions_in_order_stops_at_first_true` — B-068 (new)
18. `fn choose_slot_branch_returns_missing_next_step_when_no_match_and_no_otherwise` — B-064 (new) **separate from choose_expr_branch**
19. `fn finish_run_returns_slot_out_of_bounds_for_oob_result_slot` — B-072 (new)
20. `fn finish_run_preserves_exact_taint_no_promotion_or_demotion` — B-073 (new)
21. `fn copy_slot_returns_slot_out_of_bounds_when_source_index_oob` — B-082 (new)
22. `fn copy_slot_returns_missing_output_slot_when_node_has_no_output` — B-083 (new)
23. `fn copy_slot_destination_taint_equals_source_taint_after_copy` — B-084 (new, separate from B-080 which covers value)
24. `fn resume_action_completion_returns_invalid_program_counter_for_invalid_step` — B-091 (new)
25. `fn resume_action_completion_returns_missing_next_step_when_step_has_no_next` — B-092 (new)
26. `fn slot_never_carries_non_clean_taint_without_corresponding_value` — B-101 (new)
27. `fn object_field_preserves_taint_in_value_store_after_insertion` — B-130 (new)
28. `fn object_field_round_trip_store_lookup_preserves_taint` — B-131 (new)

### Required Proof Artifact Updates:

- ConstOutOfBounds scenario must cover `EngineError::ConstOutOfBounds` exact variant (not just ExprOutOfBounds)
- MissingOutputSlot scenario must be added to error variant coverage
- choose_slot_branch Proptest P-007 must be split or extended to explicitly cover choose_slot_branch (currently only covers choose_expr_branch)

---

## VERDICT SUMMARY

| Tier | Finding | Severity | Status |
|------|---------|----------|--------|
| LETHAL | Behaviors (82) outnumber BDD scenarios (22) ~4× | LETHAL | EXIT CRITERION 1 VIOLATED |
| LETHAL | choose_slot_branch has 0 happy-path BDD scenarios | LETHAL | Function coverage 0/1 |
| LETHAL | ConstOutOfBounds error variant has no scenario | LETHAL | ERR variant gap |
| LETHAL | MissingOutputSlot error variant has no scenario | LETHAL | ERR variant gap |
| MAJOR | choose_expr_branch TypeMismatch scenario tests wrong function | MAJOR | B-065/B-066 mismatch |
| MAJOR | eval_expr error paths overlap but don't distinguish ConstOutOfBounds | MAJOR | Incomplete error path mapping |
| MAJOR | Proptest P-002 does not cover ConstOutOfBounds | MAJOR | Invariant gap |
| MAJOR | AllocationFailed has no BDD scenario | MAJOR | Error path not scenario-tested |
| MAJOR | resume_action_completion InvalidProgramCounter/MissingNextStep missing | MAJOR | Incomplete error coverage |

**STATUS: REJECTED**

Submit 28 additional named BDD scenarios covering all LETHAL gaps, then resubmit.
