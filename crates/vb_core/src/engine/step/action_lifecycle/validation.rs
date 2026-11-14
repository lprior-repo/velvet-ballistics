use crate::action::{
    ActionContract, ActionResumeRejection, ActionResumeReport, ActionTicket,
    action_ticket_has_valid_key,
};
use crate::errors::EngineError;
use crate::frame::{RunFrame, StepState};

pub(super) fn validate_resume_ticket(
    run: &RunFrame,
    ticket: ActionTicket,
) -> Result<(), EngineError> {
    if ticket.run != run.run_id() {
        return Err(resume_rejection(ticket, ActionResumeRejection::RunMismatch));
    }
    if ticket.step != run.pc() {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::StepNotCurrentPc,
        ));
    }
    validate_ticket_shape(ticket)?;
    if run.step_state(ticket.step)? != StepState::Running {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::StepNotRunning,
        ));
    }
    Ok(())
}

pub(super) fn validate_ticket_shape(ticket: ActionTicket) -> Result<(), EngineError> {
    if ticket.attempt == 0 {
        return Err(resume_rejection(ticket, ActionResumeRejection::AttemptZero));
    }
    if ticket.capacity == 0 {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::CapacityZero,
        ));
    }
    if ticket.attempt > ticket.capacity {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::AttemptExceedsCapacity,
        ));
    }
    if !action_ticket_has_valid_key(ticket) {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::IdempotencyKeyMismatch,
        ));
    }
    Ok(())
}

pub(super) fn validate_resume_payload(
    ticket: ActionTicket,
    reported_len: u32,
    encoded_payload: &[u8],
    contract: &ActionContract,
) -> Result<(), EngineError> {
    validate_payload_contract(ticket, contract)?;
    let actual_len = encoded_payload_len(ticket, encoded_payload)?;
    validate_reported_payload_len(ticket, actual_len, reported_len)?;
    validate_payload_limit(ticket, actual_len, contract.max_output_bytes)
}

fn validate_payload_contract(
    ticket: ActionTicket,
    contract: &ActionContract,
) -> Result<(), EngineError> {
    if contract.id != ticket.action {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::ContractMismatch,
        ));
    }
    Ok(())
}

fn encoded_payload_len(ticket: ActionTicket, payload: &[u8]) -> Result<u32, EngineError> {
    u32::try_from(payload.len())
        .map_err(|_| resume_rejection(ticket, ActionResumeRejection::EncodedPayloadTooLarge))
}

fn validate_reported_payload_len(
    ticket: ActionTicket,
    actual_len: u32,
    reported_len: u32,
) -> Result<(), EngineError> {
    if actual_len != reported_len {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::EncodedPayloadLenMismatch,
        ));
    }
    Ok(())
}

fn validate_payload_limit(
    ticket: ActionTicket,
    actual_len: u32,
    max_output_bytes: u32,
) -> Result<(), EngineError> {
    if actual_len > max_output_bytes {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::EncodedPayloadTooLarge,
        ));
    }
    Ok(())
}

pub(super) fn resume_rejection(
    ticket: ActionTicket,
    rejection: ActionResumeRejection,
) -> EngineError {
    EngineError::ActionResumeRejected {
        report: ActionResumeReport::new(rejection, ticket),
    }
}
