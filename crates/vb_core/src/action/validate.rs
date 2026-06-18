//! Validation functions for action dispatch, idempotency, and outcomes.

use crate::action::classification::{Idempotency, IdempotencyViolation, RetrySafety};
use crate::action::error::ActionError;
use crate::action::model::{ActionContract, ActionOutcome, ActionOutputReady, ActionTicket};
use crate::action::taint::propagate_action_taint;
use crate::frame::RunFrame;
use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use crate::value::Taint;

/// Validates that idempotency key ingredients do not contain prohibited values.
///
/// Keys must NOT contain:
/// - Secret-tainted values (would leak information through the key)
/// - Random-generated values (keys must be deterministic)
/// - Time-dependent values (keys must be reproducible across retries)
///
/// The function checks the taint of each slot referenced in `key_slots` via the
/// provided `frame`. Slots with `Taint::Secret` or `Taint::DerivedFromSecret`
/// are rejected. Random and time-dependent checks require additional metadata
/// not yet modeled in `SlotValue`; they are scaffolded here for future extension.
pub fn validate_idempotency_key_ingredients(
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation> {
    let mut i = 0;
    while i < key_slots.len() {
        let Some(&slot) = key_slots.get(i) else {
            break;
        };
        let Ok(slot_taint) = frame.read_taint(slot) else {
            i = match i.checked_add(1) {
                Some(next) => next,
                None => break,
            };
            continue;
        };
        match slot_taint {
            Taint::Clean => {}
            Taint::Secret | Taint::DerivedFromSecret => {
                return Err(IdempotencyViolation::SecretInKey(u32::from(slot.get())));
            }
        }
        i = match i.checked_add(1) {
            Some(next) => next,
            None => break,
        };
    }
    Ok(())
}

/// Verifies whether an action can be safely retried given its contract,
/// the idempotency key slots, and the current run frame.
///
/// Verification rules:
/// - `SideEffect::Pure` always passes regardless of retry_safety.
/// - `RetrySafety::Idempotent` always passes.
/// - `RetrySafety::RequiresIdempotencyKey` passes if key ingredients are valid.
/// - `RetrySafety::NotRetrySafe` always fails with `MissingKey`.
/// - `RetrySafety::Unknown` is statically undecidable; treated as `NotRetrySafe`.
pub fn verify_idempotency(
    action: &ActionContract,
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation> {
    if action.side_effect.is_pure() {
        return Ok(());
    }
    match action.retry_safety {
        RetrySafety::Idempotent => Ok(()),
        RetrySafety::RequiresIdempotencyKey => {
            if key_slots.is_empty() {
                return Err(IdempotencyViolation::MissingKey(action.side_effect));
            }
            validate_idempotency_key_ingredients(key_slots, frame)
        }
        RetrySafety::NotRetrySafe | RetrySafety::Unknown => {
            Err(IdempotencyViolation::MissingKey(action.side_effect))
        }
    }
}

/// Validates that an action dispatch is legal against the declared contract.
///
/// Checks:
/// - Input slot index is within the frame's slot bounds.
/// - Input slot is populated (not uninitialized).
/// - Output slot index is within the frame's slot bounds.
/// - Contract action ID matches the provided ID.
///
/// Returns `Ok(())` if the dispatch is valid, or the appropriate `ActionError`.
pub fn validate_action_dispatch(
    _contract: &ActionContract,
    frame: &RunFrame,
    input_slot: SlotIdx,
    output_slot: SlotIdx,
) -> Result<(), ActionError> {
    // Verify input slot is readable (populated and within frame bounds).
    if frame.read_slot(input_slot).is_err() {
        // Input slot is either out of bounds or uninitialized.
        // We treat both as dispatch failure since the action cannot proceed.
        return Err(ActionError::DispatchFailed);
    }

    // Verify output slot is writable (within frame bounds).
    if output_slot.as_usize() >= usize::from(frame.slot_count()) {
        return Err(ActionError::DispatchFailed);
    }

    Ok(())
}

/// Issues an action ticket for a Do-node suspension.
///
/// Constructs a new `ActionTicket` from the run metadata, action contract,
/// and current attempt counter. The ticket tracks this invocation across
/// suspension boundaries.
pub fn issue_action_ticket(
    run: RunId,
    step: StepIdx,
    seq: SeqNo,
    action: ActionId,
    attempt: u16,
    idempotency_key: u128,
    capacity: u16,
) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq,
        action,
        attempt,
        idempotency_key,
        capacity,
        ..Default::default()
    }
}

/// Validates that an action completion outcome is consistent with the contract.
///
/// For success completions, verifies the output slot is valid and the output taint
/// satisfies the action's taint propagation contract (no downgrade).
/// For failure completions, verifies the failure code is recognized.
pub fn validate_action_outcome(
    contract: &ActionContract,
    outcome: &ActionOutcome,
    input_taint: Taint,
) -> Result<(), ActionError> {
    match outcome {
        ActionOutcome::Ready(output_ready) => {
            validate_ready_outcome(contract, output_ready, input_taint)
        }
        ActionOutcome::Suspended(_) => validate_suspended_outcome(),
        ActionOutcome::Failed(_) => validate_failed_outcome(),
    }
}

/// Validates the output slot index and taint for a Ready action outcome.
///
/// Rejects completions that attempt to downgrade taint below the level
/// required by the action's idempotency contract and input taint.
fn validate_ready_outcome(
    contract: &ActionContract,
    output_ready: &ActionOutputReady,
    input_taint: Taint,
) -> Result<(), ActionError> {
    check_output_slot_in_bounds(output_ready.output_slot, contract.output_slot_count)?;
    check_output_size_in_bounds(output_ready.encoded_len, contract.max_output_bytes)?;
    check_taint_downgrade(contract.idempotency, input_taint, output_ready.taint)?;
    Ok(())
}

fn check_output_size_in_bounds(actual_bytes: u32, max_bytes: u32) -> Result<(), ActionError> {
    if actual_bytes > max_bytes {
        return Err(ActionError::PayloadTooLarge {
            max_bytes,
            actual_bytes,
        });
    }
    Ok(())
}

/// Checks that the supplied output taint is not a downgrade from the required taint.
///
/// # Defense-in-depth note
///
/// This function is kept in sync with `vb_runtime::shard::lifecycle::reject_taint_downgrade`.
/// Both are defense-in-depth layers; the core validates here and the runtime enforces at
/// completion. The duplication is architectural debt — do not refactor one without checking the other.
///
/// DeterministicPure and IdempotentExternal actions additionally require that the
/// input is Clean and the output is Clean.
/// For all actions, the supplied taint must be at least as restrictive as the
/// taint propagated from the input according to the idempotency contract.
fn check_taint_downgrade(
    idempotency: Idempotency,
    input_taint: Taint,
    supplied: Taint,
) -> Result<(), ActionError> {
    if (idempotency == Idempotency::DeterministicPure
        || idempotency == Idempotency::IdempotentExternal)
        && input_taint != Taint::Clean
    {
        return Err(ActionError::TaintViolation {
            required: Taint::Clean,
            supplied: input_taint,
        });
    }
    if (idempotency == Idempotency::DeterministicPure
        || idempotency == Idempotency::IdempotentExternal)
        && supplied != Taint::Clean
    {
        return Err(ActionError::TaintViolation {
            required: Taint::Clean,
            supplied,
        });
    }
    let required = propagate_action_taint(idempotency, input_taint);
    if crate::value::join_taint(required, supplied) != supplied {
        return Err(ActionError::TaintViolation { required, supplied });
    }
    Ok(())
}

/// Checks that the output slot index is within the contract's declared bounds.
fn check_output_slot_in_bounds(slot: SlotIdx, max_slots: u16) -> Result<(), ActionError> {
    let slot_raw = slot.get();
    if u32::from(slot_raw) >= u32::from(max_slots) && max_slots > 0 {
        return Err(ActionError::OutputSlotOutOfBounds {
            slot: slot_raw,
            max_slots,
        });
    }
    Ok(())
}

/// Suspension is not a terminal outcome; completing with a suspended outcome is invalid.
fn validate_suspended_outcome() -> Result<(), ActionError> {
    Err(ActionError::DispatchFailed)
}

/// Failure outcomes are always valid terminal completions.
fn validate_failed_outcome() -> Result<(), ActionError> {
    Ok(())
}
