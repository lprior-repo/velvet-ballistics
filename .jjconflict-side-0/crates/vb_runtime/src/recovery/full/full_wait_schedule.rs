use vb_core::ids::StepIdx;

/// Folds journal events into the currently unresolved wait schedule for one step.
///
/// Mirrors `full_ask_schedule::pending_ask_schedule_for_step`: a
/// `WaitScheduledEvent` with a nonzero attempt opens (or replaces) the pending
/// schedule, a zero-attempt schedule clears it, and a `WaitResolvedEvent`
/// carrying the scheduled attempt closes it. Returns the boundary sequence and
/// attempt of the still-open schedule, or `None` when the step has no
/// unresolved wait at the recovery point.
pub(crate) fn pending_wait_schedule_for_step(
    events: &[vb_storage::JournalEvent],
    step: StepIdx,
) -> Option<(vb_storage::EventSeq, u16)> {
    events
        .iter()
        .filter_map(|event| wait_schedule_mutation(event, step))
        .fold(None, apply_wait_schedule_mutation)
}

#[derive(Clone, Copy)]
enum WaitScheduleMutation {
    Scheduled(vb_storage::EventSeq, u16),
    Cleared,
    Resolved(u16),
}

fn wait_schedule_mutation(
    event: &vb_storage::JournalEvent,
    step: StepIdx,
) -> Option<WaitScheduleMutation> {
    scheduled_wait_mutation(event, step).or_else(|| resolved_wait_mutation(event, step))
}

fn scheduled_wait_mutation(
    event: &vb_storage::JournalEvent,
    step: StepIdx,
) -> Option<WaitScheduleMutation> {
    match event {
        vb_storage::JournalEvent::WaitScheduledEvent {
            seq,
            step: scheduled_step,
            attempt,
            ..
        } if *scheduled_step == step && *attempt != 0 => {
            Some(WaitScheduleMutation::Scheduled(*seq, *attempt))
        }
        vb_storage::JournalEvent::WaitScheduledEvent {
            step: scheduled_step,
            attempt: 0,
            ..
        } if *scheduled_step == step => Some(WaitScheduleMutation::Cleared),
        _ => None,
    }
}

fn resolved_wait_mutation(
    event: &vb_storage::JournalEvent,
    step: StepIdx,
) -> Option<WaitScheduleMutation> {
    match event {
        vb_storage::JournalEvent::WaitResolvedEvent {
            step: resolved_step,
            attempt,
            ..
        } if *resolved_step == step => Some(WaitScheduleMutation::Resolved(*attempt)),
        _ => None,
    }
}

fn apply_wait_schedule_mutation(
    pending: Option<(vb_storage::EventSeq, u16)>,
    mutation: WaitScheduleMutation,
) -> Option<(vb_storage::EventSeq, u16)> {
    match mutation {
        WaitScheduleMutation::Scheduled(seq, attempt) => Some((seq, attempt)),
        WaitScheduleMutation::Cleared => None,
        WaitScheduleMutation::Resolved(attempt)
            if pending.is_some_and(|(_, scheduled_attempt)| scheduled_attempt == attempt) =>
        {
            None
        }
        WaitScheduleMutation::Resolved(_) => pending,
    }
}

#[cfg(test)]
mod tests {
    use super::pending_wait_schedule_for_step;
    use vb_core::ids::StepIdx;

    fn scheduled(seq: u64, step: u16, attempt: u16) -> vb_storage::JournalEvent {
        vb_storage::JournalEvent::WaitScheduledEvent {
            run: vb_core::ids::RunId::new(1),
            seq: vb_storage::EventSeq::new(seq),
            step: StepIdx::new(step),
            attempt,
        }
    }

    fn resolved(seq: u64, step: u16, attempt: u16) -> vb_storage::JournalEvent {
        vb_storage::JournalEvent::WaitResolvedEvent {
            run: vb_core::ids::RunId::new(1),
            seq: vb_storage::EventSeq::new(seq),
            step: StepIdx::new(step),
            attempt,
        }
    }

    #[test]
    fn unresolved_wait_schedule_reports_boundary() {
        let events = [scheduled(3, 0, 1)];
        assert_eq!(
            pending_wait_schedule_for_step(&events, StepIdx::ZERO),
            Some((vb_storage::EventSeq::new(3), 1))
        );
    }

    #[test]
    fn resolved_wait_schedule_clears_boundary() {
        let events = [scheduled(3, 0, 1), resolved(4, 0, 1)];
        assert_eq!(pending_wait_schedule_for_step(&events, StepIdx::ZERO), None);
    }

    #[test]
    fn mismatched_resolution_keeps_pending_schedule() {
        let events = [scheduled(3, 0, 1), resolved(4, 0, 2)];
        assert_eq!(
            pending_wait_schedule_for_step(&events, StepIdx::ZERO),
            Some((vb_storage::EventSeq::new(3), 1))
        );
    }

    #[test]
    fn other_step_schedule_is_ignored() {
        let events = [scheduled(3, 1, 1)];
        assert_eq!(pending_wait_schedule_for_step(&events, StepIdx::ZERO), None);
    }
}
