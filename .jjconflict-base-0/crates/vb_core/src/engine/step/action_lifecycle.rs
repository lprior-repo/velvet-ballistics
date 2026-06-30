use crate::EngineSignal;
use crate::action::{
    ActionContract, ActionFailure, ActionFailureReport, ActionJournalEvent, ActionOutputReady,
    ActionResumeRejection, ActionResumeReport, ActionTicket, action_ticket_has_valid_key,
};
use crate::engine::error_routing::{ErrorHandlerOutcome, route_error_handler};
use crate::errors::EngineError;
use crate::frame::{RunFrame, StepState};
use crate::ids::{ActionId, SlotIdx, StepIdx};
use crate::workflow::{CompiledNodeKind, CompiledWorkflow};

struct SuspendedActionContext {
    step: StepIdx,
    output: SlotIdx,
}

pub fn journal_action_suspended(
    ticket: ActionTicket,
    action: ActionId,
    input_slot: SlotIdx,
    output_slot: SlotIdx,
    step: StepIdx,
) -> Result<ActionJournalEvent, EngineError> {
    if ticket.action != action {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::ActionMismatch,
        ));
    }
    if ticket.step != step {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::StepNotCurrentPc,
        ));
    }
    validate_ticket_shape(ticket)?;
    Ok(ActionJournalEvent::Suspended {
        ticket,
        attempt: ticket.attempt,
        action,
        input_slot,
        output_slot,
        step,
    })
}

pub fn resume_action_completion(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    ticket: ActionTicket,
    output: ActionOutputReady,
    encoded_payload: &[u8],
    contract: &ActionContract,
) -> Result<(EngineSignal, ActionJournalEvent), EngineError> {
    let context = resolve_completion_context(plan, run, ticket, output.output_slot)?;
    validate_contract_output_slot(ticket, contract)?;
    validate_resume_payload(ticket, output.encoded_len, encoded_payload, contract)?;
    run.write_slot_with_taint(output.output_slot, output.value, output.taint)?;
    run.mark_succeeded(context.step)?;
    run.set_pc(context.next)?;
    run.increment_executed()?;

    let journal = ActionJournalEvent::Completed {
        ticket,
        attempt: ticket.attempt,
        output,
    };
    Ok((EngineSignal::Continue, journal))
}

pub fn resume_action_failure(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    ticket: ActionTicket,
    failure: ActionFailure,
    encoded_payload: &[u8],
    contract: &ActionContract,
) -> Result<(EngineSignal, ActionJournalEvent), EngineError> {
    let context = resolve_action_resume(plan, run, ticket)?;
    validate_contract_output_slot(ticket, contract)?;
    validate_resume_payload(ticket, failure.encoded_len, encoded_payload, contract)?;

    let journal = ActionJournalEvent::Failed {
        ticket,
        attempt: ticket.attempt,
        output_slot: context.output,
        failure: failure.clone(),
    };

    let report = ActionFailureReport::new(context.step, ticket.action, failure);
    let error = EngineError::ActionFailed { report };
    let mut staged = run.clone();
    staged.mark_failed(context.step)?;
    match route_error_handler(plan, &mut staged, context.step, &error)? {
        ErrorHandlerOutcome::Routed => {
            *run = staged;
            Ok((EngineSignal::Continue, journal))
        }
        ErrorHandlerOutcome::NoHandler => {
            *run = staged;
            Ok((EngineSignal::ActionFailureUnhandled, journal))
        }
    }
}

fn validate_contract_output_slot(
    ticket: ActionTicket,
    contract: &ActionContract,
) -> Result<(), EngineError> {
    if contract.output_slot_count == 0 {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::ContractOutputUndeclared,
        ));
    }
    Ok(())
}

fn resolve_completion_context(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    ticket: ActionTicket,
    output_slot: SlotIdx,
) -> Result<ResolvedActionCompletion, EngineError> {
    let resume = resolve_action_resume(plan, run, ticket)?;
    let expected = resume.output;
    if expected != output_slot {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::OutputMismatch,
        ));
    }
    if output_slot.as_usize() >= usize::from(run.slot_count()) {
        return Err(EngineError::SlotOutOfBounds { slot: output_slot });
    }
    let next = plan
        .node(resume.step)
        .ok_or(EngineError::InvalidProgramCounter { step: resume.step })?
        .next
        .ok_or(EngineError::MissingNextStep { step: resume.step })?;
    validate_next_step(plan, run, next)?;
    validate_can_increment(run)?;
    Ok(ResolvedActionCompletion {
        step: resume.step,
        next,
    })
}

fn resolve_action_resume(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    ticket: ActionTicket,
) -> Result<SuspendedActionContext, EngineError> {
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

    let node = plan
        .node(ticket.step)
        .ok_or(EngineError::InvalidProgramCounter { step: ticket.step })?;
    match &node.kind {
        CompiledNodeKind::Do { action, .. } if *action == ticket.action => {
            let output = node
                .output
                .ok_or(EngineError::MissingOutputSlot { step: ticket.step })?;
            Ok(SuspendedActionContext {
                step: ticket.step,
                output,
            })
        }
        CompiledNodeKind::Do { .. } => Err(resume_rejection(
            ticket,
            ActionResumeRejection::ActionMismatch,
        )),
        _ => Err(resume_rejection(ticket, ActionResumeRejection::NonDoNode)),
    }
}

fn validate_ticket_shape(ticket: ActionTicket) -> Result<(), EngineError> {
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

fn validate_next_step(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    next: StepIdx,
) -> Result<(), EngineError> {
    if next.as_usize() >= usize::from(run.step_count()) || plan.node(next).is_none() {
        return Err(EngineError::InvalidProgramCounter { step: next });
    }
    Ok(())
}

fn validate_can_increment(run: &RunFrame) -> Result<(), EngineError> {
    if run.executed() == u64::MAX {
        return Err(EngineError::StepCounterOverflow);
    }
    Ok(())
}

fn validate_resume_payload(
    ticket: ActionTicket,
    reported_len: u32,
    encoded_payload: &[u8],
    contract: &ActionContract,
) -> Result<(), EngineError> {
    if contract.id != ticket.action {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::ContractMismatch,
        ));
    }
    let actual_len = u32::try_from(encoded_payload.len())
        .map_err(|_| resume_rejection(ticket, ActionResumeRejection::EncodedPayloadTooLarge))?;
    if actual_len != reported_len {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::EncodedPayloadLenMismatch,
        ));
    }
    if actual_len > contract.max_output_bytes {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::EncodedPayloadTooLarge,
        ));
    }
    Ok(())
}

fn resume_rejection(ticket: ActionTicket, rejection: ActionResumeRejection) -> EngineError {
    EngineError::ActionResumeRejected {
        report: ActionResumeReport::new(rejection, ticket),
    }
}

struct ResolvedActionCompletion {
    step: StepIdx,
    next: StepIdx,
}
