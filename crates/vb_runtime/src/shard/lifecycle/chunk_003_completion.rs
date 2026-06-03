use vb_core::action::{
    ActionContract, ActionError, ActionOutcome, ActionOutputReady, ActionTicket, Idempotency,
};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::{join_taint, SlotValue, Taint};
use vb_core::workflow::CompiledNodeKind;
use vb_core::ValueStore;

use crate::shard::types::RunState;
use crate::{RuntimeError, RuntimeResult};

pub(crate) fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionFailureOutcome {
    RetryNow,
    DriveHandler,
    FailRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionCompletionPreflight {
    pub(crate) ticket: ActionTicket,
    pub(crate) output_slot: SlotIdx,
    pub(crate) value: SlotValue,
    pub(crate) taint: Taint,
    pub(crate) encoded_value: Vec<u8>,
    pub(crate) encoded_len: u32,
    pub(crate) value_digest: [u8; 32],
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
    check_output_slot(output.output_slot, expected_output)?;
    let input_taint = read_input_taint(state, input)?;
    reject_taint_downgrade(input_taint, contract, output.taint)?;
    let (encoded_value, encoded_len) =
        encode_output_value(&output.value, contract.max_output_bytes)?;
    check_output_encoding_size(
        encoded_len,
        output.encoded_len,
        contract.max_output_bytes,
        state.workflow.resource_contract().max_blob_bytes,
    )?;
    vb_core::action::validate_action_outcome(
        contract,
        &ActionOutcome::Ready(output.clone()),
        input_taint,
    )
    .map_err(runtime_error_from_action_error)?;
    let value_digest = *blake3::hash(&encoded_value).as_bytes();
    Ok(build_action_preflight(
        ticket,
        output,
        encoded_value,
        encoded_len,
        value_digest,
    ))
}

fn check_output_slot(slot: SlotIdx, expected: SlotIdx) -> RuntimeResult<()> {
    if slot == expected {
        Ok(())
    } else {
        Err(RuntimeError::InvalidActionCompletion)
    }
}

fn read_input_taint(state: &RunState, input: SlotIdx) -> RuntimeResult<Taint> {
    state
        .frame
        .read_taint(input)
        .map_err(|_| RuntimeError::InvalidActionCompletion)
}

fn build_action_preflight(
    ticket: ActionTicket,
    output: ActionOutputReady,
    encoded_value: Vec<u8>,
    encoded_len: u32,
    value_digest: [u8; 32],
) -> ActionCompletionPreflight {
    ActionCompletionPreflight {
        ticket,
        output_slot: output.output_slot,
        value: output.value,
        taint: output.taint,
        value_digest,
        encoded_value,
        encoded_len,
    }
}

fn encode_output_value(output: &SlotValue, max_output_bytes: u32) -> RuntimeResult<(Vec<u8>, u32)> {
    let encoded_value = postcard::to_allocvec(output).map_err(|_| RuntimeError::EncodeFailed)?;
    let encoded_len = encoded_len_u32(encoded_value.len(), max_output_bytes)?;
    Ok((encoded_value, encoded_len))
}

fn check_output_encoding_size(
    encoded_len: u32,
    output_encoded_len: u32,
    max_output_bytes: u32,
    max_blob_bytes: u64,
) -> RuntimeResult<()> {
    reject_encoded_len_mismatch(output_encoded_len, encoded_len)?;
    reject_contract_output_size(encoded_len, max_output_bytes)?;
    reject_resource_output_size(encoded_len, max_blob_bytes)
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

/// Rejects a taint downgrade for an action completion.
///
/// # Defense-in-depth note
///
/// This function is kept in sync with `vb_core::action::check_taint_downgrade`.
/// Both are defense-in-depth layers; the runtime enforces at completion and the core enforces
/// at validation. The duplication is architectural debt — do not refactor one without checking the other.
pub(crate) fn reject_taint_downgrade(
    input_taint: Taint,
    contract: &ActionContract,
    supplied: Taint,
) -> RuntimeResult<()> {
    // DeterministicPure and IdempotentExternal actions must operate only on Clean input.
    // Defense-in-depth: engine-side check in execute_do also enforces this,
    // but the completion path must independently reject non-Clean input
    // before allowing frame mutation.
    if (contract.idempotency == Idempotency::DeterministicPure
        || contract.idempotency == Idempotency::IdempotentExternal)
        && input_taint != Taint::Clean
    {
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
        ActionError::TaintViolation { required, supplied } => {
            RuntimeError::ActionTaintDowngrade { required, supplied }
        }
        _ => RuntimeError::InvalidActionCompletion,
    }
}

pub(crate) fn retry_is_available(
    state: &mut RunState,
    ticket: ActionTicket,
    retry_policy: vb_core::action::RetryPolicy,
) -> RuntimeResult<bool> {
    if retry_policy != vb_core::action::RetryPolicy::Retryable
        || !crate::shard::helpers::retry_metadata_exists(state, ticket.step)
    {
        return Ok(false);
    }
    let policy = crate::shard::helpers::retry_policy_after_action(state, ticket.step)?;
    crate::shard::helpers::record_retry_attempt(state, ticket, policy)
}
