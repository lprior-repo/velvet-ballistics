impl StorageRuntimeJournal {
    fn run_compatibility_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::CompatibilityDecision { run, decision } => {
                Ok(Some(JournalEvent::CompatibilityDecisionEvent {
                    run,
                    seq,
                    decision,
                }))
            }
            RuntimeJournalEvent::RunBudgetOrigin {
                run,
                started_millis,
            } => Ok(Some(JournalEvent::RunBudgetOriginEvent {
                run,
                seq,
                started_millis,
            })),
            _ => Ok(None),
        }
    }

    fn run_tenant_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::TenantAdmission {
                run,
                tenant,
                cost_units,
            } => Ok(Some(JournalEvent::TenantAdmissionEvent {
                run,
                seq,
                tenant,
                cost_units,
            })),
            _ => Ok(None),
        }
    }

    fn run_step_status_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::StepStarted { run, step, attempt } => {
                Ok(Some(JournalEvent::StepStarted {
                    run,
                    seq,
                    step,
                    attempt,
                }))
            }
            RuntimeJournalEvent::StepFailed { run, step, attempt } => {
                Ok(Some(JournalEvent::StepFailed {
                    run,
                    seq,
                    step,
                    attempt,
                }))
            }
            _ => Ok(None),
        }
    }

    fn run_step_success_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::StepSucceeded {
                run, step, output, ..
            } => Ok(Some(JournalEvent::StepSucceeded {
                run,
                seq,
                step,
                output,
            })),
            _ => Ok(None),
        }
    }
}
