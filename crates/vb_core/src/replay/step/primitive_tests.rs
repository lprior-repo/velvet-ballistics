use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::replay::{ReplayEngine, ReplayError};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use super::replay_step;

fn make_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
) -> Result<CompiledWorkflow, CoreError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "test_primitive_replay".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: vec![].into(),
        accessors: vec![].into(),
        constants: constants.into(),
        slot_count: 8,
        symbols_count: 1,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| CoreError::InternalInvariantViolation {
        reason: "test workflow validation failed",
    })
}

fn node_at(plan: &CompiledWorkflow, step: u16) -> Result<&CompiledNode, CoreError> {
    plan.node(StepIdx::new(step))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "test node missing",
        })
}

fn new_run(plan: &CompiledWorkflow) -> Result<RunFrame, CoreError> {
    RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
}

fn replay_err_to_core(error: ReplayError) -> CoreError {
    match error {
        ReplayError::StepNotFound { step } => CoreError::InvalidProgramCounter { step },
        ReplayError::SlotNotAvailable { slot } => CoreError::SlotOutOfBounds { slot },
        ReplayError::ExpressionEvalFailed { step } => CoreError::InvalidProgramCounter { step },
        ReplayError::NonDeterministicStep { step, .. } => CoreError::InvalidProgramCounter { step },
        ReplayError::Internal { reason } => CoreError::InternalInvariantViolation { reason },
    }
}

fn write_list(
    store: &mut ValueStore,
    run: &mut RunFrame,
    slot: SlotIdx,
    values: &[SlotValue],
) -> Result<(), CoreError> {
    let id = store.insert_list(values.to_vec().into_boxed_slice())?;
    run.write_slot(slot, SlotValue::List(id))
}

fn finish_node(step: u16) -> CompiledNode {
    crate::workflow::node(
        StepIdx::new(step),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )
}

// ---- ForEachStart ----

#[test]
fn replay_for_each_start_binds_first_and_routes_body() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(
        &mut store,
        &mut run,
        SlotIdx::new(0),
        &[SlotValue::I64(10), SlotValue::I64(20)],
    )?;
    run.write_taint(SlotIdx::new(0), Taint::Secret)?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ForEachStart should Continue to body",
            });
        }
    }
    if *run.read_slot(SlotIdx::new(1))? != SlotValue::I64(10) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "item slot should hold first element",
        });
    }
    if run.read_taint(SlotIdx::new(1))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "item taint should propagate from input",
        });
    }
    Ok(())
}

#[test]
fn replay_for_each_start_empty_routes_done() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(&mut store, &mut run, SlotIdx::new(0), &[])?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "empty ForEachStart should Continue to done",
            });
        }
    }
    match *run.read_slot(SlotIdx::new(2))? {
        SlotValue::List(id) => {
            if !store.list(id)?.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "empty ForEachStart should write empty list",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "iterator slot should hold a list",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_for_each_start_limit_exceeded_errors() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 1,
                    body: StepIdx::new(1),
                    done: StepIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(
        &mut store,
        &mut run,
        SlotIdx::new(0),
        &[SlotValue::I64(1), SlotValue::I64(2)],
    )?;

    let result = replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "for_each item count exceeds limit during replay"
            })
        ),
        "over-limit ForEachStart must fail"
    );
    Ok(())
}

fn for_each_loop_plan() -> Result<CompiledWorkflow, CoreError> {
    make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(2),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(4),
                },
                Some(SlotIdx::new(5)),
                Some(StepIdx::new(4)),
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(5),
                },
                None,
                None,
            ),
        ],
        vec![],
    )
}

#[test]
fn replay_for_each_next_advances_then_exits() -> Result<(), CoreError> {
    let plan = for_each_loop_plan()?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(
        &mut store,
        &mut run,
        SlotIdx::new(0),
        &[SlotValue::I64(10), SlotValue::I64(20)],
    )?;

    replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    replay_step(node_at(&plan, 1)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    let action =
        replay_step(node_at(&plan, 2)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ForEachNext should Continue to body while items remain",
            });
        }
    }
    if *run.read_slot(SlotIdx::new(1))? != SlotValue::I64(20) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "ForEachNext should bind the second element",
        });
    }
    replay_step(node_at(&plan, 1)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    let action =
        replay_step(node_at(&plan, 2)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(3) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "exhausted ForEachNext should Continue to done",
        }),
    }
}

#[test]
fn replay_for_each_next_empty_state_routes_done() -> Result<(), CoreError> {
    let plan = for_each_loop_plan()?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(&mut store, &mut run, SlotIdx::new(2), &[])?;

    let action =
        replay_step(node_at(&plan, 2)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(3) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "empty-state ForEachNext should Continue to done",
        }),
    }
}

#[test]
fn replay_for_each_join_materializes_results() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(3),
                },
                Some(SlotIdx::new(4)),
                Some(StepIdx::new(1)),
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(&mut store, &mut run, SlotIdx::new(3), &[SlotValue::I64(30)])?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ForEachJoin should Continue to next",
            });
        }
    }
    match (
        *run.read_slot(SlotIdx::new(4))?,
        *run.read_slot(SlotIdx::new(3))?,
    ) {
        (SlotValue::List(out), SlotValue::List(materialized)) if out == materialized => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "ForEachJoin should copy materialized list to output",
        }),
    }
}

// ---- TogetherStart ----

#[test]
fn replay_together_start_creates_accumulator_and_forks() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::TogetherStart {
                    branches: Box::from([StepIdx::new(1)]),
                    join: StepIdx::new(1),
                },
                Some(SlotIdx::new(7)),
                None,
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "TogetherStart should Continue to first branch",
            });
        }
    }
    if run.parallel_in_flight() != 1 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "TogetherStart should reserve one parallel slot",
        });
    }
    match *run.read_slot(SlotIdx::new(7))? {
        SlotValue::List(id) => {
            if !store.list(id)?.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "TogetherStart should write an empty accumulator",
                });
            }
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "accumulator slot should hold a list",
        }),
    }
}

#[test]
fn replay_together_start_empty_branches_errors() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::TogetherStart {
                    branches: Box::from([]),
                    join: StepIdx::new(1),
                },
                Some(SlotIdx::new(7)),
                None,
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();

    let result = replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "together_start requires at least one branch during replay"
            })
        ),
        "branchless TogetherStart must fail"
    );
    Ok(())
}

// ---- TogetherBranch ----

#[test]
fn replay_together_branch_first_jumps_without_append() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::TogetherBranch {
                    branch: 0,
                    entry: StepIdx::new(1),
                    join: StepIdx::new(1),
                    accumulator: SlotIdx::new(7),
                },
                None,
                None,
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(&mut store, &mut run, SlotIdx::new(7), &[])?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "first TogetherBranch should Continue to entry",
        }),
    }
}

#[test]
fn replay_together_branch_appends_previous_result() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::TogetherBranch {
                    branch: 1,
                    entry: StepIdx::new(1),
                    join: StepIdx::new(1),
                    accumulator: SlotIdx::new(7),
                },
                Some(SlotIdx::new(6)),
                None,
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(&mut store, &mut run, SlotIdx::new(7), &[SlotValue::I64(11)])?;
    run.write_slot(SlotIdx::new(6), SlotValue::I64(22))?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "TogetherBranch should Continue to entry",
            });
        }
    }
    match *run.read_slot(SlotIdx::new(7))? {
        SlotValue::List(id) => {
            if store.list(id)? != [SlotValue::I64(11), SlotValue::I64(22)] {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "accumulator should append the previous result",
                });
            }
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "accumulator slot should hold a list",
        }),
    }
}

// ---- TogetherJoin ----

#[test]
fn replay_together_join_merges_last_result_and_releases() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::TogetherJoin {
                    branch_count: 2,
                    accumulator: SlotIdx::new(7),
                },
                Some(SlotIdx::new(6)),
                Some(StepIdx::new(1)),
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    run.add_parallel_in_flight(2)?;
    write_list(&mut store, &mut run, SlotIdx::new(7), &[SlotValue::I64(11)])?;
    run.write_slot(SlotIdx::new(6), SlotValue::I64(22))?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "TogetherJoin should Continue to next",
            });
        }
    }
    if run.parallel_in_flight() != 0 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "TogetherJoin should release parallel slots",
        });
    }
    match *run.read_slot(SlotIdx::new(6))? {
        SlotValue::List(id) => {
            if store.list(id)? != [SlotValue::I64(11), SlotValue::I64(22)] {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "TogetherJoin should merge the last result",
                });
            }
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "join output should hold a list",
        }),
    }
}

#[test]
fn replay_together_join_list_result_keeps_accumulator() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::TogetherJoin {
                    branch_count: 1,
                    accumulator: SlotIdx::new(7),
                },
                Some(SlotIdx::new(6)),
                Some(StepIdx::new(1)),
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    run.add_parallel_in_flight(1)?;
    write_list(&mut store, &mut run, SlotIdx::new(7), &[SlotValue::I64(11)])?;
    write_list(&mut store, &mut run, SlotIdx::new(6), &[SlotValue::I64(22)])?;

    replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match (
        *run.read_slot(SlotIdx::new(6))?,
        *run.read_slot(SlotIdx::new(7))?,
    ) {
        (SlotValue::List(out), SlotValue::List(acc)) if out == acc => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "list result should keep the accumulator unchanged",
        }),
    }
}

// ---- ReduceStart ----

fn reduce_head_plan() -> Result<CompiledWorkflow, CoreError> {
    make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ReduceStart {
                    input: SlotIdx::new(0),
                    accumulator: SlotIdx::new(3),
                    initial: ConstIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            finish_node(2),
        ],
        vec![ConstValue::I64(0)],
    )
}

#[test]
fn replay_reduce_start_inits_accumulator_and_binds_first() -> Result<(), CoreError> {
    let plan = reduce_head_plan()?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(
        &mut store,
        &mut run,
        SlotIdx::new(0),
        &[SlotValue::I64(5), SlotValue::I64(6)],
    )?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ReduceStart should Continue to body",
            });
        }
    }
    if *run.read_slot(SlotIdx::new(3))? != SlotValue::I64(0) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "accumulator should hold the initial constant",
        });
    }
    if *run.read_slot(SlotIdx::new(0))? != SlotValue::I64(5) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "input slot should be rebound to the first element",
        });
    }
    Ok(())
}

#[test]
fn replay_reduce_start_empty_routes_done() -> Result<(), CoreError> {
    let plan = reduce_head_plan()?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(&mut store, &mut run, SlotIdx::new(0), &[])?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(2) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "empty ReduceStart should Continue to done",
            });
        }
    }
    if *run.read_slot(SlotIdx::new(3))? != SlotValue::I64(0) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "accumulator should still hold the initial constant",
        });
    }
    Ok(())
}

// ---- ReduceNext / ReduceFinish ----

#[test]
fn replay_reduce_next_advances_then_exits() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ReduceStart {
                    input: SlotIdx::new(0),
                    accumulator: SlotIdx::new(3),
                    initial: ConstIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::ReduceNext {
                    iterator_slot: SlotIdx::new(2),
                    accumulator: SlotIdx::new(3),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::ReduceFinish {
                    accumulator: SlotIdx::new(3),
                },
                Some(SlotIdx::new(4)),
                Some(StepIdx::new(4)),
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(4),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(0)],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    write_list(
        &mut store,
        &mut run,
        SlotIdx::new(0),
        &[SlotValue::I64(5), SlotValue::I64(6)],
    )?;

    replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    replay_step(node_at(&plan, 1)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    let action =
        replay_step(node_at(&plan, 2)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ReduceNext should Continue to body while items remain",
            });
        }
    }
    if *run.read_slot(SlotIdx::new(1))? != SlotValue::I64(6) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "ReduceNext should bind the second element",
        });
    }
    replay_step(node_at(&plan, 1)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    let action =
        replay_step(node_at(&plan, 2)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(3) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "exhausted ReduceNext should Continue to done",
        }),
    }
}

#[test]
fn replay_reduce_finish_writes_accumulator() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ReduceFinish {
                    accumulator: SlotIdx::new(3),
                },
                Some(SlotIdx::new(4)),
                Some(StepIdx::new(1)),
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(3), SlotValue::I64(11))?;
    run.write_taint(SlotIdx::new(3), Taint::Secret)?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ReduceFinish should Continue to next",
            });
        }
    }
    if *run.read_slot(SlotIdx::new(4))? != SlotValue::I64(11) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "ReduceFinish should copy the accumulator to output",
        });
    }
    if run.read_taint(SlotIdx::new(4))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "ReduceFinish should propagate accumulator taint",
        });
    }
    Ok(())
}
