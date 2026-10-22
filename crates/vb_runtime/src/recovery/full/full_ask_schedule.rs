use vb_core::ids::StepIdx;

fn ask_schedule_mutation(
    event: &vb_storage::JournalEvent,
    step: StepIdx,
) -> Option<(vb_storage::EventSeq, u16)> {
    if let vb_storage::JournalEvent::AskScheduledEvent { step: event_step, .. } = event {
        if *event_step == step {
            return Some((event.seq(), 0));
        }
    }
    if let vb_storage::JournalEvent::AskAnsweredEvent { step: event_step, .. } = event {
        if *event_step == step {
            return None;
        }
    }
    None
}

pub(crate) fn pending_ask_schedule_for_step(
    events: &[vb_storage::JournalEvent],
    step: StepIdx,
) -> Option<(vb_storage::EventSeq, u16)> {
    let mut pending = None;
    for event in events {
        if let Some(result) = ask_schedule_mutation(event, step) {
            pending = Some(result);
        }
    }
    pending
}