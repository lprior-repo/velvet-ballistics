#![forbid(unsafe_code)]
//! Choice-related replay step handlers.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::StepIdx;
use crate::value::SlotValue;

use super::{ReplayAction, ReplayError, eval_expr_for_replay, slot_to_replay_err};

/// Replays a ChooseSlot node which selects a branch based on boolean slot values.
pub fn replay_choose_slot(
    run: &mut RunFrame,
    branches: &[crate::workflow::SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<ReplayAction, ReplayError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches.get(index).ok_or(ReplayError::Internal {
            reason: "choose_slot branch index checked by loop bound",
        })?;
        let value = run.read_slot(branch.condition).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
            EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
            _ => ReplayError::Internal {
                reason: "unexpected error reading choose_slot condition",
            },
        })?;
        match value {
            SlotValue::Bool(true) => {
                run.set_pc(branch.target).map_err(slot_to_replay_err)?;
                run.increment_executed()
                    .map_err(|_| ReplayError::Internal {
                        reason: "executed counter overflow",
                    })?;
                return Ok(ReplayAction::Continue(branch.target));
            }
            SlotValue::Bool(false) => {}
            _ => {
                return Err(ReplayError::Internal {
                    reason: "choose_slot condition is not boolean",
                });
            }
        }
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "choose_slot branch index overflow",
        })?;
    }
    let target = otherwise.ok_or(ReplayError::Internal {
        reason: "choose_slot no branch matched and no otherwise",
    })?;
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
}

/// Replays a ChooseExpr node which selects a branch based on evaluated expressions.
pub fn replay_choose_expr(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut crate::value_store::ValueStore,
    branches: &[crate::workflow::ExprBranch],
    otherwise: Option<StepIdx>,
) -> Result<ReplayAction, ReplayError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches.get(index).ok_or(ReplayError::Internal {
            reason: "choose_expr branch index checked by loop bound",
        })?;
        let (value, _taint) = eval_expr_for_replay(plan, run, store, branch.condition)
            .map_err(|_| ReplayError::ExpressionEvalFailed { step: run.pc() })?;
        match value {
            SlotValue::Bool(true) => {
                run.set_pc(branch.target).map_err(slot_to_replay_err)?;
                run.increment_executed()
                    .map_err(|_| ReplayError::Internal {
                        reason: "executed counter overflow",
                    })?;
                return Ok(ReplayAction::Continue(branch.target));
            }
            SlotValue::Bool(false) => {}
            _ => {
                return Err(ReplayError::Internal {
                    reason: "choose_expr condition is not boolean",
                });
            }
        }
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "choose_expr branch index overflow",
        })?;
    }
    let target = otherwise.ok_or(ReplayError::Internal {
        reason: "choose_expr no branch matched and no otherwise",
    })?;
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::RunFrame;
    use crate::ids::{RunId, SlotIdx, StepIdx};
    use crate::value::SlotValue;
    use crate::workflow::SlotBranch;

    fn make_frame(slot_count: u16) -> RunFrame {
        RunFrame::new(RunId::new(1), StepIdx::new(0), 5, slot_count).expect("valid frame")
    }

    #[test]
    fn choose_slot_true_branch_advances_pc() {
        let mut run = make_frame(2);
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(result, Ok(ReplayAction::Continue(s)) if s == StepIdx::new(2)),
            "expected Continue(2)"
        );
        assert_eq!(run.pc(), StepIdx::new(2));
    }

    #[test]
    fn choose_slot_false_branch_falls_through_to_otherwise() {
        let mut run = make_frame(2);
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(result, Ok(ReplayAction::Continue(s)) if s == StepIdx::new(3)),
            "expected Continue(3)"
        );
        assert_eq!(run.pc(), StepIdx::new(3));
    }

    #[test]
    fn choose_slot_out_of_bounds_slot_returns_slot_not_available() {
        let mut run = make_frame(1);
        let branches = [SlotBranch {
            condition: SlotIdx::new(5),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(result, Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(5)),
            "expected SlotNotAvailable(5)"
        );
    }

    #[test]
    fn choose_slot_uninitialized_slot_returns_slot_not_available() {
        let mut run = make_frame(2);
        // Slot 0 is never written — remains uninitialized
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(result, Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0)),
            "expected SlotNotAvailable(0)"
        );
    }

    #[test]
    fn choose_slot_non_boolean_condition_returns_internal_error() {
        let mut run = make_frame(2);
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(
                result,
                Err(ReplayError::Internal { reason }) if reason == "choose_slot condition is not boolean"
            ),
            "expected Internal(non-bool)"
        );
    }

    #[test]
    fn choose_slot_no_otherwise_and_all_false_returns_internal_error() {
        let mut run = make_frame(2);
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, None);
        assert!(
            matches!(
                result,
                Err(ReplayError::Internal { reason }) if reason == "choose_slot no branch matched and no otherwise"
            ),
            "expected Internal(no otherwise)"
        );
    }

    #[test]
    fn choose_slot_set_pc_failure_returns_slot_to_replay_err() {
        let mut run = make_frame(1);
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(99), // out of bounds for frame with 1 step
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(
                result,
                Err(ReplayError::Internal { reason }) if reason == "unexpected engine error during replay"
            ),
            "expected Internal(engine error)"
        );
    }
}
