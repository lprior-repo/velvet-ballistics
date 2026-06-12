#![forbid(unsafe_code)]
//! Trace event formatting for the compiled-workflow runner.
//!
//! Extracted from `run_compiled_runtime.rs` to keep that file under the
//! 300-line source cap. All formatters here are public to the parent
//! module so existing call sites in `run_compiled_workflow` and
//! `print_trace_event` continue to work.

/// Prints a single `TraceEvent` to stdout in text mode.
pub(crate) fn print_trace_event(event: &vb_runtime::trace::TraceEvent) {
    match event {
        vb_runtime::trace::TraceEvent::StepStarted { step, .. } => {
            crate::outln!("  trace: StepStarted step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::StepEnded { step, .. } => {
            crate::outln!("  trace: StepEnded step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::SlotWritten { slot, .. } => {
            crate::outln!("  trace: SlotWritten slot={}", slot.get());
        }
        vb_runtime::trace::TraceEvent::ActionScheduled { step, .. } => {
            crate::outln!("  trace: ActionScheduled step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::ActionCompleted { step, .. } => {
            crate::outln!("  trace: ActionCompleted step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::ActionFailed { step, .. } => {
            crate::outln!("  trace: ActionFailed step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::AskAnswered { step, slot, .. } => {
            crate::outln!(
                "  trace: AskAnswered step={} slot={}",
                step.get(),
                slot.get()
            );
        }
        vb_runtime::trace::TraceEvent::RunSubmitted { .. } => {
            crate::outln!("  trace: RunSubmitted");
        }
        vb_runtime::trace::TraceEvent::RunFinished { .. } => {
            crate::outln!("  trace: RunFinished");
        }
        vb_runtime::trace::TraceEvent::RunFailed { .. } => {
            crate::outln!("  trace: RunFailed");
        }
        vb_runtime::trace::TraceEvent::RunCancelled { .. } => {
            crate::outln!("  trace: RunCancelled");
        }
        _ => {
            crate::outln!("  trace: Unknown");
        }
    }
}
