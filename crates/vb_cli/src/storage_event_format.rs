#![forbid(unsafe_code)]
//! Journal event formatting for the storage command module.
//!
//! Extracted from `storage.rs` to keep that file under the 300-line
//! source cap. All formatters here are public to the parent module so
//! existing call sites in `cmd_events` and `cmd_replay` continue to work.

use vb_storage::JournalEvent;

/// Prints a single `JournalEvent` to stdout in text mode.
pub(crate) fn print_event(event: &JournalEvent) {
    match event {
        JournalEvent::RunAccepted { seq, .. } => {
            crate::outln!("  seq={}: RunAccepted", seq.get());
        }
        JournalEvent::RunAdmission { seq, policy, .. } => {
            crate::outln!("  seq={}: RunAdmission policy={policy:?}", seq.get());
        }
        JournalEvent::StepStarted { seq, step, .. } => {
            crate::outln!("  seq={}: StepStarted step={}", seq.get(), step.get());
        }
        JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => {
            crate::outln!(
                "  seq={}: StepSucceeded step={} output={}",
                seq.get(),
                step.get(),
                output.get()
            );
        }
        JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => {
            crate::outln!(
                "  seq={}: ActionScheduled step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => {
            crate::outln!(
                "  seq={}: ActionCompleted step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => {
            crate::outln!(
                "  seq={}: ActionFailed step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            crate::outln!("  seq={}: SlotWritten slot={}", seq.get(), slot.get());
        }
        JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            crate::outln!("  seq={}: WaitScheduled step={}", seq.get(), step.get());
        }
        JournalEvent::AskScheduledEvent { seq, step, .. } => {
            crate::outln!("  seq={}: AskScheduled step={}", seq.get(), step.get());
        }
        JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            crate::outln!("  seq={}: AskAnswered step={}", seq.get(), step.get());
        }
        JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            crate::outln!("  seq={}: RetryScheduled step={}", seq.get(), step.get());
        }
        JournalEvent::RunCancelled { seq, .. } => {
            crate::outln!("  seq={}: RunCancelled", seq.get());
        }
        JournalEvent::RunFinished { seq, result, .. } => {
            crate::outln!("  seq={}: RunFinished result={}", seq.get(), result.get());
        }
        JournalEvent::RunFailedEvent { seq, .. } => {
            crate::outln!("  seq={}: RunFailed", seq.get());
        }
        _ => {}
    }
}

/// Returns a static name for the event's variant (used for terminal
/// reporting in the replay command).
pub(crate) fn event_name(event: &JournalEvent) -> &'static str {
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
        _ => "UnknownEvent",
    }
}
