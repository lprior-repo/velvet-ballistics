use super::*;
use vb_core::workflow::{ErrorRouting, SlotFlowError, SlotFlowFailure};

fn fixture_for_family(family: u8, numeric: bool) -> TestResultParts {
    let mut parts = fixture(numeric, false)?;
    let body = StepIdx::new(3);
    let done = StepIdx::new(5);
    let cursor = SlotIdx::new(2);
    let accumulator = SlotIdx::new(1);
    let source = SlotIdx::ZERO;
    let destination = SlotIdx::new(3);
    let counter = SlotIdx::new(1);
    let mut nodes = parts.nodes.into_vec();
    nodes.remove(6);
    for (index, node) in nodes.iter_mut().enumerate() {
        node.id = StepIdx::new(u16::try_from(index)?);
    }
    let start = nodes.get_mut(2).ok_or("start")?;
    start.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(6),
        error_slot: None,
    };
    start.kind = match family {
        0 => K::ForEachStart {
            input: source,
            item_slot: counter,
            limit: 2,
            body,
            done,
        },
        1 => K::ReduceStart {
            input: source,
            accumulator,
            initial: ConstIdx::new(1),
            limit: 2,
            body,
            done,
        },
        2 => K::CollectStart {
            source,
            limit: 2,
            page_size: 1,
            body,
            done,
        },
        3 => K::RepeatStart {
            max_attempts: 1,
            body,
            done,
        },
        _ => return Err("family".into()),
    };
    if family == 3 {
        start.output = Some(counter).into();
    }
    if family == 1 {
        nodes.get_mut(3).ok_or("body")?.kind = K::Copy { source: cursor };
        nodes.get_mut(3).ok_or("body")?.output = Some(accumulator).into();
    }
    let control = nodes.get_mut(4).ok_or("control")?;
    control.kind = match family {
        0 => K::ForEachNext {
            iterator_slot: cursor,
            body,
            done,
        },
        1 => K::ReduceNext {
            iterator_slot: cursor,
            accumulator,
            body,
            done,
        },
        2 => K::CollectNext {
            collector_slot: cursor,
            body,
            done,
        },
        3 => K::RepeatCheck {
            attempt_slot: counter,
            done,
        },
        _ => return Err("family".into()),
    };
    if family == 1 {
        control.output = Some(source).into();
    }
    if family >= 2 {
        control.output = None.into();
    }
    if family == 3 {
        control.routing.next = Some(body).into();
    }
    let finish = nodes.get_mut(5).ok_or("finish")?;
    finish.kind = match family {
        0 => K::ForEachJoin { output: cursor },
        1 => K::ReduceFinish { accumulator },
        2 => K::CollectFinish {
            collector_slot: cursor,
        },
        3 => K::RepeatFinish { result: source },
        _ => return Err("family".into()),
    };
    finish.routing.next = None.into();
    finish.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(6),
        error_slot: None,
    };
    let handler = nodes.get_mut(6).ok_or("handler")?;
    if !numeric {
        handler.kind = K::Copy {
            source: destination,
        };
    }
    handler.routing.next = Some(StepIdx::new(7)).into();
    parts.nodes = nodes.into_boxed_slice();
    parts.expressions = Box::from([ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(destination),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ]))?]);
    let mut constraint = CompiledInputConstraint::required();
    constraint.default_value = Some(SlotValue::I64(42));
    parts.input_slots = Box::from([CompiledInputSlot {
        slot: destination,
        kind: InputSlotKind::I64,
        constraint,
    }]);
    Ok(parts)
}

type TestResultParts = Result<WorkflowParts, Box<dyn std::error::Error>>;

#[test]
fn public_constructor_retains_postwrite_failure_kinds() -> TestResult {
    for family in 0..4 {
        let parts = fixture_for_family(family, true)?;
        assert_eq!(
            CompiledWorkflow::try_from_parts(parts).as_ref().map(|_| ()),
            Err(&WorkflowError::SlotFlow(SlotFlowError {
                step: StepIdx::new(6),
                slot: None,
                reason: SlotFlowFailure::IncompatibleType
            }))
        );
    }
    Ok(())
}

#[test]
fn public_runtime_late_transition_failure_keeps_actual_written_value() -> TestResult {
    for (family, list, writes) in [(0, 2, 3), (1, 2, 3), (2, 0, 4), (3, 0, 1)] {
        execute_recovery_with_cursor_writes(
            fixture_for_family(family, false)?,
            5,
            SlotValue::List(vb_core::ListId::new(list)),
            writes,
        )?;
    }
    Ok(())
}
