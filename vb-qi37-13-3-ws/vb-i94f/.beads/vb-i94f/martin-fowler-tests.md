# Martin Fowler Test Plan: Taint Propagation Through EvalExpr, BuildObject, BuildList, Choose, and Finish Paths

## Happy Path Tests

### test_untainted_expression_output_remains_untainted
**Given**: A workflow with an `EvalExpr` node that reads two `Clean`-tainted slots and adds them
**When**: The expression is evaluated via `eval_expr_with_store`
**Then**: The returned taint is `Taint::Clean` and the output slot carries `Taint::Clean`

### test_untainted_build_object_output_remains_untainted
**Given**: A `BuildObject` node with all field slots carrying `Taint::Clean`
**When**: `build_object_with_taint` is called
**Then**: The returned object handle carries `Taint::Clean` and the output slot is written with `Taint::Clean`

### test_untainted_build_list_output_remains_untainted
**Given**: A `BuildList` node with all item slots carrying `Taint::Clean`
**When**: `build_list_with_taint` is called
**Then**: The returned list handle carries `Taint::Clean` and the output slot is written with `Taint::Clean`

### test_copy_slot_preserves_clean_taint
**Given**: A `Copy` node where the source slot carries `Taint::Clean`
**When**: `copy_slot` executes
**Then**: The destination slot value equals the source value and the destination slot taint equals `Taint::Clean`

### test_finish_clean_result_signal_carries_clean_taint
**Given**: A `Finish` node targeting a slot with `Taint::Clean`
**When**: `finish_run` executes
**Then**: `EngineSignal::Finished(value, Taint::Clean)` is returned with the correct value

### test_choose_with_clean_conditions_takes_first_true_branch
**Given**: A `Choose` node with two `ExprBranch` conditions, both evaluating to `true` from `Clean` slots
**When**: `choose_expr_branch` executes
**Then**: PC is set to the first matching branch target

### test_choose_slot_with_clean_conditions_takes_branch
**Given**: A `ChooseSlot` node with two `SlotBranch` conditions reading `true` from `Clean` slots
**When**: `choose_slot_branch` executes
**Then**: PC is set to the first matching branch target

---

## Error Path Tests

### test_eval_expr_returns_slot_uninitialized_when_slot_not_written
**Given**: An expression that references a slot that has never been written
**When**: `eval_expr_with_store` is called
**Then**: `EngineError::SlotUninitialized` is returned

### test_eval_expr_returns_expr_out_of_bounds_for_invalid_index
**Given**: A workflow with `ExprIdx(99)` that does not exist
**When**: `eval_expr_with_store` is called with that index
**Then**: `EngineError::ExprOutOfBounds { expr: ExprIdx(99) }` is returned

### test_build_object_returns_slot_uninitialized_for_unwritten_field_slot
**Given**: A `BuildObject` node with one field slot that has never been initialized
**When**: `build_object_with_taint` is called
**Then**: `EngineError::SlotUninitialized` is returned and no object handle is inserted

### test_build_list_returns_slot_uninitialized_for_unwritten_item_slot
**Given**: A `BuildList` node with one item slot that has never been initialized
**When**: `build_list_with_taint` is called
**Then**: `EngineError::SlotUninitialized` is returned and no list handle is inserted

### test_build_object_returns_allocation_failed_on_reserve_failure
**Given**: A `BuildObject` node with fields; the `Vec` reserve fails (simulated via test injection)
**When**: `build_object_with_taint` is called
**Then**: `EngineError::AllocationFailed` is returned

### test_choose_returns_missing_next_step_when_no_branch_matches_and_no_otherwise
**Given**: A `Choose` node with all branches evaluating to `false` and `otherwise = None`
**When**: `choose_expr_branch` executes
**Then**: `EngineError::MissingNextStep { step }` is returned

### test_choose_slot_returns_type_mismatch_when_condition_non_boolean
**Given**: A `ChooseSlot` node where a branch condition slot contains `I64(1)` instead of `Bool`
**When**: `choose_slot_branch` executes
**Then**: `EngineError::TypeMismatch { expected: "boolean", found: "number" }` is returned

### test_finish_returns_slot_uninitialized_when_result_slot_not_written
**Given**: A `Finish` node targeting a slot that has never been initialized
**When**: `finish_run` executes
**Then**: `EngineError::SlotUninitialized` is returned

### test_finish_returns_slot_out_of_bounds_for_invalid_result_slot
**Given**: A `Finish` node targeting `SlotIdx(99)` which exceeds the frame's slot count
**When**: `finish_run` executes
**Then**: `EngineError::SlotOutOfBounds { slot: SlotIdx(99) }` is returned

---

## Edge Case Tests

### test_taint_join_clean_and_clean_is_clean
**Given**: Two slots both carrying `Taint::Clean`
**When**: `join_taint(Taint::Clean, Taint::Clean)` is called
**Then**: The result is `Taint::Clean`

### test_taint_join_clean_and_secret_is_secret
**Given**: `join_taint(Taint::Clean, Taint::Secret)`
**When**: The function is evaluated
**Then**: The result is `Taint::Secret`

### test_taint_join_clean_and_derived_is_derived
**Given**: `join_taint(Taint::Clean, Taint::DerivedFromSecret)`
**When**: The function is evaluated
**Then**: The result is `Taint::DerivedFromSecret`

### test_taint_join_secret_and_secret_is_secret
**Given**: `join_taint(Taint::Secret, Taint::Secret)`
**When**: The function is evaluated
**Then**: The result is `Taint::Secret`

### test_taint_join_derived_and_secret_is_secret
**Given**: `join_taint(Taint::DerivedFromSecret, Taint::Secret)`
**When**: The function is evaluated
**Then**: The result is `Taint::Secret`

### test_taint_join_is_commutative_all_pairs
**Given**: All nine combinations of `(Clean, Clean)`, `(Clean, Derived)`, `(Clean, Secret)`, `(Derived, Clean)`, `(Derived, Derived)`, `(Derived, Secret)`, `(Secret, Clean)`, `(Secret, Derived)`, `(Secret, Secret)`
**When**: `join_taint(a, b)` and `join_taint(b, a)` are compared
**Then**: Both return the same result

### test_build_object_with_single_secret_field_joins_to_secret
**Given**: A `BuildObject` node with two fields: first `Clean`, second `Secret`
**When**: `build_object_with_taint` is called
**Then**: The returned taint is `Taint::Secret`

### test_build_object_with_single_derived_field_joins_to_derived
**Given**: A `BuildObject` node with two fields: first `Clean`, second `DerivedFromSecret`
**When**: `build_object_with_taint` is called
**Then**: The returned taint is `Taint::DerivedFromSecret`

### test_build_list_with_mixed_taints_joins_to_secret
**Given**: A `BuildList` node with items carrying `[Clean, DerivedFromSecret, Secret]`
**When**: `build_list_with_taint` is called
**Then**: The returned taint is `Taint::Secret`

### test_build_list_empty_returns_clean
**Given**: A `BuildList` node with an empty items list
**When**: `build_list_with_taint` is called
**Then**: The returned taint is `Taint::Clean`

### test_build_object_empty_returns_clean
**Given**: A `BuildObject` node with an empty fields list
**When**: `build_object_with_taint` is called
**Then**: The returned taint is `Taint::Clean`

### test_expression_load_const_always_returns_clean_taint
**Given**: An expression that only loads a constant (no slot reads)
**When**: `eval_expr_with_store` is called
**Then**: The returned taint is `Taint::Clean` regardless of constant value

### test_expression_multiple_load_slots_join_all_taints
**Given**: An expression that loads 4 slots with taints `[Clean, DerivedFromSecret, Clean, Secret]`
**When**: `eval_expr_with_store` is called
**Then**: The returned taint is `Taint::Secret`

### test_copy_slot_rejects_uninitialized_source
**Given**: A `Copy` node where the source slot has never been written
**When**: `copy_slot` executes
**Then**: `EngineError::SlotUninitialized` is returned and destination slot is unchanged

### test_finish_run_increments_executed_counter
**Given**: A `Finish` node with an initialized result slot
**When**: `finish_run` executes successfully
**Then**: The run's `executed` counter is incremented by 1

---

## Contract Verification Tests

### test_precondition_runframe_initialized_before_eval_expr
**Given**: A `RunFrame` with an uninitialized slot at index 0
**When**: `eval_expr_node` is called with a slot index of 0 as input
**Then**: `EngineError::SlotUninitialized` propagates from `eval_expr_with_store`

### test_precondition_slot_index_in_bounds_for_write_slot_with_taint
**Given**: A `RunFrame` with `slot_count = 3`
**When**: `write_slot_with_taint(SlotIdx(99), value, taint)` is called
**Then**: `EngineError::SlotOutOfBounds { slot: SlotIdx(99) }` is returned

### test_postcondition_eval_expr_output_slot_has_joined_taint
**Given**: A `RunFrame` with two slots: slot 0 carries `Taint::Secret`, slot 1 carries `Taint::Clean`
**And**: An expression that loads both slots 0 and 1 and adds them
**When**: `eval_expr_with_store` is called and the output is written to slot 2
**Then**: Slot 2 carries `Taint::Secret`

### test_postcondition_build_object_joins_all_field_taints
**Given**: A `RunFrame` with field slots carrying `[Clean, DerivedFromSecret]`
**When**: `build_object_with_taint` is called
**Then**: The returned object taint is `Taint::DerivedFromSecret`

### test_postcondition_build_list_joins_all_item_taints
**Given**: A `RunFrame` with item slots carrying `[Clean, Clean, Secret]`
**When**: `build_list_with_taint` is called
**Then**: The returned list taint is `Taint::Secret`

### test_postcondition_finish_signal_taint_equals_slot_taint
**Given**: A `RunFrame` with result slot carrying `Taint::DerivedFromSecret`
**When**: `finish_run` is called
**Then**: `EngineSignal::Finished(value, Taint::DerivedFromSecret)` is returned

### test_postcondition_copy_slot_preserves_taint_exactly
**Given**: A `RunFrame` with source slot carrying `Taint::Secret`
**When**: `copy_slot` writes to destination slot
**Then**: Destination slot taint equals `Taint::Secret`

### test_invariant_slot_and_taint_arrays_always_synced
**Given**: A `RunFrame` after multiple `write_slot_with_taint` calls
**When**: All initialized slots are read back with both `read_slot` and `read_taint`
**Then**: For every slot index, the taint matches what was written and the slot has a value

### test_invariant_taint_never_decreases_without_reinitialize
**Given**: A `RunFrame` where slot 0 has been written with `Taint::Secret`
**When**: A subsequent operation writes to slot 0 without calling `reinitialize`
**Then**: The taint of slot 0 can only stay at `Secret` or go to `DerivedFromSecret` if explicitly re-tainted, but never to `Clean`

### test_invariant_join_taint_commutative_proven_by_exhaustive_test
**Given**: All 9 ordered pairs of the 3 taint levels
**When**: `join_taint(a, b)` and `join_taint(b, a)` are computed for each pair
**Then**: Both results are equal for all pairs

### test_invariant_join_taint_associative_proven_by_exhaustive_test
**Given**: All 27 ordered triples of the 3 taint levels
**When**: `join_taint(join_taint(a, b), c)` and `join_taint(a, join_taint(b, c))` are computed
**Then**: Both results are equal for all triples

### test_invariant_no_tainted_wrapper_around_tainted_content
**Given**: A `BuildObject` with fields carrying `[Clean, Secret]`
**When**: `build_object_with_taint` is called
**Then**: The returned taint is `Secret` (not `Clean`); it is structurally impossible to produce a `Clean`-tainted container

### test_invariant_no_tainted_wrapper_around_tainted_list_content
**Given**: A `BuildList` with items carrying `[DerivedFromSecret]`
**When**: `build_list_with_taint` is called
**Then**: The returned taint is `DerivedFromSecret` (not `Clean`)

---

## Given-When-Then Scenarios

### Scenario 1: Tainted secret value flows through expression into object field
**Given**: A `RunFrame` where slot 0 holds a `Secret`-tainted integer value and slot 1 holds a `Clean`-tainted string value
**And**: A `BuildObject` node with fields `[(field_a, SlotIdx(0)), (field_b, SlotIdx(1))]`
**When**: `build_object_with_taint` executes
**Then**:
- The returned object taint is `Taint::Secret`
- The output slot receives `SlotValue::Object(handle)` with `Taint::Secret`
- Field `field_a` carries `Taint::Secret` in its `ObjectField::taint`
- Field `field_b` carries `Taint::Clean` in its `ObjectField::taint`
- No `Clean`-tainted wrapper is created around the secret-containing object

### Scenario 2: Tainted value flows through expression evaluation
**Given**: A `RunFrame` where slot 0 carries `DerivedFromSecret` and slot 1 carries `Clean`
**And**: An expression that loads slot 0 and slot 1, computes `slot0 + slot1`
**When**: `eval_expr_with_store` executes
**Then**:
- The returned taint is `Taint::DerivedFromSecret` (join of `DerivedFromSecret` and `Clean`)
- The output slot is written with `Taint::DerivedFromSecret`
- No `Clean` value can result from a computation involving a `DerivedFromSecret` input

### Scenario 3: Clean expression result remains clean through finish
**Given**: A `RunFrame` where the result slot carries `Taint::Clean`
**And**: A `Finish` node targeting that slot
**When**: `finish_run` executes
**Then**:
- `EngineSignal::Finished(value, Taint::Clean)` is returned
- The value is identical to what was in the result slot
- The taint is `Clean` (not promoted)

### Scenario 4: DerivedFromSecret finish result is allowed but triggers redaction
**Given**: A `RunFrame` where the result slot carries `Taint::DerivedFromSecret`
**And**: A `Finish` node targeting that slot
**When**: `finish_run` executes
**Then**:
- `EngineSignal::Finished(value, Taint::DerivedFromSecret)` is returned
- The `DerivedFromSecret` taint is preserved through the signal
- The runtime UI/log/action boundary must redact or reject the output per policy

### Scenario 5: Choose with tainted branch condition selects correct branch
**Given**: A `ChooseSlot` node with two branches: condition slot 0 is `Bool(true)` with `Taint::Secret`, condition slot 1 is `Bool(false)` with `Taint::Clean`
**And**: `otherwise = Some(StepIdx(9))`
**When**: `choose_slot_branch` executes
**Then**:
- PC is set to `branch[0].target` (first matching branch)
- No taint is accumulated or emitted by the choose operation itself
- The selected branch's subsequent execution will propagate taint

### Scenario 6: Choose falls through to otherwise when all branches false
**Given**: A `ChooseSlot` node with two branches: condition slot 0 is `Bool(false)`, condition slot 1 is `Bool(false)`
**And**: `otherwise = Some(StepIdx(9))`
**When**: `choose_slot_branch` executes
**Then**:
- PC is set to `StepIdx(9)` (the otherwise target)
- `EngineSignal::Continue` is returned

### Scenario 7: Action completion preserves taint across action boundary
**Given**: A `resume_action_completion` call with `output_taint = Taint::Secret`
**When**: The function writes the output to the designated slot
**Then**:
- The output slot receives `Taint::Secret`
- The returned `ActionJournalEvent::Completed` carries `output_taint = Taint::Secret`
- The taint is preserved through the journal record and replay

### Scenario 8: Uninitialized slot read returns typed error not panic
**Given**: A `RunFrame` where slot 0 has never been written
**When**: `read_taint(SlotIdx::ZERO)` is called
**Then**:
- `CoreError::SlotUninitialized { slot: SlotIdx::ZERO }` is returned
- No panic, unwrap, or expect is invoked
- The frame is not mutated

### Scenario 9: Slot index out of bounds returns typed error not panic
**Given**: A `RunFrame` with `slot_count = 2`
**When**: `read_taint(SlotIdx(99))` is called
**Then**:
- `CoreError::SlotOutOfBounds { slot: SlotIdx(99) }` is returned
- No panic, unwrap, or expect is invoked

### Scenario 10: Copy slot from tainted source preserves taint exactly
**Given**: A `RunFrame` where source slot carries `Taint::DerivedFromSecret`
**And**: The destination slot is uninitialized
**When**: `copy_slot` executes with `source = SlotIdx(0)` and `output = SlotIdx(1)`
**Then**:
- Destination slot value equals source slot value
- Destination slot taint equals `Taint::DerivedFromSecret`
- Source slot is unchanged
