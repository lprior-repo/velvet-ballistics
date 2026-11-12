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
        reason: "test workflow validation failed".into(),
    })
}

fn node_at(plan: &CompiledWorkflow, step: u16) -> Result<&CompiledNode, CoreError> {
    plan.node(StepIdx::new(step))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "test node missing".into(),
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
        ReplayError::Internal { reason } => CoreError::InternalInvariantViolation {
            reason: reason.into(),
        },
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

// ---- RepeatStart ----

#[test]
fn replay_repeat_start_writes_packed_state() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::RepeatStart {
                    max_attempts: 2.into(),
                    body: StepIdx::new(1),
                    done: StepIdx::new(1),
                },
                Some(SlotIdx::new(0)),
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
                reason: "RepeatStart should Continue to body".into(),
            });
        }
    }
    match *run.read_slot(SlotIdx::new(0))? {
        SlotValue::I64(_) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "RepeatStart should write packed I64 state".into(),
        }),
    }
}

// ---- RepeatAttempt / RepeatCheck / RepeatFinish ----

#[test]
fn replay_repeat_attempt_validates_packed_state() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::RepeatStart {
                    max_attempts: 2.into(),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                Some(SlotIdx::new(0)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::RepeatAttempt {
                    attempt_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                None,
                None,
            ),
            finish_node(2),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();

    replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    let action =
        replay_step(node_at(&plan, 1)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "RepeatAttempt should Continue to body".into(),
        }),
    }
}

#[test]
fn replay_repeat_attempt_corrupt_state_errors() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::RepeatAttempt {
                    attempt_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(1),
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
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))?;

    let result = replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "invalid_repeat_state during replay"
            })
        ),
        "corrupt repeat state must fail"
    );
    Ok(())
}

fn repeat_loop_plan() -> Result<CompiledWorkflow, CoreError> {
    make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::RepeatStart {
                    max_attempts: 2.into(),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(0)),
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
                CompiledNodeKind::RepeatCheck {
                    attempt_slot: SlotIdx::new(0),
                    done: StepIdx::new(3),
                },
                None,
                Some(StepIdx::new(1)),
            ),
            finish_node(3),
        ],
        vec![],
    )
}

#[test]
fn replay_repeat_check_counts_attempts_then_routes_done() -> Result<(), CoreError> {
    let plan = repeat_loop_plan()?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();

    replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    replay_step(node_at(&plan, 1)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    let action =
        replay_step(node_at(&plan, 2)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "first RepeatCheck should Continue to body entry".into(),
            });
        }
    }
    replay_step(node_at(&plan, 1)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    let action =
        replay_step(node_at(&plan, 2)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(3) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "exhausted RepeatCheck should Continue to done".into(),
        }),
    }
}

#[test]
fn replay_repeat_finish_writes_result() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::RepeatFinish {
                    result: SlotIdx::new(3),
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
    run.write_slot(SlotIdx::new(3), SlotValue::I64(77))?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "RepeatFinish should Continue to next".into(),
            });
        }
    }
    if *run.read_slot(SlotIdx::new(4))? != SlotValue::I64(77) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "RepeatFinish should copy the result to output".into(),
        });
    }
    Ok(())
}

// ---- RetryCheck ----

#[test]
fn replay_retry_check_first_visit_routes_body_and_counts() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    exhausted: StepIdx::new(2),
                },
                None,
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
                reason: "first RetryCheck should Continue to body".into(),
            });
        }
    }
    if *run.read_slot(SlotIdx::new(0))? != SlotValue::I64(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "RetryCheck should write back attempt 1".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_retry_check_exhausted_routes_exhausted() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    exhausted: StepIdx::new(2),
                },
                None,
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
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(3))?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(2) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "exhausted RetryCheck should Continue to exhausted".into(),
            });
        }
    }
    if *run.read_slot(SlotIdx::new(0))? != SlotValue::I64(4) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "RetryCheck should still advance the counter on exhaustion".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_retry_check_overflow_errors() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    exhausted: StepIdx::new(2),
                },
                None,
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
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(65535))?;

    let result = replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "retry_attempt_overflow during replay"
            })
        ),
        "overflowing RetryCheck must fail"
    );
    Ok(())
}

// ---- ErrorHandler ----

#[test]
fn replay_error_handler_routes_body() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: None.into(),
                },
                None,
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
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "ErrorHandler should Continue to body".into(),
        }),
    }
}

// ---- AskResume ----

#[test]
fn replay_ask_resume_forwards_answer() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(1)),
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))?;
    run.write_taint(SlotIdx::new(0), Taint::Secret)?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "AskResume should Continue to next".into(),
            });
        }
    }
    if *run.read_slot(SlotIdx::new(1))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "AskResume should forward the answer to output".into(),
        });
    }
    if run.read_taint(SlotIdx::new(1))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "AskResume should propagate answer taint".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_ask_resume_missing_answer_errors() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(1)),
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();

    match replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "AskResume without an answer must fail".into(),
        }),
    }
}

#[test]
fn replay_ask_resume_missing_next_errors() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node_with_handler(
                StepIdx::new(0),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                None,
                StepIdx::new(1),
                None,
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))?;

    let result = replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "AskResume node missing next step"
            })
        ),
        "AskResume without next must fail"
    );
    Ok(())
}

#[test]
fn replay_ask_resume_without_output_continues() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(0),
                },
                None,
                Some(StepIdx::new(1)),
            ),
            finish_node(1),
        ],
        vec![],
    )?;
    let mut run = new_run(&plan)?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))?;

    let action =
        replay_step(node_at(&plan, 0)?, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    match action {
        crate::replay::ReplayAction::Continue(next) if next == StepIdx::new(1) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "AskResume without output should still Continue".into(),
        }),
    }
}

// ---- End-to-end: retry chain terminates via counter write-back ----

#[test]
fn replay_engine_retry_chain_terminates_at_exhausted() -> Result<(), CoreError> {
    let nop = |step: u16, next: u16| {
        crate::workflow::node(
            StepIdx::new(step),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(next)),
        )
    };
    let check = |step: u16, body: u16| {
        crate::workflow::node(
            StepIdx::new(step),
            CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(body),
                exhausted: StepIdx::new(7),
            },
            None,
            None,
        )
    };
    let plan = make_plan(
        vec![
            check(0, 1),
            nop(1, 2),
            check(2, 3),
            nop(3, 4),
            check(4, 5),
            nop(5, 6),
            check(6, 7),
            finish_node(7),
        ],
        vec![],
    )?;
    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let reached = engine
        .replay_up_to(StepIdx::new(7), &mut store)
        .map_err(replay_err_to_core)?;
    if reached != StepIdx::new(7) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "retry chain should terminate at the exhausted finish".into(),
        });
    }
    let frame = engine
        .replay_frame_through(StepIdx::new(7), &mut store)
        .map_err(replay_err_to_core)?;
    if *frame.read_slot(SlotIdx::new(0))? != SlotValue::I64(4) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "retry chain should advance the counter once per visit".into(),
        });
    }
    Ok(())
}

// ---- End-to-end: repeat loop terminates at done ----

#[test]
fn replay_engine_repeat_loop_terminates_at_done() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::RepeatStart {
                    max_attempts: 2.into(),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(0)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::RepeatCheck {
                    attempt_slot: SlotIdx::new(0),
                    done: StepIdx::new(3),
                },
                None,
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(9)],
    )?;
    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let frame = engine
        .replay_frame_through(StepIdx::new(3), &mut store)
        .map_err(replay_err_to_core)?;
    if *frame.read_slot(SlotIdx::new(1))? != SlotValue::I64(9) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "repeat loop should reach done with the body result".into(),
        });
    }
    Ok(())
}

// ---- End-to-end: for_each loop replays to join ----

#[test]
fn replay_engine_for_each_loop_replays_to_join() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10.into(),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::BuildList {
                    items: Box::from([SlotIdx::new(1)]),
                },
                Some(SlotIdx::new(4)),
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
    )?;
    let mut store = ValueStore::new();
    let source = store.insert_list(Box::from([SlotValue::I64(10), SlotValue::I64(20)]))?;
    let mut run = new_run(&plan)?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(source))?;

    let mut current = plan.entry();
    let mut guard: u32 = 0;
    loop {
        guard = guard
            .checked_add(1)
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "for_each replay did not terminate".into(),
            })?;
        if guard > 32 {
            return Err(CoreError::InternalInvariantViolation {
                reason: "for_each replay exceeded step bound".into(),
            });
        }
        let node = node_at(&plan, current.get())?;
        match replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)? {
            crate::replay::ReplayAction::Continue(next) => current = next,
            crate::replay::ReplayAction::Finished => break,
            crate::replay::ReplayAction::Suspended { .. } => {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "for_each replay suspended unexpectedly".into(),
                });
            }
        }
    }
    match *run.read_slot(SlotIdx::new(5))? {
        SlotValue::List(id) => {
            if store.list(id)? != [SlotValue::I64(20)] {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "for_each replay should finish with the last body list".into(),
                });
            }
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "for_each replay result should be a list".into(),
        }),
    }
}

// ---- End-to-end: together branches merge through the engine ----

#[test]
fn replay_engine_together_branches_merge() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::TogetherStart {
                    branches: Box::from([StepIdx::new(1), StepIdx::new(3)]),
                    join: StepIdx::new(5),
                },
                Some(SlotIdx::new(7)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::TogetherBranch {
                    branch: 0.into(),
                    entry: StepIdx::new(2),
                    join: StepIdx::new(5),
                    accumulator: SlotIdx::new(7),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(6)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::TogetherBranch {
                    branch: 1.into(),
                    entry: StepIdx::new(4),
                    join: StepIdx::new(5),
                    accumulator: SlotIdx::new(7),
                },
                Some(SlotIdx::new(6)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(SlotIdx::new(6)),
                Some(StepIdx::new(5)),
            ),
            crate::workflow::node(
                StepIdx::new(5),
                CompiledNodeKind::TogetherJoin {
                    branch_count: 2.into(),
                    accumulator: SlotIdx::new(7),
                },
                Some(SlotIdx::new(6)),
                Some(StepIdx::new(6)),
            ),
            crate::workflow::node(
                StepIdx::new(6),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(6),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(11), ConstValue::I64(22)],
    )?;
    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let frame = engine
        .replay_frame_through(StepIdx::new(6), &mut store)
        .map_err(replay_err_to_core)?;
    match *frame.read_slot(SlotIdx::new(6))? {
        SlotValue::List(id) => {
            if store.list(id)? != [SlotValue::I64(11), SlotValue::I64(22)] {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "together replay should merge branch results in order".into(),
                });
            }
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "together replay result should be a list".into(),
        }),
    }
}

// ---- End-to-end: error-handler and ask-resume chains ----

#[test]
fn replay_engine_error_handler_chain_replays() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: None.into(),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(5)],
    )?;
    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let frame = engine
        .replay_frame_through(StepIdx::new(2), &mut store)
        .map_err(replay_err_to_core)?;
    if *frame.read_slot(SlotIdx::new(0))? != SlotValue::I64(5) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "error-handler chain should replay the body".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_engine_ask_resume_chain_replays() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(42)],
    )?;
    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let frame = engine
        .replay_frame_through(StepIdx::new(2), &mut store)
        .map_err(replay_err_to_core)?;
    if *frame.read_slot(SlotIdx::new(1))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "ask-resume chain should forward the seeded answer".into(),
        });
    }
    Ok(())
}
