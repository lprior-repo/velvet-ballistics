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

/// Absolute runtime ceiling for the encoded byte length of a single action
/// completion output.  This backstop is intentionally smaller than the
/// `ActionContract::max_output_bytes` and `ResourceContract::max_blob_bytes`
/// limits so that a malformed or oversized contract cannot bypass runtime
/// memory containment.  A 64 KiB ceiling is large enough for the largest
/// realistic single-value outputs the runtime is designed to carry (small
/// structured records, encoded JSON blobs, medium-length list arenas) while
/// keeping the worst-case per-completion allocation bounded and cache-line
/// friendly.  See master §19 (action ABI) and §44 points 11/14/19.
pub(crate) const MAX_ACTION_OUTPUT_BYTES: u32 = 64 * 1024;

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
    validate_completion_output(state, ticket, contract, output)?;
    let encoded_value = encode_completion_value(&output)?;
    let encoded_len = encoded_completion_len(state, contract, output, encoded_value.len())?;
    validate_completion_outcome(contract, output)?;
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

fn validate_completion_output(
    state: &RunState,
    ticket: ActionTicket,
    contract: &ActionContract,
    output: ActionOutputReady,
) -> RuntimeResult<()> {
    let input = do_input_slot(state, ticket)?;
    if output.output_slot != do_output_slot(state, ticket)? {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    reject_taint_downgrade(state, input, contract, output.taint)
}

fn encode_completion_value(output: &ActionOutputReady) -> RuntimeResult<Vec<u8>> {
    postcard::to_allocvec(&output.value).map_err(|_| RuntimeError::EncodeFailed)
}

fn encoded_completion_len(
    state: &RunState,
    contract: &ActionContract,
    output: ActionOutputReady,
    actual_len: usize,
) -> RuntimeResult<u32> {
    let encoded_len = encoded_len_u32(actual_len, contract.max_output_bytes)?;
    reject_encoded_len_mismatch(output.encoded_len, encoded_len)?;
    reject_absolute_output_size(encoded_len)?;
    reject_contract_output_size(encoded_len, contract.max_output_bytes)?;
    reject_resource_output_size(
        encoded_len,
        state.workflow.resource_contract().max_blob_bytes,
    )?;
    Ok(encoded_len)
}

fn validate_completion_outcome(
    contract: &ActionContract,
    output: ActionOutputReady,
) -> RuntimeResult<()> {
    vb_core::action::validate_action_outcome(contract, &ActionOutcome::Ready(output))
        .map_err(runtime_error_from_action_error)
}

fn reject_invalid_ticket_key(ticket: ActionTicket) -> RuntimeResult<()> {
    let expected = crate::engine::action::compute_idempotency_key(
        ticket.run,
        ticket.run_attempt,
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
        .and_then(|node| node.output.slot())
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
    validate_completion_taint(contract.idempotency, input_taint, supplied)
}

pub(crate) fn validate_completion_taint(
    idempotency: vb_core::action::Idempotency,
    input_taint: Taint,
    supplied: Taint,
) -> RuntimeResult<()> {
    let required = vb_core::action::propagate_action_taint(idempotency, input_taint);
    if join_taint(required, supplied) == supplied {
        Ok(())
    } else {
        Err(RuntimeError::ActionTaintDowngrade { required, supplied })
    }
}

#[cfg(any(test, kani))]
mod completion_taint_checks {
    include!("completion_taint_checks.rs");
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

/// Enforces the absolute runtime ceiling [`MAX_ACTION_OUTPUT_BYTES`] on the
/// encoded action output length.  This check runs before the
/// per-contract and per-workflow size gates so that no contract value can
/// grant a completion permission to write a payload larger than the runtime
/// is willing to admit into its hot path.  The cap is the only place where
/// the literal `64 KiB` boundary is enforced; the contract and resource
/// limits are merely per-action and per-workflow refinements.
fn reject_absolute_output_size(size: u32) -> RuntimeResult<()> {
    if size <= MAX_ACTION_OUTPUT_BYTES {
        Ok(())
    } else {
        Err(RuntimeError::ActionOutputTooLarge {
            size,
            max: MAX_ACTION_OUTPUT_BYTES,
        })
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
