use vb_core::ValueStore;
use vb_core::action::{
    ActionContract, ActionError, ActionFailure, ActionOutcome, ActionOutputReady, ActionTicket,
    RetryPolicy as VbCoreRetryPolicy,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint, join_taint};
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::engine::{
    EvidenceCollector, RetryPolicy, RuntimeEngineResult, RuntimeSignal, drive_deterministic_full,
};
use crate::journal::RuntimeJournalEvent;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use crate::primitives::collect::CollectStates;
use crate::shard::types::{
    AskAnswer, PendingTimer, PendingTimerKind, ResumeError, ResumeResult, ResumeStatus, RunState,
    RuntimeEvent, RuntimeState, Shard,
};

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActionFailureOutcome {
    RetryNow,
    DriveHandler,
    FailRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionCompletionPreflight {
    ticket: ActionTicket,
    output_slot: SlotIdx,
    value: SlotValue,
    taint: Taint,
    encoded_value: Vec<u8>,
    encoded_len: u32,
    value_digest: [u8; 32],
}

pub(crate) fn preflight_action_completion(
    state: &RunState,
    ticket: ActionTicket,
    output: ActionOutputReady,
) -> RuntimeResult<ActionCompletionPreflight> {
    crate::shard::helpers::validate_action_completion(state, ticket)?;
    reject_invalid_ticket_key(ticket)?;
    let contract = resolve_completion_contract(state, ticket.action)?;
    let input = do_input_slot(state, ticket)?;
    let expected_output = do_output_slot(state, ticket)?;
    if output.output_slot != expected_output {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    reject_taint_downgrade(state, input, contract, output.taint)?;
    let encoded_value =
        postcard::to_allocvec(&output.value).map_err(|_| RuntimeError::EncodeFailed)?;
    let encoded_len = encoded_len_u32(encoded_value.len(), contract.max_output_bytes)?;
    reject_encoded_len_mismatch(output.encoded_len, encoded_len)?;
    reject_contract_output_size(encoded_len, contract.max_output_bytes)?;
    reject_resource_output_size(
        encoded_len,
        state.workflow.resource_contract().max_blob_bytes,
    )?;
    vb_core::action::validate_action_outcome(contract, &ActionOutcome::Ready(output))
        .map_err(runtime_error_from_action_error)?;
    Ok(ActionCompletionPreflight {
        ticket,
        output_slot: output.output_slot,
        value: output.value,
        taint: output.taint,
        value_digest: *blake3::hash(&encoded_value).as_bytes(),
        encoded_value,
        encoded_len,
    })
}

fn reject_invalid_ticket_key(ticket: ActionTicket) -> RuntimeResult<()> {
    let expected =
        crate::engine::action::compute_idempotency_key(ticket.run, ticket.seq, ticket.action);
    if ticket.idempotency_key == expected {
        Ok(())
    } else {
        Err(RuntimeError::InvalidActionCompletion)
    }
}

fn resolve_completion_contract(
    state: &RunState,
    action: vb_core::ActionId,
) -> RuntimeResult<&ActionContract> {
    crate::engine::action::resolve_contract(action, &state.action_contracts)
        .map_err(|_| RuntimeError::InvalidActionCompletion)
}

fn do_input_slot(state: &RunState, ticket: ActionTicket) -> RuntimeResult<SlotIdx> {
    match state.workflow.node(ticket.step).map(|node| &node.kind) {
        Some(CompiledNodeKind::Do { input, .. }) => Ok(*input),
        _ => Err(RuntimeError::InvalidActionCompletion),
    }
}

fn do_output_slot(state: &RunState, ticket: ActionTicket) -> RuntimeResult<SlotIdx> {
    state
        .workflow
        .node(ticket.step)
        .and_then(|node| node.output)
        .ok_or(RuntimeError::InvalidActionCompletion)
}

fn reject_taint_downgrade(
    state: &RunState,
    input: SlotIdx,
    contract: &ActionContract,
    supplied: Taint,
) -> RuntimeResult<()> {
    let input_taint = state
        .frame
        .read_taint(input)
        .map_err(|_| RuntimeError::InvalidActionCompletion)?;
    let required = vb_core::action::propagate_action_taint(contract.idempotency, input_taint);
    if join_taint(required, supplied) == supplied {
        Ok(())
    } else {
        Err(RuntimeError::ActionTaintDowngrade { required, supplied })
    }
}

fn encoded_len_u32(len: usize, max: u32) -> RuntimeResult<u32> {
    u32::try_from(len).map_err(|_| RuntimeError::ActionOutputTooLarge {
        size: u32::MAX,
        max,
    })
}

fn reject_encoded_len_mismatch(declared: u32, actual: u32) -> RuntimeResult<()> {
    if declared == actual {
        Ok(())
    } else {
        Err(RuntimeError::ActionOutputLengthMismatch { declared, actual })
    }
}

fn reject_contract_output_size(size: u32, max: u32) -> RuntimeResult<()> {
    if size <= max {
        Ok(())
    } else {
        Err(RuntimeError::ActionOutputTooLarge { size, max })
    }
}

fn reject_resource_output_size(size: u32, max: u64) -> RuntimeResult<()> {
    let size_u64 = u64::from(size);
    if size_u64 <= max {
        Ok(())
    } else {
        Err(RuntimeError::ActionOutputBlobTooLarge {
            size: size_u64,
            max,
        })
    }
}

fn runtime_error_from_action_error(error: ActionError) -> RuntimeError {
    match error {
        ActionError::PayloadTooLarge {
            max_bytes,
            actual_bytes,
        } => RuntimeError::ActionOutputTooLarge {
            size: actual_bytes,
            max: max_bytes,
        },
        ActionError::OutputSlotOutOfBounds { .. } => RuntimeError::InvalidActionCompletion,
        _ => RuntimeError::InvalidActionCompletion,
    }
}

/// Preflighted outcome of an action failure, capturing every value the
/// apply step needs without mutating state. This split keeps the durable
/// journal append on the critical path before any frame mutation so a
/// failed append never diverges memory-only state from durable evidence.
pub(crate) struct ActionFailurePreflight {
    ticket: ActionTicket,
    outcome: ActionFailureOutcome,
    /// Value to write into `state.action_attempts[step]` for `RetryNow`.
    /// Unused for `DriveHandler` and `FailRun`.
    next_attempt: u16,
    /// Handler step for `DriveHandler`. Required when `outcome ==
    /// ActionFailureOutcome::DriveHandler`.
    handler_pc: Option<StepIdx>,
    /// Error slot for `DriveHandler`. `None` means no slot write is
    /// required; `Some(slot)` triggers `write_failure_slot`.
    error_slot: Option<SlotIdx>,
}

/// Read-only preflight: validates the ticket, decides the failure
/// outcome (Retry / DriveHandler / FailRun), and pre-computes every
/// value the apply step will need. NEVER mutates `state`; the caller
/// is expected to append the journal event first and only then call
/// `apply_action_failure_preflight` on a `&mut RunState`.
pub(crate) fn preflight_action_failure(
    state: &RunState,
    ticket: ActionTicket,
    failure: &ActionFailure,
) -> RuntimeResult<ActionFailurePreflight> {
    crate::shard::helpers::validate_action_completion(state, ticket)?;

    // Retry path: only when policy is Retryable AND retry metadata exists.
    if failure.retry_policy == VbCoreRetryPolicy::Retryable
        && crate::shard::helpers::retry_metadata_exists(state, ticket.step)
    {
        let policy = crate::shard::helpers::retry_policy_after_action(state, ticket.step)?;
        // attempt must be within (0, max_attempts] — mirrors
        // record_retry_attempt's validation.
        if policy.max_attempts == 0 || ticket.attempt == 0 || ticket.attempt > policy.max_attempts {
            return Err(RuntimeError::AttemptBeyondMax {
                attempt: ticket.attempt,
                max: policy.max_attempts,
            });
        }
        let current_attempt = state
            .action_attempts
            .get(ticket.step.as_usize())
            .copied()
            .unwrap_or(0);
        let new_attempt = current_attempt.max(ticket.attempt);
        if new_attempt < policy.max_attempts {
            let next = new_attempt
                .checked_add(1)
                .ok_or(RuntimeError::UnsupportedOperation {
                    operation: "retry_attempt_overflow",
                })?;
            return Ok(ActionFailurePreflight {
                ticket,
                outcome: ActionFailureOutcome::RetryNow,
                next_attempt: next,
                handler_pc: None,
                error_slot: None,
            });
        }
        // Retry budget exhausted: fall through to error-handler path
        // (matches existing semantics where `record_retry_attempt`
        // returning false causes the caller to fall through to
        // `apply_error_handler`).
    }

    // Error-handler path.
    match crate::shard::helpers::find_error_handler_for_failure(&state.workflow, ticket.step) {
        Some((handler, error_slot)) => Ok(ActionFailurePreflight {
            ticket,
            outcome: ActionFailureOutcome::DriveHandler,
            next_attempt: 0,
            handler_pc: Some(handler),
            error_slot,
        }),
        None => Ok(ActionFailurePreflight {
            ticket,
            outcome: ActionFailureOutcome::FailRun,
            next_attempt: 0,
            handler_pc: None,
            error_slot: None,
        }),
    }
}

/// Apply the preflighted outcome to state. Pure mutation driven by
/// the preflight decision; never touches the journal.
pub(crate) fn apply_action_failure_preflight(
    state: &mut RunState,
    preflight: &ActionFailurePreflight,
) -> RuntimeResult<()> {
    match preflight.outcome {
        ActionFailureOutcome::RetryNow => {
            // Persist the pre-computed attempt counter, then reset pc
            // to the failed step for the next attempt.
            if let Some(slot) = state
                .action_attempts
                .get_mut(preflight.ticket.step.as_usize())
            {
                *slot = preflight.next_attempt;
            }
            state
                .frame
                .set_pc(preflight.ticket.step)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            Ok(())
        }
        ActionFailureOutcome::DriveHandler => {
            let Some(handler) = preflight.handler_pc else {
                return Err(RuntimeError::UnsupportedOperation {
                    operation: "drive_handler_missing_target",
                });
            };
            state
                .frame
                .mark_failed(preflight.ticket.step)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            write_failure_slot(state, preflight.ticket.step, preflight.error_slot)?;
            state
                .frame
                .set_pc(handler)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            Ok(())
        }
        ActionFailureOutcome::FailRun => {
            // No frame mutation here; the caller drives
            // take_run_state / apply(Fail) / fail_run_state.
            Ok(())
        }
    }
}

fn write_failure_slot(
    state: &mut RunState,
    step: StepIdx,
    error_slot: Option<SlotIdx>,
) -> RuntimeResult<()> {
    match error_slot {
        Some(slot) => state
            .frame
            .write_slot(slot, vb_core::value::SlotValue::I64(i64::from(step.get())))
            .map_err(|_| RuntimeError::InvalidActionCompletion),
        None => Ok(()),
    }
}
