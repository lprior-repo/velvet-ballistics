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
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let new_attempt = current_attempt.max(ticket.attempt);
        if new_attempt < policy.max_attempts {
            let next = new_attempt
                .checked_add(1)
                .ok_or(RuntimeError::UnsupportedOperation {
                    operation: "retry_attempt_overflow",
                })?;
            validate_retry_apply(state, ticket.step)?;
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
        Some((handler, error_slot)) => {
            validate_handler_apply(state, ticket.step, handler, error_slot)?;
            Ok(ActionFailurePreflight {
                ticket,
                outcome: ActionFailureOutcome::DriveHandler,
                next_attempt: 0,
                handler_pc: Some(handler),
                error_slot,
            })
        }
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
/// the preflight decision; never touches the journal. All bounds and
/// transition predicates for these fallible frame operations are checked in
/// `preflight_action_failure` before the durable `ActionFailed` append.
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

fn validate_retry_apply(state: &RunState, step: StepIdx) -> RuntimeResult<()> {
    validate_step_index(state, step)?;
    if state.action_attempts.get(step.as_usize()).is_none() {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    Ok(())
}

fn validate_handler_apply(
    state: &RunState,
    failed: StepIdx,
    handler: StepIdx,
    error_slot: Option<SlotIdx>,
) -> RuntimeResult<()> {
    validate_step_index(state, failed)?;
    validate_step_index(state, handler)?;
    if state
        .frame
        .step_state(failed)
        .map_err(|_| RuntimeError::InvalidActionCompletion)?
        != vb_core::frame::StepState::Running
    {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    if let Some(slot) = error_slot {
        validate_slot_index(state, slot)?;
    }
    Ok(())
}

fn validate_step_index(state: &RunState, step: StepIdx) -> RuntimeResult<()> {
    if step.as_usize() >= usize::from(state.frame.step_count()) {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    Ok(())
}

fn validate_slot_index(state: &RunState, slot: SlotIdx) -> RuntimeResult<()> {
    if slot.as_usize() >= usize::from(state.frame.slot_count()) {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    Ok(())
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
