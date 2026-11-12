use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::workflow::{CompiledNode, CompiledWorkflow};

use super::super::{ReplayError, engine_to_replay_err};
use super::iter::replay_next;
use super::nodes::replay_jump;
use super::types::ReplayAction;

/// Replays `RetryCheck`: advances the attempt counter with checked write-back
/// and routes to `body` while attempts remain, else to `exhausted`.
/// Mirrors `vb_runtime::engine::execute::budget::handle_retry_check`.
///
/// The executor compares against an external runtime `RetryPolicy`; replay
/// has no policy input, so the bound is the workflow's own resource contract
/// (`max_retry_attempts`, default 3, matching the executor `DEFAULT`
/// policy). An uninitialized policy slot is the first visit (attempt 0),
/// mirroring the executor RE-003 rule.
pub(super) fn replay_retry_check(
    run: &mut RunFrame,
    plan: &CompiledWorkflow,
    policy_slot: SlotIdx,
    body: StepIdx,
    exhausted: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let current_attempt = match run.read_slot(policy_slot) {
        Ok(value) => match *value {
            SlotValue::I64(raw) => u16::try_from(raw).map_err(|_| ReplayError::Internal {
                reason: "retry attempt out of range during replay",
            })?,
            _ => {
                return Err(ReplayError::Internal {
                    reason: "retry policy slot was not number during replay",
                });
            }
        },
        Err(CoreError::SlotUninitialized { .. }) => 0,
        Err(other) => return Err(engine_to_replay_err(other)),
    };
    let max_attempts = plan.resource_contract().max_retry_attempts;
    let target = if current_attempt < max_attempts {
        body
    } else {
        exhausted
    };
    let next_attempt = current_attempt
        .checked_add(1)
        .ok_or(ReplayError::Internal {
            reason: "retry_attempt_overflow during replay",
        })?;
    run.write_slot(policy_slot, SlotValue::I64(i64::from(next_attempt)))
        .map_err(engine_to_replay_err)?;
    replay_jump(run, target)
}

/// Replays `ErrorHandler`: routes the program counter to the body step.
/// Mirrors `handle_error_handler`; `error_slot` is ignored exactly like the
/// executor (failure writes belong to error routing, and replay has no
/// failure to route).
pub(super) fn replay_error_handler(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    replay_jump(run, body)
}

/// Replays `AskResume`: validates the answer slot, forwards it to the node
/// output when declared, and continues. Mirrors
/// `vb_runtime::primitives::wait_ask::ask_resume`.
pub(super) fn replay_ask_resume(
    run: &mut RunFrame,
    node: &CompiledNode,
    answer: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let answer_value = *run.read_slot(answer).map_err(engine_to_replay_err)?;
    let answer_taint = run.read_taint(answer).map_err(engine_to_replay_err)?;
    if let Some(output) = node.output.slot() {
        run.write_slot_with_taint(output, answer_value, answer_taint)
            .map_err(engine_to_replay_err)?;
    }
    let target = replay_next(node, "AskResume node missing next step")?;
    replay_jump(run, target)
}
