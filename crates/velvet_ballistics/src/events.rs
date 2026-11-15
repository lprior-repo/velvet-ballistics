//! Module: events

use crate::app_impl::prelude::*;

mod command;

pub(crate) fn cmd_events(
    run_id: &str,
    db: &std::path::Path,
    output: OutputFormat,
    status: Option<EventStatus>,
    limit: Option<i64>,
) -> ExitCode {
    match command::read(run_id, db, output) {
        Ok(events) => command::render(run_id, command::select(events, status, limit), output),
        Err(code) => code,
    }
}

pub(crate) fn print_event(event: &vb_storage::JournalEvent) {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, .. } => {
            outln!("  seq={}: RunAccepted", seq.get());
        }
        vb_storage::JournalEvent::RunAdmission { seq, policy, .. } => {
            outln!("  seq={}: RunAdmission policy={policy:?}", seq.get());
        }
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            outln!("  seq={}: StepStarted step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => {
            outln!(
                "  seq={}: StepSucceeded step={} output={}",
                seq.get(),
                step.get(),
                output.get()
            );
        }
        vb_storage::JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => {
            outln!(
                "  seq={}: ActionScheduled step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => {
            outln!(
                "  seq={}: ActionCompleted step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => {
            outln!(
                "  seq={}: ActionFailed step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            outln!("  seq={}: SlotWritten slot={}", seq.get(), slot.get());
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: WaitScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: AskScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            outln!("  seq={}: AskAnswered step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: RetryScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            outln!("  seq={}: RunCancelled", seq.get());
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            outln!("  seq={}: RunFinished result={}", seq.get(), result.get());
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            outln!("  seq={}: RunFailed", seq.get());
        }
        vb_storage::JournalEvent::RunResumed { run, .. } => {
            outln!("  RunResumed run={}", run.get());
        }
        vb_storage::JournalEvent::RunRetried { run, .. } => {
            outln!("  RunRetried run={}", run.get());
        }
        vb_storage::JournalEvent::RunAnswered { run, slot_idx, .. } => {
            outln!("  RunAnswered run={} slot={}", run.get(), slot_idx.get());
        }
        _ => {
            outln!("  Unknown");
        }
    }
}
