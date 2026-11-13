impl StorageRuntimeJournal {
    fn run_status_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::RunSubmitted { run, workflow } => {
                Ok(Some(JournalEvent::RunAccepted { run, seq, workflow }))
            }
            RuntimeJournalEvent::RunFailed { run, attempt } => {
                Ok(Some(JournalEvent::RunFailedEvent { run, seq, attempt }))
            }
            RuntimeJournalEvent::RunKilled { run, attempt } => {
                Ok(Some(JournalEvent::RunKilled { run, seq, attempt }))
            }
            RuntimeJournalEvent::Resumed { run, timestamp } => {
                resumed_storage_event(run, seq, timestamp).map(Some)
            }
            _ => Ok(None),
        }
    }

    fn run_finished_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::RunFinished {
                run,
                result,
                attempt,
            } => Ok(Some(JournalEvent::RunFinished {
                run,
                seq,
                result,
                attempt,
            })),
            _ => Ok(None),
        }
    }

    fn run_cancelled_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::RunCancelled {
                run,
                reason,
                attempt,
            } => Ok(Some(JournalEvent::RunCancelled {
                run,
                seq,
                attempt,
                reason,
            })),
            _ => Ok(None),
        }
    }

    fn run_retry_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::RetryCheckpoint {
                run,
                attempt,
                step,
                payload_digest,
                payload,
            } => Ok(Some(JournalEvent::RetryCheckpoint {
                run,
                seq,
                attempt,
                step,
                payload_digest,
                payload,
            })),
            _ => Ok(None),
        }
    }
}
