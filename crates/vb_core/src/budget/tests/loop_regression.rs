// =========================================================================
// Security regression tests: loop-aware step counting
// =========================================================================

/// Verifies that a ForEachStart loop multiplies body steps by the limit.
/// Workflow: ForEachStart(limit=5, body=1, done=2) -> Nop -> Finish
/// Expected: 1 (header) + 5 * 1 (body * iterations) + 1 (Finish) = 7
#[test]
fn budget_foreach_loop_multiplies_body_steps() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 7);

    assert!(
        budget.is_some(),
        "for-each loop should multiply body steps by limit"
    );
}

/// Verifies that a RepeatStart loop multiplies body steps by max_attempts.
/// Workflow: RepeatStart(max=3, body=1, done=2) -> Nop -> Finish
/// Expected: 1 (header) + 3 * 1 (body * attempts) + 1 (Finish) = 5
#[test]
fn budget_repeat_loop_multiplies_body_steps() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 5);

    assert!(
        budget.is_some(),
        "repeat loop should multiply body steps by max_attempts"
    );
}

/// Verifies nested loop step counting multiplies correctly.
/// Outer ForEachStart(limit=10, body=1, done=4)
///   Inner ForEachStart(limit=10, body=2, done=3)
///     Nop (node 2)
///   ForEachJoin (node 3)
/// ForEachJoin (node 4)
///
/// Inner: body_count=1, product = 1*10=10, inner total = 1+10+1 = 12.
/// Outer body region: node1=12, node2=1, node3=1 = 14.
/// Wait — body_count counts distinct nodes in the region, inner ForEachStart(1) itself
/// is counted as 1, then its body is 1*10=10, then ForEachJoin(3) is 1. So region for
/// outer body = 1 + 10 + 1 = 12. Outer: product = 12*10 = 120, total = 1+120+1 = 122.
#[test]
fn budget_nested_loop_multiplies_correctly() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
            None,
            None,
        ),
        node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(3),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
            Some(SlotIdx::new(4)),
            None,
        ),
        node(
            StepIdx::new(4),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
            Some(SlotIdx::new(5)),
            None,
        ),
    ];
    let contract = test_contract(5, 6);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 122 && b.max_nesting_depth == 2);

    assert!(
        budget.is_some(),
        "nested loop should multiply step counts at each nesting level"
    );
}

/// Security regression: a workflow with loops that previously appeared as a
/// small step count (unique nodes only) now correctly reports the worst-case
/// multiplied count, which should exceed the default policy.
///
/// ForEachStart(limit=1000, body=1, done=2) -> Nop -> Finish
/// Old: 3 steps (passed policy).
/// New: 1 + 1000*1 + 1 = 1002 steps (still under 1M policy, but realistic).
#[test]
fn budget_large_loop_counted_realistically() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1000,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 1002);

    assert!(budget.is_some(), "large loop should count 1002 steps not 3");

    // The default policy (1M steps) should accept this
    let budget_val = budget.as_ref().unwrap();
    assert!(
        BoundednessPolicy::DEFAULT.validate(budget_val).is_ok(),
        "1002 steps should be within default policy"
    );
}

/// Verifies that a CollectStart loop multiplies body steps by the limit.
#[test]
fn budget_collect_loop_multiplies_body_steps() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 5,
                page_size: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 7);

    assert!(
        budget.is_some(),
        "collect loop should multiply body steps by limit"
    );
}

/// Verifies that StepBudget clamps values above MAX_STEP_BUDGET.
#[test]
fn step_budget_clamps_above_max() {
    let budget = StepBudget::new(crate::limits::MAX_STEP_BUDGET + 100);
    assert_eq!(
        budget.remaining(),
        crate::limits::MAX_STEP_BUDGET,
        "budget should be clamped to MAX_STEP_BUDGET"
    );
}

/// Verifies that StepBudget::MAX equals MAX_STEP_BUDGET.
#[test]
fn step_budget_max_equals_limit() {
    assert_eq!(
        StepBudget::MAX.remaining(),
        crate::limits::MAX_STEP_BUDGET,
        "StepBudget::MAX should equal MAX_STEP_BUDGET"
    );
}

/// Verifies that StepBudget zero budget exhausts immediately.
#[test]
fn step_budget_zero_exhausts_immediately() {
    let mut budget = StepBudget::new(0);
    let result = budget.try_take();
    assert!(
        result.is_ok() && result.as_ref().map_err(|_| "").unwrap() == &false,
        "zero budget should return Ok(false) immediately"
    );
}
