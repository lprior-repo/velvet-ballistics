use super::*;
use vb_core::workflow::{InputSlotKind, SlotFlowFailure};

fn for_each(result: u16) -> vb_core::workflow::WorkflowParts {
    let mut p = parts(
        vec![
            node(
                0,
                K::ForEachStart {
                    input: slot(0),
                    item_slot: slot(2),
                    limit: 4.into(),
                    body: step(1),
                    done: step(3),
                },
                Some(1),
                None,
            ),
            node(1, K::Copy { source: slot(2) }, Some(3), Some(2)),
            node(
                2,
                K::ForEachNext {
                    iterator_slot: slot(1),
                    body: step(1),
                    done: step(3),
                },
                Some(2),
                None,
            ),
            finish(3, result),
        ],
        4,
    );
    declare(&mut p, 0, InputSlotKind::List);
    p
}

#[test]
fn empty_for_each_does_not_initialize_body_result() -> TestResult {
    rejected(for_each(3), SlotFlowFailure::UninitializedRead)
}

#[test]
fn empty_for_each_still_initializes_iterator_and_body_binds_item() -> TestResult {
    CompiledWorkflow::try_from_parts(for_each(1))?;
    Ok(())
}

#[test]
fn repeat_runs_body_before_done() -> TestResult {
    let p = parts(
        vec![
            node(
                0,
                K::RepeatStart {
                    max_attempts: 2.into(),
                    body: step(1),
                    done: step(3),
                },
                Some(0),
                None,
            ),
            set(1, 0, 1, 2),
            node(
                2,
                K::RepeatCheck {
                    attempt_slot: slot(0),
                    done: step(3),
                },
                None,
                Some(1),
            ),
            finish(3, 1),
        ],
        2,
    );
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

#[test]
fn backedge_cannot_initialize_first_activation_read() -> TestResult {
    let p = parts(
        vec![
            node(
                0,
                K::RepeatStart {
                    max_attempts: 2.into(),
                    body: step(1),
                    done: step(4),
                },
                Some(0),
                None,
            ),
            node(1, K::Copy { source: slot(1) }, Some(2), Some(2)),
            set(2, 0, 1, 3),
            node(
                3,
                K::RepeatCheck {
                    attempt_slot: slot(0),
                    done: step(4),
                },
                None,
                Some(1),
            ),
            finish(4, 1),
        ],
        3,
    );
    rejected(p, SlotFlowFailure::UninitializedRead)
}

#[test]
fn collect_page_has_no_zero_iteration_exit() -> TestResult {
    let mut p = parts(
        vec![
            node(
                0,
                K::CollectStart {
                    source: slot(0),
                    limit: 4.into(),
                    page_size: 2,
                    body: step(1),
                    done: step(4),
                },
                Some(1),
                None,
            ),
            node(
                1,
                K::CollectPage {
                    collector_slot: slot(1),
                    body: step(2),
                    done: step(4),
                },
                None,
                None,
            ),
            node(2, K::Copy { source: slot(1) }, Some(2), Some(3)),
            node(
                3,
                K::CollectNext {
                    collector_slot: slot(1),
                    body: step(1),
                    done: step(4),
                },
                None,
                None,
            ),
            finish(4, 1),
        ],
        3,
    );
    declare(&mut p, 0, InputSlotKind::List);
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

#[test]
fn multi_step_reduce_has_initialized_loop_carried_dependency() -> TestResult {
    let mut p = parts(
        vec![
            node(
                0,
                K::ReduceStart {
                    limit: 65_535,
                    input: slot(0),
                    accumulator: slot(2),
                    initial: vb_core::ids::ConstIdx::new(0),
                    body: step(1),
                    done: step(5),
                },
                Some(1),
                None,
            ),
            node(1, K::Copy { source: slot(2) }, Some(3), Some(2)),
            node(2, K::Copy { source: slot(3) }, Some(4), Some(3)),
            node(3, K::Copy { source: slot(4) }, Some(2), Some(4)),
            node(
                4,
                K::ReduceNext {
                    iterator_slot: slot(1),
                    accumulator: slot(2),
                    body: step(1),
                    done: step(5),
                },
                Some(0),
                None,
            ),
            node(
                5,
                K::ReduceFinish {
                    accumulator: slot(2),
                },
                Some(5),
                Some(6),
            ),
            finish(6, 5),
        ],
        6,
    );
    declare(&mut p, 0, InputSlotKind::List);
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

fn nested_for_each(result: u16) -> vb_core::workflow::WorkflowParts {
    let mut p = parts(
        vec![
            node(
                0,
                K::ForEachStart {
                    input: slot(0),
                    item_slot: slot(4),
                    limit: 4.into(),
                    body: step(1),
                    done: step(5),
                },
                Some(2),
                None,
            ),
            node(
                1,
                K::ForEachStart {
                    input: slot(1),
                    item_slot: slot(5),
                    limit: 4.into(),
                    body: step(2),
                    done: step(4),
                },
                Some(3),
                None,
            ),
            node(2, K::Copy { source: slot(5) }, Some(6), Some(3)),
            node(
                3,
                K::ForEachNext {
                    iterator_slot: slot(3),
                    body: step(2),
                    done: step(4),
                },
                Some(5),
                None,
            ),
            node(
                4,
                K::ForEachNext {
                    iterator_slot: slot(2),
                    body: step(1),
                    done: step(5),
                },
                Some(4),
                None,
            ),
            finish(5, result),
        ],
        7,
    );
    declare(&mut p, 0, InputSlotKind::List);
    declare(&mut p, 1, InputSlotKind::List);
    p
}

#[test]
fn nested_loop_activation_preserves_declared_sources_and_binds_inner_items() -> TestResult {
    vb_core::workflow::validate_slot_flow(&nested_for_each(2))?;
    Ok(())
}

#[test]
fn nested_empty_exits_do_not_initialize_inner_body_outputs() -> TestResult {
    let result = vb_core::workflow::validate_slot_flow(&nested_for_each(6));
    match result {
        Err(error) => {
            assert_eq!(error.step, step(5));
            assert_eq!(error.slot, Some(slot(6)));
            assert_eq!(error.reason, SlotFlowFailure::UninitializedRead);
        }
        Ok(()) => return Err("nested empty exits initialized body output".into()),
    }
    Ok(())
}
