#![forbid(unsafe_code)]

//! Iteration and compound node execution handlers.

use vb_core::action::{ActionContract, ActionError};
use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SeqNo, SlotIdx, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::signals::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::engine::transition::compute_idempotency_key;
use crate::primitives;

/// Backward-compatible execute_do.
pub fn execute_do(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    seq: SeqNo,
    _contract: &ActionContract,
    registry_contracts: &[ActionContract],
) -> RuntimeEngineResult<RuntimeSignal> {
    let action_index = usize::from(action.get());
    let resolved = registry_contracts
        .get(action_index)
        .filter(|c| c.id == action)
        .ok_or(ActionError::UnknownAction { action })?;

    let input_taint = run.read_taint(input).map_err(RuntimeEngineError::Core)?;
    if resolved.idempotency == Idempotency::DeterministicPure && input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

    let output_taint = propagate_action_taint(resolved.idempotency, input_taint);

    let ticket = ActionTicket {
        run: run.run_id(),
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run.run_id(), seq, action),
    };

    if output_taint == Taint::Clean && input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

    Ok(RuntimeSignal::AwaitingAction(ticket))
}

#[allow(clippy::unnecessary_wraps)]
pub fn execute_do_without_contract(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    _input: SlotIdx,
    seq: SeqNo,
) -> RuntimeEngineResult<RuntimeSignal> {
    let ticket = ActionTicket {
        run: run.run_id(),
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run.run_id(), seq, action),
    };
    Ok(RuntimeSignal::AwaitingAction(ticket))
}

/// Resumes an action outcome into the run frame.
///
/// When a retryable failure occurs, the original ticket is used to build a
/// retry ticket with the correct action ID, incremented sequence number,
/// incremented attempt count, and recomputed idempotency key.
pub fn resume_action_outcome(
    run: &mut RunFrame,
    outcome: &vb_core::action::ActionOutcome,
    original_ticket: &ActionTicket,
) -> RuntimeEngineResult<RuntimeSignal> {
    match outcome {
        vb_core::action::ActionOutcome::Ready(ready) => {
            run.write_slot_with_taint(ready.output_slot, ready.value, ready.taint)
                .map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }
        vb_core::action::ActionOutcome::Suspended(ticket) => {
            Ok(RuntimeSignal::AwaitingAction(*ticket))
        }
        vb_core::action::ActionOutcome::Failed(failure) => {
            if failure.retry_policy == vb_core::action::RetryPolicy::Retryable {
                let next_seq = original_ticket
                    .seq
                    .checked_add(1)
                    .ok_or(RuntimeEngineError::Core(
                        EngineError::InternalInvariantViolation {
                            reason: "seq_overflow_on_retry",
                        },
                    ))?;
                let next_attempt = original_ticket
                    .attempt
                    .checked_add(1)
                    .ok_or(RuntimeEngineError::Core(
                        EngineError::InternalInvariantViolation {
                            reason: "attempt_overflow_on_retry",
                        },
                    ))?;
                let idempotency_key =
                    compute_idempotency_key(run.run_id(), next_seq, original_ticket.action);
                Ok(RuntimeSignal::AwaitingAction(ActionTicket {
                    run: run.run_id(),
                    step: original_ticket.step,
                    seq: next_seq,
                    action: original_ticket.action,
                    attempt: next_attempt,
                    idempotency_key,
                }))
            } else {
                Err(RuntimeEngineError::Core(
                    EngineError::UnsupportedPrimitive {
                        primitive: "action_failed_non_retryable",
                    },
                ))
            }
        }
    }
}

use vb_core::action::{propagate_action_taint, ActionTicket, Idempotency};
use vb_core::ids::ActionId;
use vb_core::value::Taint;

pub fn resolve_action_contract(
    action: ActionId,
    contracts: &[ActionContract],
) -> RuntimeEngineResult<&ActionContract> {
    let index = usize::from(action.get());
    contracts
        .get(index)
        .filter(|c| c.id == action)
        .ok_or(ActionError::UnknownAction { action })
        .map_err(RuntimeEngineError::Action)
}

/// Executes an action node (Do variant).
pub fn execute_do_node(
    run: &mut RunFrame,
    node: &vb_core::workflow::CompiledNode,
    action: ActionId,
    input: SlotIdx,
    contracts: &[ActionContract],
) -> RuntimeEngineResult<RuntimeSignal> {
    let seq = SeqNo::new(run.executed());
    if contracts.is_empty() {
        execute_do_without_contract(run, node.id, action, input, seq)
    } else {
        execute_do(
            run,
            node.id,
            action,
            input,
            seq,
            resolve_action_contract(action, contracts)?,
            contracts,
        )
    }
}
