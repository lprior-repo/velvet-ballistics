use crate::io::outln;
use vb_storage::JournalEvent;

pub fn print_event(event: &JournalEvent) {
    match event {
        JournalEvent::RunAccepted { seq, .. } => {
            outln!("  seq={}: RunAccepted", seq.get());
        }
        JournalEvent::StepStarted { seq, step, .. } => {
            outln!("  seq={}: StepStarted step={}", seq.get(), step.get());
        }
        JournalEvent::StepSucceeded { seq, step, output, .. } => {
            outln!("  seq={}: StepSucceeded step={} output={}", seq.get(), step.get(), output.get());
        }
        JournalEvent::ActionScheduled { seq, step, action, .. } => {
            outln!("  seq={}: ActionScheduled step={} action={}", seq.get(), step.get(), action.get());
        }
        JournalEvent::ActionCompletedEvent { seq, step, action, .. } => {
            outln!("  seq={}: ActionCompleted step={} action={}", seq.get(), step.get(), action.get());
        }
        JournalEvent::ActionFailedEvent { seq, step, action, .. } => {
            outln!("  seq={}: ActionFailed step={} action={}", seq.get(), step.get(), action.get());
        }
        JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            outln!("  seq={}: SlotWritten slot={}", seq.get(), slot.get());
        }
        JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: WaitScheduled step={}", seq.get(), step.get());
        }
        JournalEvent::AskScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: AskScheduled step={}", seq.get(), step.get());
        }
        JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            outln!("  seq={}: AskAnswered step={}", seq.get(), step.get());
        }
        JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: RetryScheduled step={}", seq.get(), step.get());
        }
        JournalEvent::RunCancelled { seq, .. } => {
            outln!("  seq={}: RunCancelled", seq.get());
        }
        JournalEvent::RunFinished { seq, result, .. } => {
            outln!("  seq={}: RunFinished result={}", seq.get(), result.get());
        }
        JournalEvent::RunFailedEvent { seq, .. } => {
            outln!("  seq={}: RunFailed", seq.get());
        }
    }
}

pub fn cmd_replay(run_id: &str, db: &Path) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };

    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    match vb_storage::recovery::recover_full_journal(&journal, rid, &mut tracker, &[], &[]) {
        Ok(events) => {
            outln!("recovered {} event(s) for run {run_id}", events.len());
            for event in &events {
                print_event(event);
            }
            match vb_storage::recovery::extract_terminal(&events) {
                Some(terminal) => {
                    outln!("terminal: {}", event_name(terminal));
                }
                None => {
                    outln!("terminal: none");
                }
            }
        }
        Err(e) => {
            errln!("error replaying run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

pub fn event_name(event: &JournalEvent) -> &'static str {
    match event {
        JournalEvent::RunAccepted { .. } => "RunAccepted",
        JournalEvent::StepStarted { .. } => "StepStarted",
        JournalEvent::StepSucceeded { .. } => "StepSucceeded",
        JournalEvent::ActionScheduled { .. } => "ActionScheduled",
        JournalEvent::ActionCompletedEvent { .. } => "ActionCompleted",
        JournalEvent::ActionFailedEvent { .. } => "ActionFailed",
        JournalEvent::SlotWrittenEvent { .. } => "SlotWritten",
        JournalEvent::WaitScheduledEvent { .. } => "WaitScheduled",
        JournalEvent::AskScheduledEvent { .. } => "AskScheduled",
        JournalEvent::AskAnsweredEvent { .. } => "AskAnswered",
        JournalEvent::RetryScheduledEvent { .. } => "RetryScheduled",
        JournalEvent::RunCancelled { .. } => "RunCancelled",
        JournalEvent::RunFinished { .. } => "RunFinished",
        JournalEvent::RunFailedEvent { .. } => "RunFailed",
    }
}
