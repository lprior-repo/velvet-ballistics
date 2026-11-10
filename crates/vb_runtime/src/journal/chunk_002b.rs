impl StorageRuntimeJournal {
    fn action_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
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
            RuntimeJournalEvent::ActionAbandoned { ticket } => {
                Some(JournalEvent::ActionAbandoned {
                    run: ticket.run,
                    seq,
                    ticket,
                })
            }
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::CompatibilityDecision { .. }
            | RuntimeJournalEvent::TenantAdmission { .. }
            | RuntimeJournalEvent::RetryCheckpoint { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::RunKilled { .. }
            | RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::AskTimedOut { .. }
            | RuntimeJournalEvent::SlotWritten { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. }
            | RuntimeJournalEvent::Resumed { .. } => None,
        }
    }

    fn boundary_storage_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::WaitScheduled { run, step } => {
                Ok(Some(JournalEvent::WaitScheduledEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
            RuntimeJournalEvent::WaitResolved { run, step } => {
                Ok(Some(JournalEvent::WaitResolvedEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
            RuntimeJournalEvent::AskScheduled { run, step } => {
                Ok(Some(JournalEvent::AskScheduledEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
            RuntimeJournalEvent::AskAnswered { run, step, .. } => {
                Ok(Some(JournalEvent::AskAnsweredEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
            RuntimeJournalEvent::AskTimedOut { run, step } => {
                Ok(Some(JournalEvent::AskTimedOutEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
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
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::CompatibilityDecision { .. }
            | RuntimeJournalEvent::TenantAdmission { .. }
            | RuntimeJournalEvent::RetryCheckpoint { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::RunKilled { .. }
            | RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }
            | RuntimeJournalEvent::ActionScheduledTicket { .. }
            | RuntimeJournalEvent::ActionCompletedEnvelope { .. }
            | RuntimeJournalEvent::ActionFailed { .. }
            | RuntimeJournalEvent::ActionAbandoned { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. }
            | RuntimeJournalEvent::Resumed { .. } => Ok(None),
        }
    }
}
