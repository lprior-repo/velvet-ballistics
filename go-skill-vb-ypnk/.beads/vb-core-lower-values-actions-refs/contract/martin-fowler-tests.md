# Martin Fowler Test Plan — vb-core-lower-values-actions-refs

## Happy Path Tests

### test_lower_slot_reference_direct_slot
**Given**: a valid slot reference string `"$slot.7"`
**When**: `lower_slot_reference("$slot.7", &mut Vec::new())` is called
**Then**: returns `Ok(ExprOp::LoadSlot(SlotIdx::new(7)))` and does not push to accessors

### test_lower_accessor_reference_numeric_nested_path
**Given**: a valid nested slot reference string `"$slots.2.0.3"`
**When**: `lower_slot_reference("$slots.2.0.3", &mut Vec::new())` is called
**Then**: returns `Ok(ExprOp::LoadAccessor(AccessorIdx::new(0)))` and accessors contains exactly one `AccessorProgram { root: SlotIdx::new(2), path: [Index(0), Index(3)] }`

### test_lower_accessor_reference_single_index
**Given**: a valid single-index accessor string `"$slot.4.12"`
**When**: `lower_slot_reference("$slot.4.12", &mut Vec::new())` is called
**Then**: returns `Ok(ExprOp::LoadAccessor(AccessorIdx::new(0)))` with `AccessorProgram { root: SlotIdx::new(4), path: [Index(12)] }`

### test_compile_expr_binary_arithmetic
**Given**: parsed expression `"1 + 2 * 3"`
**When**: `compile_expr_to_bytecode` is called
**Then**: returns `ExprProgram` with ops `[LoadConst, LoadConst, LoadConst, Mul, Add]` and `max_stack == 3`

### test_compile_expr_unary_not_and_negation
**Given**: parsed expression `"not -1"`
**When**: `compile_expr_to_bytecode` is called
**Then**: returns `ExprProgram` with ops `[LoadConst, LoadConst, Sub, Not]` and `max_stack == 2`

### test_compile_expr_boolean_equality
**Given**: parsed expression `"true == false"`
**When**: `compile_expr_to_bytecode` is called
**Then**: returns `ExprProgram` with ops `[LoadConst, LoadConst, Eq]` and `max_stack == 2`

### test_compile_expr_helper_two_args
**Given**: parsed expression `"contains(1, 2)"`
**When**: `compile_expr_to_bytecode` is called
**Then**: returns `ExprProgram` with ops `[LoadConst, LoadConst, Contains]` and `max_stack == 2`

### test_slot_compiler_new_empty
**Given**: a new `SlotCompiler`
**When**: `slot_count()` is called
**Then**: returns `Ok(0)`

### test_slot_compiler_record_slot_tracks_max
**Given**: a `SlotCompiler` with slots 3, 7, and 1 recorded (in any order)
**When**: `slot_count()` is called
**Then**: returns `Ok(8)` (max 7 + 1)

### test_slot_compiler_push_constant_returns_index
**Given**: a `SlotCompiler` with one existing constant
**When**: `push_constant(ConstValue::I64(42))` is called
**Then**: returns `Ok(ConstIdx::new(1))` and constants.len() == 2

### test_slot_compiler_build_parts_sets_symbols_count_zero
**Given**: a `SlotCompiler` with recorded slots and constants
**When**: `build_parts("test", digest)` is called
**Then**: returns `WorkflowParts` with `symbols_count == 0` and all Box conversions correct

### test_lower_do_records_input_slot
**Given**: a `SlotCompiler` and `lower_do(id, action, input, output, next, builder)`
**When**: builder's state is inspected after the call
**Then**: `builder.record_slot(input)` was called

### test_lower_choose_validates_branch_route
**Given**: non-empty branches and a valid `otherwise`
**When**: `lower_choose(id, branches, otherwise, builder)` is called
**Then**: returns `Ok(CompiledNode)` with `CompiledNodeKind::ChooseSlot`

### test_lower_wait_until_records_deadline_slot
**Given**: a `WaitKind::Until { deadline }` and a `SlotCompiler`
**When**: `lower_wait(id, kind, builder)` is called
**Then**: `builder.record_slot(deadline)` is called and returns correct `WaitUntil` node

---

## Error Path Tests

### test_rejects_references_until_accessor_table_exists
**Given**: an expression with a slot reference `"$input.value"`
**When**: `compile_expr_to_bytecode` (without accessor resolver) is called
**Then**: returns `Err(CompileError::ExpressionLoweringUnsupported { feature: "accessor references" })`

### test_rejects_non_numeric_slot_index
**Given**: a slot reference `"$slot.abc"`
**When**: `lower_slot_reference` is called
**Then**: returns `Err(CompileError::UnknownReferenceName { kind: "slot", ... })`

### test_rejects_unknown_reference_root
**Given**: a reference `"$unknown.5"`
**When**: `lower_slot_reference` is called
**Then**: returns `Err(CompileError::UnknownReferenceRoot { root: "unknown", ... })`

### test_rejects_field_accessor_without_symbol_table
**Given**: an expression `"$slot.1.name"` (field accessor)
**When**: `compile_expr_to_bytecode_with_accessors` is called
**Then**: returns `Err(CompileError::UnsupportedAccessorReference { root: "slot.1", path: "name" })`

### test_rejects_field_accessor_after_list_index
**Given**: an expression `"$slots.1.0.name"`
**When**: `compile_expr_to_bytecode_with_accessors` is called
**Then**: returns `Err(CompileError::UnsupportedAccessorReference { root: "slots.1", path: "0.name" })`

### test_rejects_empty_accessor_segment
**Given**: an expression `"$slot.1..0"`
**When**: `compile_expr_to_bytecode_with_accessors` is called
**Then**: returns `Err(CompileError::UnsupportedAccessorReference { root: "slot.1", path: ".0" })` with exact diagnostic code `UNSUPPORTED_ACCESSOR_REFERENCE`

### test_text_literal_rejected_as_unsupported
**Given**: a text literal expression `"\"hello\""`
**When**: `compile_expr_to_bytecode` is called
**Then**: returns `Err(CompileError::ExpressionLoweringUnsupported { feature: "text constants" })`

### test_empty_string_literal_rejected_as_unsupported
**Given**: an empty string literal expression `""`
**When**: `compile_expr_to_bytecode` is called
**Then**: returns `Err(CompileError::ExpressionLoweringUnsupported { feature: "text constants" })`

### test_helper_zero_args_rejected
**Given**: a helper call `"contains()"`
**When**: `compile_expr_to_bytecode` is called
**Then**: returns `Err(CompileError::ExpressionHelperArity { helper: "contains", expected: 2, actual: 0 })`

### test_helper_too_many_args_rejected
**Given**: a helper call `"append_if(1, 2)"`
**When**: `compile_expr_to_bytecode` is called
**Then**: returns `Err(CompileError::ExpressionHelperArity { helper: "append_if", expected: 3, actual: 2 })`

### test_constant_pool_overflow_rejected
**Given**: 65536 existing constants in the pool
**When**: `push_constant` is called with one more
**Then**: returns `Err(CompileError::Workflow(WorkflowError::ConstOutOfBounds))`

### test_slot_index_out_of_range
**Given**: a slot index computation that overflows `i64`
**When**: `SlotCompiler::slot_count()` is called
**Then**: returns `Err(CompileError::SlotIndexOutOfRange { value: i64::MAX })`

### test_together_branch_count_exceeds_u16
**Given**: more than 65535 branches in `lower_together`
**When**: `lower_together` is called
**Then**: returns `Err(CompileError::PrimitiveLoweringLimitExceeded { primitive: "together", field: "branches", value: >65535, limit: 65535 })`

---

## Edge Case Tests

### test_zero_integer_constant_lowers_correctly
**Given**: expression `"0"`
**When**: `compile_expr_to_bytecode` is called
**Then**: constants contains `ConstValue::I64(0)` and single `LoadConst` op

### test_near_max_integer_constant_lowers_correctly
**Given**: expression `i64::MAX.to_string()`
**When**: `compile_expr_to_bytecode` is called
**Then**: constants contains `ConstValue::I64(i64::MAX)` and single `LoadConst` op

### test_negative_one_integer_lowers_correctly
**Given**: expression `"-1"`
**When**: `compile_expr_to_bytecode` is called
**Then**: constants contains `[ConstValue::I64(0), ConstValue::I64(1)]` with 3 ops ending in `Sub`

### test_deeply_nested_arithmetic_produces_valid_bytecode
**Given**: expression `"1 + 2 + 3 + 4 + 5"`
**When**: `compile_expr_to_bytecode` is called
**Then**: constants.len() == 5, ops.len() == 9, max_stack >= 2

### test_null_constant_lowers_to_const
**Given**: expression `"null"`
**When**: `compile_expr_to_bytecode` is called
**Then**: constants contains `ConstValue::Null` and single `LoadConst` op

### test_true_boolean_lowers_to_const
**Given**: expression `"true"`
**When**: `compile_expr_to_bytecode` is called
**Then**: constants contains `ConstValue::Bool(true)` and single `LoadConst` op

### test_false_boolean_lowers_to_const
**Given**: expression `"false"`
**When**: `compile_expr_to_bytecode` is called
**Then**: constants contains `ConstValue::Bool(false)` and single `LoadConst` op

### test_parenthesized_expression_lowers_identically
**Given**: expressions `"1 + 2"` and `"(1 + 2)"`
**When**: both are compiled via `compile_expr_to_bytecode`
**Then**: both produce identical `ExprProgram`

### test_deeply_nested_unary_not_lowers_correctly
**Given**: expression `"not not true"`
**When**: `compile_expr_to_bytecode` is called
**Then**: ops.len() == 3 with two `Not` ops

### test_division_lowers_to_div_op
**Given**: expression `"10 / 2"`
**When**: `compile_expr_to_bytecode` is called
**Then**: ops contain `Div` as final op

### test_greater_than_lowers_to_gt_op
**Given**: expression `"5 > 3"`
**When**: `compile_expr_to_bytecode` is called
**Then**: ops contain `Gt` as final op

### test_and_operator_lowers_to_and_op
**Given**: expression `"true and false"`
**When**: `compile_expr_to_bytecode` is called
**Then**: ops contain `And` as final op

### test_or_operator_lowers_to_or_op
**Given**: expression `"true or false"`
**When**: `compile_expr_to_bytecode` is called
**Then**: ops contain `Or` as final op

### test_chained_comparison_operators_lowers_with_precedence
**Given**: expression `"1 < 2 and 3 > 0 or 4 >= 4"`
**When**: `compile_expr_to_bytecode` is called
**Then**: final op is `Or` with `Lt`, `And`, `Gt`, `Gte` in the ops

### test_exists_with_one_arg_succeeds
**Given**: expression `"exists(1)"`
**When**: `compile_expr_to_bytecode` is called
**Then**: ops contain `Exists` as final op

### test_sum_with_one_arg_succeeds
**Given**: expression `"sum(1)"`
**When**: `compile_expr_to_bytecode` is called
**Then**: ops contain `Sum` as final op

### test_merge_with_two_args_succeeds
**Given**: expression `"merge(1, 2)"`
**When**: `compile_expr_to_bytecode` is called
**Then**: ops contain `Merge` as final op

### test_append_if_with_three_args_succeeds
**Given**: expression `"append_if(1, 2, 3)"`
**When**: `compile_expr_to_bytecode` is called
**Then**: ops contain `AppendIf` as final op

---

## Contract Verification Tests

### test_precondition_slot_compiler_new_empty
**Given**: `SlotCompiler::new()`
**When**: inspected
**Then**: `max_slot == None` (verified via `slot_count() == 0`)

### test_precondition_valid_u16_slot_indices
**Given**: valid `"$slot.65535"`
**When**: `lower_slot_reference` is called
**Then**: returns `Ok(ExprOp::LoadSlot(SlotIdx::new(65535)))`

### test_postcondition_slot_reference_does_not_mutate_accessors
**Given**: `Vec::new()` as accessors and `"$slot.7"`
**When**: `lower_slot_reference` is called
**Then**: accessors remains empty

### test_postcondition_accessor_reference_pushes_exactly_one_program
**Given**: `Vec::new()` as accessors and `"$slots.2.0.3"`
**When**: `lower_slot_reference` is called
**Then**: accessors.len() == 1

### test_postcondition_bytecode_respects_max_stack
**Given**: a deeply nested expression
**When**: `compile_expr_to_bytecode` is called
**Then**: `result.max_stack <= MAX_EXPRESSION_STACK`

### test_postcondition_single_stack_result
**Given**: any valid expression
**When**: `compile_expr_to_bytecode` is called
**Then**: `result.ops` leaves exactly one value on the stack

### test_invariant_max_slot_preserved
**Given**: `SlotCompiler` with max recorded slot of 5
**When**: `slot_count()` is called
**Then**: returns `Ok(6)`

### test_invariant_accessor_program_path_all_numeric
**Given**: `lower_accessor_reference` for valid nested path
**When**: the result is inspected
**Then**: `AccessorProgram.path` contains only `PathSegment::Index(u32)`

### test_invariant_no_duplicate_step_indices
**Given**: multiple steps compiled via `lower_steps_to_ir`
**When**: the resulting `WorkflowParts.nodes` is inspected
**Then**: all `node.id` values are unique

---

## Integration Scenario Tests

### Scenario: Full expression lowering pipeline
**Given**: YAML workflow with expression `"$slot.1 + 2 > 3"`
**When**: compiled via `compile_workflow`
**Then**: expression is lowered to `ExprProgram` with correct ops and the workflow validates successfully

### Scenario: Invalid reference rejected at compile time
**Given**: YAML workflow with `"$unknown.5"`
**When**: `compile_workflow` is called
**Then**: returns `Err(CompileErrors(...))` with `UnknownReferenceRoot` diagnostic

### Scenario: Taint leak rejected at compile time
**Given**: YAML workflow where `secrets` field is used in `result`
**When**: `compile_workflow` is called
**Then**: returns `Err(CompileErrors(...))` with `SecretTaintLeak` diagnostic
