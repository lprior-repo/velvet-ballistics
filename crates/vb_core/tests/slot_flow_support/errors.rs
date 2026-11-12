use super::*;
use vb_core::workflow::{ErrorRouting, SlotFlowFailure};

pub(super) fn action(result: u16) -> vb_core::workflow::WorkflowParts {
    let mut do_node = node(
        1,
        K::Do {
            action: vb_core::ids::ActionId::new(0),
            input: slot(0),
        },
        Some(1),
        Some(2),
    );
    do_node.routing.error = ErrorRouting::Handler {
        handler: step(3),
        error_slot: Some(slot(2)),
    };
    parts(
        vec![set(0, 0, 0, 1), do_node, finish(2, 1), finish(3, result)],
        3,
    )
}

#[test]
fn action_success_materializes_dynamic_output() -> TestResult {
    CompiledWorkflow::try_from_parts(action(2))?;
    Ok(())
}

#[test]
fn action_failure_has_error_value_and_no_success_output() -> TestResult {
    rejected(action(1), SlotFlowFailure::UninitializedRead)
}

#[test]
fn recovery_does_not_assume_a_partial_write_occurred() -> TestResult {
    let mut write = set(0, 0, 0, 1);
    write.routing.error = ErrorRouting::Handler {
        handler: step(2),
        error_slot: None,
    };
    rejected(
        parts(vec![write, finish(1, 0), finish(2, 0)], 1),
        SlotFlowFailure::UninitializedRead,
    )
}

#[test]
fn retry_first_visit_initializes_its_absent_counter() -> TestResult {
    let p = parts(
        vec![
            node(
                0,
                K::RetryCheck {
                    policy_slot: slot(0),
                    body: step(1),
                    exhausted: step(2),
                },
                None,
                None,
            ),
            finish(1, 0),
            finish(2, 0),
        ],
        1,
    );
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

#[test]
fn wait_output_metadata_does_not_initialize_a_slot() -> TestResult {
    let p = parts(
        vec![
            set(0, 0, 0, 1),
            node(
                1,
                K::WaitUntil {
                    deadline_slot: slot(0),
                },
                Some(1),
                Some(2),
            ),
            finish(2, 1),
        ],
        2,
    );
    rejected(p, SlotFlowFailure::UninitializedRead)
}

#[test]
fn handler_sees_possible_partial_overwrite_types() -> TestResult {
    use vb_core::ids::ExprIdx;
    use vb_core::value::ConstValue;
    use vb_core::workflow::{ExprOp as Op, ExprProgram};
    let mut write = node(
        1,
        K::EvalExpr {
            expr: ExprIdx::new(0),
        },
        Some(0),
        Some(2),
    );
    write.routing.error = ErrorRouting::Handler {
        handler: step(3),
        error_slot: None,
    };
    let mut p = parts(
        vec![
            set(0, 0, 0, 1),
            write,
            finish(2, 0),
            node(
                3,
                K::EvalExpr {
                    expr: ExprIdx::new(1),
                },
                Some(1),
                Some(4),
            ),
            finish(4, 1),
        ],
        2,
    );
    p.constants = vec![ConstValue::I64(1), ConstValue::Bool(true)].into_boxed_slice();
    p.expressions = vec![
        ExprProgram::try_from_ops(
            vec![Op::LoadConst(vb_core::ids::ConstIdx::new(1))].into_boxed_slice(),
        )?,
        ExprProgram::try_from_ops(
            vec![
                Op::LoadSlot(slot(0)),
                Op::LoadConst(vb_core::ids::ConstIdx::new(0)),
                Op::Add,
            ]
            .into_boxed_slice(),
        )?,
    ]
    .into_boxed_slice();
    rejected(p, SlotFlowFailure::IncompatibleType)
}

pub(super) fn action_with_legacy_wrapper(result: u16) -> vb_core::workflow::WorkflowParts {
    let mut p = action(result);
    let mut wrapper = node(
        4,
        K::ErrorHandler {
            body: step(1),
            handler: step(5),
            error_slot: Some(slot(4)).into(),
        },
        None,
        None,
    );
    wrapper.routing.error = ErrorRouting::Handler {
        handler: step(5),
        error_slot: Some(slot(4)),
    };
    let mut nodes = p.nodes.into_vec();
    nodes.push(wrapper);
    nodes.push(finish(5, 0));
    if let Some(first) = nodes.first_mut() {
        first.routing.next = Some(step(4)).into();
    }
    p.nodes = nodes.into_boxed_slice();
    p.slot_count = 5;
    p
}

#[test]
fn direct_error_slot_takes_precedence_over_legacy_wrapper() -> TestResult {
    CompiledWorkflow::try_from_parts(action_with_legacy_wrapper(2))?;
    Ok(())
}

#[test]
fn legacy_error_slot_is_not_assigned_when_direct_slot_exists() -> TestResult {
    rejected(
        action_with_legacy_wrapper(4),
        SlotFlowFailure::UninitializedRead,
    )
}

#[test]
fn legacy_error_slot_remains_fallback_when_direct_slot_is_absent() -> TestResult {
    let mut p = action_with_legacy_wrapper(4);
    if let Some(action) = p.nodes.get_mut(1) {
        action.routing.error = ErrorRouting::Handler {
            handler: step(3),
            error_slot: None,
        };
    }
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

fn guarded_action(result: u16) -> vb_core::workflow::WorkflowParts {
    let mut action = node(
        1,
        K::Do {
            action: vb_core::ids::ActionId::new(0),
            input: slot(1),
        },
        Some(2),
        Some(3),
    );
    action.routing.error = ErrorRouting::Handler {
        handler: step(4),
        error_slot: Some(slot(3)),
    };
    let mut p = parts(
        vec![
            choose(0, 1, 2),
            action,
            node(2, K::Jump { target: step(3) }, None, None),
            finish(3, result),
            finish(4, 3),
        ],
        4,
    );
    declare(&mut p, 0, vb_core::workflow::InputSlotKind::Bool);
    declare(&mut p, 1, vb_core::workflow::InputSlotKind::I64);
    p
}

#[test]
fn guarded_action_does_not_initialize_output_on_skipped_branch() -> TestResult {
    rejected(guarded_action(2), SlotFlowFailure::UninitializedRead)
}

#[test]
fn guarded_action_can_join_when_continuation_uses_an_initialized_value() -> TestResult {
    CompiledWorkflow::try_from_parts(guarded_action(1))?;
    Ok(())
}
