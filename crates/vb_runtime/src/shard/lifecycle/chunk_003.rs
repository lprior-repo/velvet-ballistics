use vb_core::ValueStore;
use vb_core::action::{
    ActionContract, ActionError, ActionFailure, ActionOutputReady, ActionOutcome, ActionTicket,
    Idempotency, RetryPolicy as VbCoreRetryPolicy,
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
    AskAnswer, PendingTimerKind, ResumeError, ResumeResult, ResumeStatus, RunState, RuntimeEvent,
    RuntimeState, Shard,
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
    let encoded_value = postcard::to_allocvec(&output.value).map_err(|_| RuntimeError::EncodeFailed)?;
    let encoded_len = encoded_len_u32(encoded_value.len(), contract.max_output_bytes)?;
    reject_encoded_len_mismatch(output.encoded_len, encoded_len)?;
    reject_contract_output_size(encoded_len, contract.max_output_bytes)?;
    reject_resource_output_size(encoded_len, state.workflow.resource_contract().max_blob_bytes)?;
    vb_core::action::validate_action_outcome(contract, &ActionOutcome::Ready(output.clone()))
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
    let expected = crate::engine::action::compute_idempotency_key(
        ticket.run,
        ticket.seq,
        ticket.action,
    );
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
    // DeterministicPure actions must operate only on Clean input.
    // Defense-in-depth: engine-side check in execute_do also enforces this,
    // but the completion path must independently reject non-Clean input
    // before allowing frame mutation.
    if contract.idempotency == Idempotency::DeterministicPure && input_taint != Taint::Clean {
        return Err(RuntimeError::ActionTaintDowngrade {
            required: Taint::Clean,
            supplied: input_taint,
        });
    }
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

fn retry_is_available(
    state: &mut RunState,
    ticket: ActionTicket,
    retry_policy: VbCoreRetryPolicy,
) -> RuntimeResult<bool> {
    if retry_policy != VbCoreRetryPolicy::Retryable
        || !crate::shard::helpers::retry_metadata_exists(state, ticket.step)
    {
        return Ok(false);
    }
    let policy = crate::shard::helpers::retry_policy_after_action(state, ticket.step)?;
    crate::shard::helpers::record_retry_attempt(state, ticket, policy)
}

fn apply_error_handler(
    state: &mut RunState,
    ticket: ActionTicket,
) -> RuntimeResult<ActionFailureOutcome> {
    match crate::shard::helpers::find_error_handler_for_failure(&state.workflow, ticket.step) {
        Some((handler, error_slot)) => {
            state
                .frame
                .mark_failed(ticket.step)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            write_failure_slot(state, ticket.step, error_slot)?;
            state
                .frame
                .set_pc(handler)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            Ok(ActionFailureOutcome::DriveHandler)
        }
        None => Ok(ActionFailureOutcome::FailRun),
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
