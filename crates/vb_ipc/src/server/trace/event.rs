//! Trace event kind mapping.
//!
//! Pure function: converts runtime [`TraceEvent`] into IPC serializable
//! [`IpcTraceEventKind`].

use vb_runtime::trace::TraceEvent;

use crate::IpcTraceEventKind;

/// Maps a runtime trace event to its IPC wire representation.
pub(super) fn trace_event_kind(event: &TraceEvent) -> IpcTraceEventKind {
    match event {
        TraceEvent::StepStarted { run, step } => IpcTraceEventKind::StepStarted {
            run: *run,
            step: *step,
        },
        TraceEvent::StepEnded { run, step } => IpcTraceEventKind::StepEnded {
            run: *run,
            step: *step,
        },
        TraceEvent::SlotWritten { run, slot, value } => IpcTraceEventKind::SlotWritten {
            run: *run,
            slot: *slot,
            value: value.clone(),
        },
        TraceEvent::ActionScheduled { run, step } => IpcTraceEventKind::ActionScheduled {
            run: *run,
            step: *step,
        },
        TraceEvent::ActionCompleted { run, step } => IpcTraceEventKind::ActionCompleted {
            run: *run,
            step: *step,
        },
        TraceEvent::ActionFailed { run, step, code } => IpcTraceEventKind::ActionFailed {
            run: *run,
            step: *step,
            code: *code,
        },
        TraceEvent::AskAnswered { run, step, slot } => IpcTraceEventKind::AskAnswered {
            run: *run,
            step: *step,
            slot: *slot,
        },
        TraceEvent::RunSubmitted { run } => IpcTraceEventKind::RunSubmitted { run: *run },
        TraceEvent::RunFinished { run } => IpcTraceEventKind::RunFinished { run: *run },
        TraceEvent::RunFailed { run } => IpcTraceEventKind::RunFailed { run: *run },
        TraceEvent::RunCancelled { run } => IpcTraceEventKind::RunCancelled { run: *run },
        _ => IpcTraceEventKind::Unknown,
    }
}
