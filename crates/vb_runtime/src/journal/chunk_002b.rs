impl StorageRuntimeJournal {
    fn action_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
        match event {
            event @ (RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }) => simple_action_event(event, seq),
            event @ (RuntimeJournalEvent::ActionScheduledTicket { .. }
            | RuntimeJournalEvent::ActionAbandoned { .. }) => ticket_action_event(event, seq),
            event @ RuntimeJournalEvent::ActionCompletedEnvelope { .. } => {
                completed_envelope_event(event, seq)
            }
            event @ RuntimeJournalEvent::ActionFailed { .. } => failed_action_event(event, seq),
            _ => None,
        }
    }

    fn boundary_storage_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            event @ RuntimeJournalEvent::RetryTimerScheduled { .. } => {
                retry_timer_event(event, seq)
            }
            event @ (RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::AskTimedOut { .. }) => Ok(wait_ask_event(event, seq)),
            event @ RuntimeJournalEvent::SlotWritten { .. } => slot_written_event(event, seq),
            _ => Ok(None),
        }
    }
}

fn simple_action_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
    match event {
        RuntimeJournalEvent::ActionScheduled { run, step, action } => {
            Some(JournalEvent::ActionScheduled {
                run,
                seq,
                step,
                action,
                attempt: 1,
            })
        }
        RuntimeJournalEvent::ActionCompleted { run, step, action } => {
            Some(JournalEvent::ActionCompletedEvent {
                run,
                seq,
                step,
                action,
                attempt: 1,
            })
        }
        _ => None,
    }
}

fn ticket_action_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
    match event {
        RuntimeJournalEvent::ActionScheduledTicket {
            ticket,
            input,
            output,
            action_abi_digest,
        } => Some(JournalEvent::ActionScheduledTicket {
            run: ticket.run,
            seq,
            ticket,
            input,
            output,
            action_abi_digest,
        }),
        RuntimeJournalEvent::ActionAbandoned { ticket } => Some(JournalEvent::ActionAbandoned {
            run: ticket.run,
            seq,
            ticket,
        }),
        _ => None,
    }
}

fn completed_envelope_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
    match event {
        RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket,
            output,
            value,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        } => Some(JournalEvent::ActionCompletedEnvelope {
            run: ticket.run,
            seq,
            ticket,
            output,
            outcome: vb_storage::DurableActionOutcome::Ready,
            value,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        }),
        _ => None,
    }
}

fn failed_action_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
    match event {
        RuntimeJournalEvent::ActionFailed {
            run,
            step,
            action,
            attempt,
        } => Some(JournalEvent::ActionFailedEvent {
            run,
            seq,
            step,
            action,
            attempt,
        }),
        _ => None,
    }
}

fn retry_timer_event(
    event: RuntimeJournalEvent,
    seq: EventSeq,
) -> RuntimeResult<Option<JournalEvent>> {
    match event {
        RuntimeJournalEvent::RetryTimerScheduled { run, step, timer } => {
            timer.validate()?;
            let attempt = timer.failed_ticket.run_attempt;
            Ok(Some(JournalEvent::RetryTimerScheduled {
                run,
                seq,
                step,
                attempt,
                timer,
            }))
        }
        _ => Ok(None),
    }
}

fn wait_ask_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
    match event {
        event @ (RuntimeJournalEvent::WaitScheduled { .. }
        | RuntimeJournalEvent::WaitResolved { .. }) => wait_event(event, seq),
        event @ (RuntimeJournalEvent::AskScheduled { .. }
        | RuntimeJournalEvent::AskAnswered { .. }
        | RuntimeJournalEvent::AskTimedOut { .. }) => ask_event(event, seq),
        _ => None,
    }
}

fn wait_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
    match event {
        RuntimeJournalEvent::WaitScheduled { run, step } => {
            Some(JournalEvent::WaitScheduledEvent {
                run,
                seq,
                step,
                attempt: 1,
            })
        }
        RuntimeJournalEvent::WaitResolved { run, step } => Some(JournalEvent::WaitResolvedEvent {
            run,
            seq,
            step,
            attempt: 1,
        }),
        _ => None,
    }
}

fn ask_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
    match event {
        RuntimeJournalEvent::AskScheduled { run, step } => Some(JournalEvent::AskScheduledEvent {
            run,
            seq,
            step,
            attempt: 1,
        }),
        RuntimeJournalEvent::AskAnswered { run, step, .. } => {
            Some(JournalEvent::AskAnsweredEvent {
                run,
                seq,
                step,
                attempt: 1,
            })
        }
        RuntimeJournalEvent::AskTimedOut { run, step } => Some(JournalEvent::AskTimedOutEvent {
            run,
            seq,
            step,
            attempt: 1,
        }),
        _ => None,
    }
}
fn slot_written_event(
    event: RuntimeJournalEvent,
    seq: EventSeq,
) -> RuntimeResult<Option<JournalEvent>> {
    match event {
        RuntimeJournalEvent::SlotWritten {
            run,
            slot,
            value,
            taint,
            extra,
        } => Ok(Some(JournalEvent::SlotWrittenEvent {
            run,
            seq,
            slot,
            value: Some(value),
            extra: encoded_slot_taint_extra(taint, extra)?,
            attempt: 1,
        })),
        _ => Ok(None),
    }
}
enum StorageEventFamily {
    Run,
    Action,
    Boundary,
}

fn storage_event_family(event: &RuntimeJournalEvent) -> StorageEventFamily {
    use RuntimeJournalEvent::*;
    use StorageEventFamily::{Action, Boundary, Run};
    match event {
        RunSubmitted { .. } | RunAdmission { .. } | RunBudgetOrigin { .. } => Run,
        CompatibilityDecision { .. } | TenantAdmission { .. } => Run,
        RetryCheckpoint { .. } | Resumed { .. } => Run,
        RunFinished { .. } | RunFailed { .. } | RunCancelled { .. } | RunKilled { .. } => Run,
        StepStarted { .. } | StepSucceeded { .. } | StepFailed { .. } => Run,
        ActionScheduled { .. } | ActionCompleted { .. } | ActionFailed { .. } => Action,
        ActionScheduledTicket { .. } | ActionCompletedEnvelope { .. } | ActionAbandoned { .. } => {
            Action
        }
        WaitScheduled { .. } | WaitResolved { .. } | RetryTimerScheduled { .. } => Boundary,
        AskScheduled { .. } | AskAnswered { .. } | AskTimedOut { .. } => Boundary,
        SlotWritten { .. } => Boundary,
    }
}
