use crate::EngineSignal;
use crate::action::{
    ActionContract, ActionFailure, ActionFailureReport, ActionJournalEvent, ActionOutputReady,
    ActionResumeRejection, ActionTicket,
};
use crate::engine::error_routing::{ErrorHandlerOutcome, route_error_handler};
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ActionId, SlotIdx, StepIdx};
use crate::workflow::{CompiledNodeKind, CompiledWorkflow};

#[path = "action_lifecycle/validation.rs"]
mod validation;
use validation::{
    resume_rejection, validate_resume_payload, validate_resume_ticket, validate_ticket_shape,
};

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
    validate_suspended_ticket(ticket, action, step)?;
    Ok(ActionJournalEvent::Suspended {
        ticket,
        attempt: ticket.attempt,
        action,
        input_slot,
        output_slot,
        step,
    })
}

fn validate_suspended_ticket(
    ticket: ActionTicket,
    action: ActionId,
    step: StepIdx,
) -> Result<(), EngineError> {
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
    validate_ticket_shape(ticket)
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
    let journal = failed_action_journal(ticket, context.output, failure.clone());
    route_action_failure(plan, run, ticket, context, failure, journal)
}

fn failed_action_journal(
    ticket: ActionTicket,
    output_slot: SlotIdx,
    failure: ActionFailure,
) -> ActionJournalEvent {
    ActionJournalEvent::Failed {
        ticket,
        attempt: ticket.attempt,
        output_slot,
        failure,
    }
}

fn route_action_failure(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    ticket: ActionTicket,
    context: SuspendedActionContext,
    failure: ActionFailure,
    journal: ActionJournalEvent,
) -> Result<(EngineSignal, ActionJournalEvent), EngineError> {
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
    if resume.output != output_slot {
        return Err(resume_rejection(
            ticket,
            ActionResumeRejection::OutputMismatch,
        ));
    }
    if output_slot.as_usize() >= usize::from(run.slot_count()) {
        return Err(EngineError::SlotOutOfBounds { slot: output_slot });
    }
    let next = resolve_completion_next(plan, run, resume.step)?;
    Ok(ResolvedActionCompletion {
        step: resume.step,
        next,
    })
}

fn resolve_completion_next(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    step: StepIdx,
) -> Result<StepIdx, EngineError> {
    let next = plan
        .node(step)
        .ok_or(EngineError::InvalidProgramCounter { step })?
        .routing
        .next
        .step()
        .ok_or(EngineError::MissingNextStep { step })?;
    validate_next_step(plan, run, next)?;
    validate_can_increment(run)?;
    Ok(next)
}

fn resolve_action_resume(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    ticket: ActionTicket,
) -> Result<SuspendedActionContext, EngineError> {
    validate_resume_ticket(run, ticket)?;
    let node = plan
        .node(ticket.step)
        .ok_or(EngineError::InvalidProgramCounter { step: ticket.step })?;
    match &node.kind {
        CompiledNodeKind::Do { action, .. } if *action == ticket.action => {
            let output = node
                .output
                .slot()
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

struct ResolvedActionCompletion {
    step: StepIdx,
    next: StepIdx,
}
