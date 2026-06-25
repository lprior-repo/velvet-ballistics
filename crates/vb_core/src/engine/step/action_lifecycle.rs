use crate::EngineSignal;
use crate::action::{
    ActionFailure, ActionFailureCode, ActionFailureReport, ActionJournalEvent,
    ActionResumeRejection, ActionTicket, RetryPolicy, action_ticket_has_valid_key,
};
use crate::engine::error_routing::{ErrorHandlerOutcome, route_error_handler};
use crate::errors::EngineError;
use crate::frame::{RunFrame, StepState};
use crate::ids::{ActionId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use crate::workflow::{CompiledNodeKind, CompiledWorkflow};

struct ResolvedActionResume {
    step: StepIdx,
    next: Option<StepIdx>,
    output: Option<SlotIdx>,
}

pub fn journal_action_suspended(
    ticket: ActionTicket,
    action: ActionId,
    input_slot: SlotIdx,
    output_slot: SlotIdx,
    step: StepIdx,
) -> ActionJournalEvent {
    ActionJournalEvent::Suspended {
        ticket,
        attempt: ticket.attempt,
        action,
        input_slot,
        output_slot,
        step,
    }
}

pub fn resume_action_completion(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    ticket: ActionTicket,
    output_slot: SlotIdx,
    output_value: SlotValue,
    output_taint: Taint,
) -> Result<(EngineSignal, ActionJournalEvent), EngineError> {
    let context = resolve_completion_context(plan, run, ticket, output_slot)?;
    run.write_slot_with_taint(output_slot, output_value, output_taint)?;
    run.mark_succeeded(context.step)?;
    run.set_pc(context.next)?;
    run.increment_executed()?;

    let journal = ActionJournalEvent::Completed {
        ticket,
        attempt: ticket.attempt,
        output_slot,
        output_taint,
    };
    Ok((EngineSignal::Continue, journal))
}

pub fn resume_action_failure(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    ticket: ActionTicket,
    failure_code: ActionFailureCode,
    retry_policy: RetryPolicy,
) -> Result<(EngineSignal, ActionJournalEvent), EngineError> {
    let context = resolve_action_resume(plan, run, ticket)?;
    run.mark_failed(context.step)?;

    let journal = ActionJournalEvent::Failed {
        ticket,
        attempt: ticket.attempt,
        code: failure_code,
        retry_policy,
    };

    let report = ActionFailureReport::new(
        context.step,
        ticket.action,
        ActionFailure {
            code: failure_code,
            retry_policy,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        },
    );
    let error = EngineError::ActionFailed { report };
    match route_error_handler(plan, run, context.step, &error)? {
        ErrorHandlerOutcome::Routed => Ok((EngineSignal::Continue, journal)),
        ErrorHandlerOutcome::NoHandler => Ok((EngineSignal::AwaitingAction, journal)),
    }
}

fn resolve_completion_context(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    ticket: ActionTicket,
    output_slot: SlotIdx,
) -> Result<ResolvedActionCompletion, EngineError> {
    let resume = resolve_action_resume(plan, run, ticket)?;
    let expected = resume
        .output
        .ok_or(EngineError::MissingOutputSlot { step: resume.step })?;
    if expected != output_slot {
        return Err(resume_rejection(ActionResumeRejection::OutputMismatch));
    }
    if output_slot.as_usize() >= usize::from(run.slot_count()) {
        return Err(EngineError::SlotOutOfBounds { slot: output_slot });
    }
    let next = resume
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
) -> Result<ResolvedActionResume, EngineError> {
    if ticket.run != run.run_id() {
        return Err(resume_rejection(ActionResumeRejection::RunMismatch));
    }
    if ticket.step != run.pc() {
        return Err(resume_rejection(ActionResumeRejection::StepNotCurrentPc));
    }
    validate_ticket_shape(ticket)?;
    if run.step_state(ticket.step)? != StepState::Running {
        return Err(resume_rejection(ActionResumeRejection::StepNotRunning));
    }

    let node = plan
        .node(ticket.step)
        .ok_or(EngineError::InvalidProgramCounter { step: ticket.step })?;
    match &node.kind {
        CompiledNodeKind::Do { action, .. } if *action == ticket.action => {
            Ok(ResolvedActionResume {
                step: ticket.step,
                next: node.next,
                output: node.output,
            })
        }
        CompiledNodeKind::Do { .. } => Err(resume_rejection(ActionResumeRejection::ActionMismatch)),
        _ => Err(resume_rejection(ActionResumeRejection::NonDoNode)),
    }
}

fn validate_ticket_shape(ticket: ActionTicket) -> Result<(), EngineError> {
    if ticket.attempt == 0 {
        return Err(resume_rejection(ActionResumeRejection::AttemptZero));
    }
    if ticket.capacity == 0 {
        return Err(resume_rejection(ActionResumeRejection::CapacityZero));
    }
    if ticket.attempt > ticket.capacity {
        return Err(resume_rejection(
            ActionResumeRejection::AttemptExceedsCapacity,
        ));
    }
    if !action_ticket_has_valid_key(ticket) {
        return Err(resume_rejection(
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

fn resume_rejection(rejection: ActionResumeRejection) -> EngineError {
    EngineError::ActionResumeRejected { rejection }
}

struct ResolvedActionCompletion {
    step: StepIdx,
    next: StepIdx,
}
