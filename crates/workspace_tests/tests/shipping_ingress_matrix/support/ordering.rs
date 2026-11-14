use super::*;
/// Asserts the master §49 ordering claims that every scenario relies on.
pub(crate) fn assert_ordering(events: &[JournalEvent], run: RunId) -> TestResult {
    let accepted = events
        .iter()
        .position(|event| matches!(event, JournalEvent::RunAccepted { run: actual, .. } if *actual == run))
        .ok_or("missing RunAccepted")?;
    let first_step_or_slot = events.iter().position(|event| {
        matches!(
            event,
            JournalEvent::StepStarted { run: actual, .. }
                | JournalEvent::SlotWrittenEvent { run: actual, .. }
                if *actual == run
        )
    });
    if let Some(index) = first_step_or_slot {
        assert!(
            accepted < index,
            "RunAccepted must precede step/slot events"
        );
    }

    let action_scheduled = events.iter().position(|event| {
        matches!(
            event,
            JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.run == run
        )
    });
    let action_completed = events.iter().position(|event| {
        matches!(
            event,
            JournalEvent::ActionCompletedEnvelope { ticket, .. } if ticket.run == run
        )
    });
    if let (Some(scheduled), Some(completed)) = (action_scheduled, action_completed) {
        assert!(
            scheduled < completed,
            "ActionScheduled must precede completion"
        );
    }

    let terminal = events.iter().position(|event| match event {
        JournalEvent::RunFinished { run: actual, .. }
        | JournalEvent::RunFailedEvent { run: actual, .. }
        | JournalEvent::RunCancelled { run: actual, .. }
        | JournalEvent::RunKilled { run: actual, .. } => *actual == run,
        _ => false,
    });
    if let Some(terminal) = terminal {
        for (index, event) in events.iter().enumerate() {
            if matches!(
                event,
                JournalEvent::WaitScheduledEvent { run: actual, .. }
                    | JournalEvent::AskScheduledEvent { run: actual, .. }
                    if *actual == run
            ) {
                assert!(
                    index < terminal,
                    "WaitScheduled/AskScheduled must precede the terminal event"
                );
            }
        }
    }

    if let Some(finished) = events.iter().position(
        |event| matches!(event, JournalEvent::RunFinished { run: actual, .. } if *actual == run),
    ) {
        if let JournalEvent::RunFinished { result, .. } = &events[finished] {
            let write = events.iter().position(|event| {
                matches!(
                    event,
                    JournalEvent::SlotWrittenEvent { run: actual, slot, .. }
                        if *actual == run && slot == result
                ) || matches!(
                    event,
                    JournalEvent::ActionCompletedEnvelope { run: actual, output, .. }
                        if *actual == run && output == result
                )
            });
            assert!(
                write.is_some_and(|index| index < finished),
                "RunFinished result slot must be written before the terminal event"
            );
        }
        assert!(
            !events.iter().skip(finished).skip(1).any(|event| matches!(
                event,
                JournalEvent::RunFinished { .. }
                    | JournalEvent::RunFailedEvent { .. }
                    | JournalEvent::RunCancelled { .. }
                    | JournalEvent::RunKilled { .. }
            )),
            "RunFinished must be the final terminal event"
        );
    }
    Ok(())
}
