use super::*;
use vb_core::workflow::SlotFlowFailure;

fn ordered() -> vb_core::workflow::WorkflowParts {
    parts(
        vec![
            node(
                0,
                K::TogetherStart {
                    branches: vec![step(1), step(3)].into_boxed_slice(),
                    join: step(5),
                },
                Some(0),
                None,
            ),
            node(
                1,
                K::TogetherBranch {
                    branch: 0.into(),
                    entry: step(2),
                    join: step(5),
                    accumulator: slot(0),
                },
                None,
                None,
            ),
            set(2, 0, 1, 3),
            node(
                3,
                K::TogetherBranch {
                    branch: 1.into(),
                    entry: step(4),
                    join: step(5),
                    accumulator: slot(0),
                },
                Some(1),
                None,
            ),
            node(4, K::Copy { source: slot(1) }, Some(1), Some(5)),
            node(
                5,
                K::TogetherJoin {
                    branch_count: 2.into(),
                    accumulator: slot(0),
                },
                Some(1),
                Some(6),
            ),
            finish(6, 1),
        ],
        2,
    )
}

#[test]
fn together_executes_ordered_bodies_before_join() -> TestResult {
    CompiledWorkflow::try_from_parts(ordered())?;
    Ok(())
}

#[test]
fn together_first_branch_cannot_observe_unwritten_output_metadata() -> TestResult {
    let mut p = ordered();
    if let Some(first) = p.nodes.get_mut(1) {
        first.output = Some(slot(1)).into();
    }
    rejected(p, SlotFlowFailure::UninitializedRead)
}

#[test]
fn together_branch_metadata_does_not_write_previous_result() -> TestResult {
    let mut p = ordered();
    if let Some(first) = p.nodes.get_mut(2) {
        first.kind = K::Nop;
    }
    rejected(p, SlotFlowFailure::UninitializedRead)
}

#[test]
fn together_join_requires_last_result_before_overwrite() -> TestResult {
    let mut p = ordered();
    if let Some(join) = p.nodes.get_mut(5) {
        join.output = Some(slot(2)).into();
    }
    p.slot_count = 3;
    rejected(p, SlotFlowFailure::UninitializedRead)
}
