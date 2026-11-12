#[test]
fn compiled_node_kind_do_constructs() {
    let kind = CompiledNodeKind::Do {
        action: ActionId::new(10),
        input: SlotIdx::new(0),
    };
    let CompiledNodeKind::Do { action, input } = kind else {
        panic!("expected Do");
    };
    assert_eq!(action, ActionId::new(10));
    assert_eq!(input, SlotIdx::new(0));
}

#[test]
fn compiled_node_kind_choose_constructs() {
    let kind = CompiledNodeKind::Choose {
        branches: Box::new([]),
        otherwise: Some(StepIdx::new(1)).into(),
    };
    let CompiledNodeKind::Choose { otherwise, .. } = kind else {
        panic!("expected Choose");
    };
    assert_eq!(otherwise.step(), Some(StepIdx::new(1)));
}

#[test]
fn compiled_node_kind_choose_slot_constructs() {
    let kind = CompiledNodeKind::ChooseSlot {
        branches: Box::new([]),
        otherwise: Some(StepIdx::new(1)).into(),
    };
    let CompiledNodeKind::ChooseSlot { otherwise, .. } = kind else {
        panic!("expected ChooseSlot");
    };
    assert_eq!(otherwise.step(), Some(StepIdx::new(1)));
}

#[test]
fn compiled_node_kind_for_each_start_constructs() {
    let kind = CompiledNodeKind::ForEachStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: 100.into(),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::ForEachStart { limit, .. } = kind else {
        panic!("expected ForEachStart");
    };
    assert_eq!(limit.get(), 100);
}

#[test]
fn compiled_node_kind_for_each_next_constructs() {
    let kind = CompiledNodeKind::ForEachNext {
        iterator_slot: SlotIdx::new(2),
        body: StepIdx::new(1),
        done: StepIdx::new(3),
    };
    let CompiledNodeKind::ForEachNext { iterator_slot, .. } = kind else {
        panic!("expected ForEachNext");
    };
    assert_eq!(iterator_slot, SlotIdx::new(2));
}

#[test]
fn compiled_node_kind_for_each_join_constructs() {
    let kind = CompiledNodeKind::ForEachJoin {
        output: SlotIdx::new(5),
    };
    let CompiledNodeKind::ForEachJoin { output } = kind else {
        panic!("expected ForEachJoin");
    };
    assert_eq!(output, SlotIdx::new(5));
}

#[test]
fn compiled_node_kind_together_start_constructs() {
    let kind = CompiledNodeKind::TogetherStart {
        branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
        join: StepIdx::new(3),
    };
    let CompiledNodeKind::TogetherStart { branches, join } = kind else {
        panic!("expected TogetherStart");
    };
    assert_eq!(branches.len(), 2);
    assert_eq!(join, StepIdx::new(3));
}

#[test]
fn compiled_node_kind_together_branch_constructs() {
    let kind = CompiledNodeKind::TogetherBranch {
        branch: 0.into(),
        entry: StepIdx::new(1),
        join: StepIdx::new(3),
        accumulator: SlotIdx::new(0),
    };
    let CompiledNodeKind::TogetherBranch { branch, .. } = kind else {
        panic!("expected TogetherBranch");
    };
    assert_eq!(branch.get(), 0);
}

#[test]
fn compiled_node_kind_together_join_constructs() {
    let kind = CompiledNodeKind::TogetherJoin {
        branch_count: 2.into(),
        accumulator: SlotIdx::new(0),
    };
    let CompiledNodeKind::TogetherJoin { branch_count, .. } = kind else {
        panic!("expected TogetherJoin");
    };
    assert_eq!(branch_count.get(), 2);
}

#[test]
fn compiled_node_kind_collect_start_constructs() {
    let kind = CompiledNodeKind::CollectStart {
        source: SlotIdx::new(0),
        limit: 50.into(),
        page_size: 10,
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::CollectStart {
        limit, page_size, ..
    } = kind
    else {
        panic!("expected CollectStart");
    };
    assert_eq!(limit.get(), 50);
    assert_eq!(page_size, 10);
}

#[test]
fn compiled_node_kind_collect_page_constructs() {
    let kind = CompiledNodeKind::CollectPage {
        collector_slot: SlotIdx::new(3),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::CollectPage { collector_slot, .. } = kind else {
        panic!("expected CollectPage");
    };
    assert_eq!(collector_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_collect_next_constructs() {
    let kind = CompiledNodeKind::CollectNext {
        collector_slot: SlotIdx::new(3),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::CollectNext { collector_slot, .. } = kind else {
        panic!("expected CollectNext");
    };
    assert_eq!(collector_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_collect_finish_constructs() {
    let kind = CompiledNodeKind::CollectFinish {
        collector_slot: SlotIdx::new(3),
    };
    let CompiledNodeKind::CollectFinish { collector_slot } = kind else {
        panic!("expected CollectFinish");
    };
    assert_eq!(collector_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_reduce_start_constructs() {
    let kind = CompiledNodeKind::ReduceStart {
        limit: 65_535,
        input: SlotIdx::new(0),
        accumulator: SlotIdx::new(1),
        initial: ConstIdx::new(0),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::ReduceStart { accumulator, .. } = kind else {
        panic!("expected ReduceStart");
    };
    assert_eq!(accumulator, SlotIdx::new(1));
}

#[test]
fn compiled_node_kind_reduce_next_constructs() {
    let kind = CompiledNodeKind::ReduceNext {
        iterator_slot: SlotIdx::new(2),
        accumulator: SlotIdx::new(1),
        body: StepIdx::new(1),
        done: StepIdx::new(3),
    };
    let CompiledNodeKind::ReduceNext { iterator_slot, .. } = kind else {
        panic!("expected ReduceNext");
    };
    assert_eq!(iterator_slot, SlotIdx::new(2));
}

#[test]
fn compiled_node_kind_reduce_finish_constructs() {
    let kind = CompiledNodeKind::ReduceFinish {
        accumulator: SlotIdx::new(1),
    };
    let CompiledNodeKind::ReduceFinish { accumulator } = kind else {
        panic!("expected ReduceFinish");
    };
    assert_eq!(accumulator, SlotIdx::new(1));
}

#[test]
fn compiled_node_kind_repeat_start_constructs() {
    let kind = CompiledNodeKind::RepeatStart {
        max_attempts: 5.into(),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::RepeatStart { max_attempts, .. } = kind else {
        panic!("expected RepeatStart");
    };
    assert_eq!(max_attempts.get(), 5);
}

#[test]
fn compiled_node_kind_repeat_attempt_constructs() {
    let kind = CompiledNodeKind::RepeatAttempt {
        attempt_slot: SlotIdx::new(3),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::RepeatAttempt { attempt_slot, .. } = kind else {
        panic!("expected RepeatAttempt");
    };
    assert_eq!(attempt_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_repeat_check_constructs() {
    let kind = CompiledNodeKind::RepeatCheck {
        attempt_slot: SlotIdx::new(3),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::RepeatCheck { attempt_slot, .. } = kind else {
        panic!("expected RepeatCheck");
    };
    assert_eq!(attempt_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_repeat_finish_constructs() {
    let kind = CompiledNodeKind::RepeatFinish {
        result: SlotIdx::new(0),
    };
    let CompiledNodeKind::RepeatFinish { result } = kind else {
        panic!("expected RepeatFinish");
    };
    assert_eq!(result, SlotIdx::new(0));
}

#[test]
fn compiled_node_kind_wait_until_constructs() {
    let kind = CompiledNodeKind::WaitUntil {
        deadline_slot: SlotIdx::new(4),
    };
    let CompiledNodeKind::WaitUntil { deadline_slot } = kind else {
        panic!("expected WaitUntil");
    };
    assert_eq!(deadline_slot, SlotIdx::new(4));
}

#[test]
fn compiled_node_kind_wait_event_constructs() {
    let kind = CompiledNodeKind::WaitEvent {
        event: SlotIdx::new(5),
        timeout_slot: Some(SlotIdx::new(6)).into(),
    };
    let CompiledNodeKind::WaitEvent {
        event,
        timeout_slot,
    } = kind
    else {
        panic!("expected WaitEvent");
    };
    assert_eq!(event, SlotIdx::new(5));
    assert_eq!(timeout_slot.slot(), Some(SlotIdx::new(6)));
}

#[test]
fn compiled_node_kind_wait_event_without_timeout_constructs() {
    let kind = CompiledNodeKind::WaitEvent {
        event: SlotIdx::new(5),
        timeout_slot: None.into(),
    };
    let CompiledNodeKind::WaitEvent { timeout_slot, .. } = kind else {
        panic!("expected WaitEvent");
    };
    assert!(timeout_slot.slot().is_none());
}

#[test]
fn compiled_node_kind_ask_constructs() {
    let kind = CompiledNodeKind::Ask {
        prompt: SlotIdx::new(7),
        timeout_slot: Some(SlotIdx::new(8)).into(),
    };
    let CompiledNodeKind::Ask { prompt, .. } = kind else {
        panic!("expected Ask");
    };
    assert_eq!(prompt, SlotIdx::new(7));
}

#[test]
fn compiled_node_kind_ask_resume_constructs() {
    let kind = CompiledNodeKind::AskResume {
        answer: SlotIdx::new(9),
    };
    let CompiledNodeKind::AskResume { answer } = kind else {
        panic!("expected AskResume");
    };
    assert_eq!(answer, SlotIdx::new(9));
}

#[test]
fn compiled_node_kind_retry_check_constructs() {
    let kind = CompiledNodeKind::RetryCheck {
        policy_slot: SlotIdx::new(10),
        body: StepIdx::new(1),
        exhausted: StepIdx::new(2),
    };
    let CompiledNodeKind::RetryCheck { policy_slot, .. } = kind else {
        panic!("expected RetryCheck");
    };
    assert_eq!(policy_slot, SlotIdx::new(10));
}

#[test]
fn compiled_node_kind_error_handler_constructs() {
    let kind = CompiledNodeKind::ErrorHandler {
        body: StepIdx::new(1),
        handler: StepIdx::new(2),
        error_slot: None.into(),
    };
    let CompiledNodeKind::ErrorHandler {
        body,
        handler,
        error_slot,
    } = kind
    else {
        panic!("expected ErrorHandler");
    };
    assert_eq!(error_slot.slot(), None);
    assert_eq!(body, StepIdx::new(1));
    assert_eq!(handler, StepIdx::new(2));
}

#[test]
fn compiled_node_kind_jump_constructs() {
    let kind = CompiledNodeKind::Jump {
        target: StepIdx::new(3),
    };
    let CompiledNodeKind::Jump { target } = kind else {
        panic!("expected Jump");
    };
    assert_eq!(target, StepIdx::new(3));
}

#[test]
fn compiled_node_kind_finish_constructs() {
    let kind = CompiledNodeKind::Finish {
        result: SlotIdx::new(0),
    };
    let CompiledNodeKind::Finish { result } = kind else {
        panic!("expected Finish");
    };
    assert_eq!(result, SlotIdx::new(0));
}

// =========================================================================
// ExprOp variant construction tests
// =========================================================================

#[test]
fn expr_op_load_slot_constructs() {
    let op = ExprOp::LoadSlot(SlotIdx::new(42));
    assert_eq!(op, ExprOp::LoadSlot(SlotIdx::new(42)));
}

#[test]
fn expr_op_load_const_constructs() {
    let op = ExprOp::LoadConst(ConstIdx::new(7));
    assert_eq!(op, ExprOp::LoadConst(ConstIdx::new(7)));
}

#[test]
fn expr_op_load_accessor_constructs() {
    let op = ExprOp::LoadAccessor(AccessorIdx::new(3));
    assert_eq!(op, ExprOp::LoadAccessor(AccessorIdx::new(3)));
}

#[test]
fn expr_op_comparison_variants_are_distinct() {
    let ops = [
        ExprOp::Eq,
        ExprOp::NotEq,
        ExprOp::Gt,
        ExprOp::Gte,
        ExprOp::Lt,
        ExprOp::Lte,
    ];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_boolean_variants_are_distinct() {
    assert_ne!(ExprOp::And, ExprOp::Or);
    assert_ne!(ExprOp::And, ExprOp::Not);
    assert_ne!(ExprOp::Or, ExprOp::Not);
}

#[test]
fn expr_op_arithmetic_variants_are_distinct() {
    let ops = [ExprOp::Add, ExprOp::Sub, ExprOp::Mul, ExprOp::Div];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_string_helpers_are_distinct() {
    let ops = [
        ExprOp::Contains,
        ExprOp::StartsWith,
        ExprOp::EndsWith,
        ExprOp::Has,
    ];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_unary_helpers_are_distinct() {
    let ops = [ExprOp::Exists, ExprOp::Length, ExprOp::Empty];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_collection_helpers_are_distinct() {
    let ops = [
        ExprOp::Append,
        ExprOp::AppendIf,
        ExprOp::Merge,
        ExprOp::Sum,
        ExprOp::Count,
        ExprOp::Unique,
    ];
    assert_pairwise_distinct(&ops);
}

// =========================================================================
// ExprProgram valid construction tests
// =========================================================================

#[test]
fn expr_program_single_load_slot_succeeds() -> Result<(), String> {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_single_load_const_succeeds() -> Result<(), String> {
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_single_load_accessor_succeeds() -> Result<(), String> {
    let ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_eq_reduces_stack() -> Result<(), String> {
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Eq,
    ]
    .into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 2 {
        return Err(format!("expected max_stack 2, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_not_preserves_stack() -> Result<(), String> {
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_empty_ops_rejected() -> Result<(), String> {
    let ops = Box::new([]) as Box<[ExprOp]>;
    match ExprProgram::try_from_ops(ops) {
        Err(CoreError::ExpressionStackUnderflow) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expr_program_try_from_parts_matches_computed_stack() -> Result<(), String> {
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice();
    let program = ExprProgram::try_from_parts(ops, 1).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_try_from_parts_rejects_empty_ops() -> Result<(), String> {
    let ops = Box::new([]) as Box<[ExprOp]>;
    match ExprProgram::try_from_parts(ops, 0) {
        Err(CoreError::ExpressionStackUnderflow) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// check_expr_stack_bound edge cases
// =========================================================================

#[test]
fn check_expr_stack_bound_single_load_returns_one() -> Result<(), String> {
    let ops = [ExprOp::LoadSlot(SlotIdx::new(0))];
    let result = check_expr_stack_bound(&ops, 64).map_err(|e| e.to_string())?;
    if result != 1 {
        return Err(format!("expected 1, got {result}"));
    }
    Ok(())
}

#[test]
fn check_expr_stack_bound_rejects_zero_capacity() -> Result<(), String> {
    let ops = [ExprOp::LoadSlot(SlotIdx::new(0))];
    match check_expr_stack_bound(&ops, 0) {
        Err(CoreError::ExpressionStackOverflow { max: 0 }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// AccessorProgram and PathSegment construction tests
// =========================================================================

#[test]
fn accessor_program_empty_path_constructs() {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([]),
    };
    assert_eq!(accessor.root, SlotIdx::new(0));
    assert!(accessor.path.is_empty());
}

#[test]
fn accessor_program_field_path_constructs() {
    let accessor = AccessorProgram {
        root: SlotIdx::new(1),
        path: vec![PathSegment::Field(SymbolId::new(42))].into_boxed_slice(),
    };
    assert_eq!(accessor.root, SlotIdx::new(1));
    assert_eq!(accessor.path.len(), 1);
    assert_eq!(accessor.path[0], PathSegment::Field(SymbolId::new(42)));
}

#[test]
fn accessor_program_index_path_constructs() {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Index(7)].into_boxed_slice(),
    };
    assert_eq!(accessor.path[0], PathSegment::Index(7));
}

#[test]
fn accessor_program_mixed_path_constructs() {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![
            PathSegment::Field(SymbolId::new(1)),
            PathSegment::Index(0),
            PathSegment::Field(SymbolId::new(2)),
        ]
        .into_boxed_slice(),
    };
    assert_eq!(accessor.path.len(), 3);
}

#[test]
fn path_segment_field_equality() {
    assert_eq!(
        PathSegment::Field(SymbolId::new(5)),
        PathSegment::Field(SymbolId::new(5))
    );
    assert_ne!(
        PathSegment::Field(SymbolId::new(5)),
        PathSegment::Field(SymbolId::new(6))
    );
}

#[test]
fn path_segment_index_equality() {
    assert_eq!(PathSegment::Index(3), PathSegment::Index(3));
    assert_ne!(PathSegment::Index(3), PathSegment::Index(4));
}

#[test]
fn path_segment_field_and_index_are_distinct() {
    assert_ne!(PathSegment::Field(SymbolId::new(0)), PathSegment::Index(0));
}

// =========================================================================
// CompiledNode construction tests
// =========================================================================

#[test]
fn compiled_node_constructs_with_all_fields() {
    let node = crate::workflow::node_with_handler(
        StepIdx::new(5),
        CompiledNodeKind::Nop,
        Some(SlotIdx::new(3)),
        Some(StepIdx::new(6)),
        StepIdx::new(10),
        Some(SlotIdx::new(7)),
    );
    assert_eq!(node.id, StepIdx::new(5));
    assert_eq!(node.output, NodeOutput::Slot(SlotIdx::new(3)));
    assert_eq!(node.routing.next, NodeContinuation::Step(StepIdx::new(6)));
    assert_eq!(
        node.routing.error,
        ErrorRouting::Handler {
            handler: StepIdx::new(10),
            error_slot: Some(SlotIdx::new(7)),
        }
    );
}

#[test]
fn compiled_node_optional_fields_can_be_none() {
    let node = crate::workflow::node(StepIdx::new(0), CompiledNodeKind::Nop, None, None);
    assert!(node.output.slot().is_none());
    assert!(matches!(node.routing.next, NodeContinuation::End));
    assert!(matches!(
        node.routing.error,
        crate::workflow::ErrorRouting::None
    ));
    assert!(node.routing.error.error_slot().is_none());
}

// =========================================================================
// CompiledWorkflow in-bounds accessor tests
// =========================================================================

#[test]
fn compiled_workflow_node_returns_some_for_valid_index() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    let node = workflow.node(StepIdx::new(0));
    if node.is_none() {
        return Err(String::from("expected Some for valid step index"));
    }
    Ok(())
}

#[test]
fn compiled_workflow_constant_returns_some_for_valid_index() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    let constant = workflow.constant(ConstIdx::new(0));
    if constant.is_none() {
        return Err(String::from("expected Some for valid constant index"));
    }
    assert_eq!(constant, Some(&ConstValue::Null));
    Ok(())
}

#[test]
fn compiled_workflow_name_returns_name() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    if workflow.name() != "resource_case" {
        return Err(format!(
            "expected 'resource_case', got '{}'",
            workflow.name()
        ));
    }
    Ok(())
}

#[test]
fn compiled_workflow_entry_returns_entry_step() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    if workflow.entry() != StepIdx::new(0) {
        return Err(format!("expected StepIdx(0), got {:?}", workflow.entry()));
    }
    Ok(())
}

#[test]
fn compiled_workflow_node_count_returns_correct_count() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    if workflow.node_count() != 1 {
        return Err(format!("expected 1 node, got {}", workflow.node_count()));
    }
    Ok(())
}
