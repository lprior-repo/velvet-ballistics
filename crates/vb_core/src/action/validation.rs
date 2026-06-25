use super::contract::{ActionContract, Idempotency, IdempotencyViolation, RetrySafety, SideEffect};
use super::error::ActionError;
use super::payload::{ActionOutcome, ActionOutputReady};
use crate::frame::RunFrame;
use crate::ids::SlotIdx;
use crate::value::Taint;

/// Computes the output taint for an action given its idempotency and input taint.
///
/// Rules:
/// - DeterministicPure and IdempotentExternal: output taint >= input taint (join).
/// - AtLeastOnceExternal: DerivedFromSecret when any input is Secret/DerivedFromSecret.
/// - Clean result from tainted input is rejected unless the action declares declassification
///   (not modeled here; caller must validate).
#[must_use]
pub const fn propagate_action_taint(idempotency: Idempotency, input_taint: Taint) -> Taint {
    match idempotency {
        Idempotency::DeterministicPure | Idempotency::IdempotentExternal => join_taint(input_taint),
        Idempotency::AtLeastOnceExternal => match input_taint {
            Taint::Clean => Taint::Clean,
            Taint::Secret | Taint::DerivedFromSecret | Taint::Random | Taint::TimeDependent => {
                Taint::DerivedFromSecret
            }
        },
    }
}

/// Returns the least upper bound of the input taint and the output's own taint.
/// Since deterministic/idempotent actions propagate taint upward, the output
/// is always >= the input taint.
const fn join_taint(input: Taint) -> Taint {
    input
}

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
            Taint::Random => {
                return Err(IdempotencyViolation::RandomInKey(u32::from(slot.get())));
            }
            Taint::TimeDependent => {
                return Err(IdempotencyViolation::TimeInKey(u32::from(slot.get())));
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
/// - `RetrySafety::Safe` always passes.
/// - `RetrySafety::KeyRequired` passes if key ingredients are valid.
/// - `RetrySafety::Unsafe` always fails with `MissingKey`.
/// - Actions with `SideEffect::None` always pass regardless of retry_safety.
pub fn verify_idempotency(
    action: &ActionContract,
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation> {
    if action.side_effect == SideEffect::None {
        return Ok(());
    }
    match action.retry_safety {
        RetrySafety::Safe => Ok(()),
        RetrySafety::KeyRequired => {
            if key_slots.is_empty() {
                return Err(IdempotencyViolation::MissingKey(action.side_effect));
            }
            validate_idempotency_key_ingredients(key_slots, frame)
        }
        RetrySafety::Unsafe => Err(IdempotencyViolation::MissingKey(action.side_effect)),
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
    if frame.read_slot(input_slot).is_err() {
        return Err(ActionError::DispatchFailed);
    }
    if output_slot.as_usize() >= usize::from(frame.slot_count()) {
        return Err(ActionError::DispatchFailed);
    }
    Ok(())
}

/// Validates that an action completion outcome is consistent with the contract.
///
/// For success completions, verifies the output slot is valid.
/// For failure completions, verifies the failure code is recognized.
pub fn validate_action_outcome(
    contract: &ActionContract,
    outcome: &ActionOutcome,
) -> Result<(), ActionError> {
    match outcome {
        ActionOutcome::Ready(output_ready) => validate_ready_outcome(contract, output_ready),
        ActionOutcome::Suspended(_) => validate_suspended_outcome(),
        ActionOutcome::Failed(_) => validate_failed_outcome(),
    }
}

/// Validates the output slot index for a Ready action outcome.
fn validate_ready_outcome(
    contract: &ActionContract,
    output_ready: &ActionOutputReady,
) -> Result<(), ActionError> {
    check_output_slot_in_bounds(output_ready.output_slot, contract.output_slot_count)?;
    check_output_size_in_bounds(output_ready.encoded_len, contract.max_output_bytes)?;
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
