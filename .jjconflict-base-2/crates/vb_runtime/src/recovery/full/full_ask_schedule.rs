use vb_core::ids::StepIdx;

#[derive(Clone, Copy)]
enum AskScheduleMutation {
    Scheduled(vb_storage::EventSeq, u16),
    Cleared,
    Resolved(u16),
}

pub(crate) fn pending_ask_schedule_for_step(
    events: &[vb_storage::JournalEvent],
    step: StepIdx,
) -> Option<(vb_storage::EventSeq, u16)> {
    events
        .iter()
        .filter_map(|event| ask_schedule_mutation(event, step))
        .fold(None, apply_ask_schedule_mutation)
}

fn ask_schedule_mutation(
    event: &vb_storage::JournalEvent,
    step: StepIdx,
) -> Option<AskScheduleMutation> {
    scheduled_ask_mutation(event, step).or_else(|| resolved_ask_mutation(event, step))
}

fn scheduled_ask_mutation(
    event: &vb_storage::JournalEvent,
    step: StepIdx,
) -> Option<AskScheduleMutation> {
    match event {
        vb_storage::JournalEvent::AskScheduledEvent {
            seq,
            step: scheduled_step,
            attempt,
            ..
        } if *scheduled_step == step && *attempt != 0 => {
            Some(AskScheduleMutation::Scheduled(*seq, *attempt))
        }
        vb_storage::JournalEvent::AskScheduledEvent {
            step: scheduled_step,
            attempt: 0,
            ..
        } if *scheduled_step == step => Some(AskScheduleMutation::Cleared),
        _ => None,
    }
}

fn resolved_ask_mutation(
    event: &vb_storage::JournalEvent,
    step: StepIdx,
) -> Option<AskScheduleMutation> {
    match event {
        vb_storage::JournalEvent::AskAnsweredEvent {
            step: resolved_step,
            attempt,
            ..
        }
        | vb_storage::JournalEvent::AskTimedOutEvent {
            step: resolved_step,
            attempt,
            ..
        } if *resolved_step == step => Some(AskScheduleMutation::Resolved(*attempt)),
        _ => None,
    }
}

fn apply_ask_schedule_mutation(
    pending: Option<(vb_storage::EventSeq, u16)>,
    mutation: AskScheduleMutation,
) -> Option<(vb_storage::EventSeq, u16)> {
    match mutation {
        AskScheduleMutation::Scheduled(seq, attempt) => Some((seq, attempt)),
        AskScheduleMutation::Cleared => None,
        AskScheduleMutation::Resolved(attempt)
            if pending.is_some_and(|(_, scheduled_attempt)| scheduled_attempt == attempt) =>
        {
            None
        }
        AskScheduleMutation::Resolved(_) => pending,
    }
}
