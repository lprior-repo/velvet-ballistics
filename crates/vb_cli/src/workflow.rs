//! Workflow execution helpers for velvet-ballistics.
#![forbid(unsafe_code)]

use crate::args::DurabilityMode;
use std::num::NonZeroUsize;
use std::path::Path;
use std::process::ExitCode;
use vb_core::{CompiledWorkflow, RunId, SlotIdx, SlotValue};
use vb_runtime::journal::{RuntimeJournal, SharedRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_runtime::trace::TraceEvent;

#[non_exhaustive]
pub(crate) enum InputMappingError {
    DecodeFailed,
    SlotCountExceeded,
    SlotIndexOutOfRange,
}

impl std::fmt::Display for InputMappingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::DecodeFailed => "INPUT_MAPPING_FAILED: input-bin decode failed",
            Self::SlotCountExceeded => {
                "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count"
            }
            Self::SlotIndexOutOfRange => {
                "INPUT_MAPPING_FAILED: input slot index out of range"
            }
        };
        write!(formatter, "{msg}")
    }
}

pub(crate) fn run_compiled_workflow(
    compiled: &CompiledWorkflow,
    inputs: Box<[(SlotIdx, SlotValue)]>,
    durability: DurabilityMode,
    db: Option<&Path>,
) -> ExitCode {
    let run_id = RunId::new(1);
    let Some(shard_count) = NonZeroUsize::new(1) else {
        crate::errln!("runtime configuration error: shard count must be non-zero");
        return ExitCode::FAILURE;
    };
    let config = ShardConfig::default();
    let journal = match runtime_journal_for_mode(durability, db) {
        Ok(journal) => journal,
        Err(code) => return code,
    };
    let mut runtime = Runtime::new_with_journal(shard_count, config, journal);

    if let Err(e) = runtime.submit_compiled_with_inputs(run_id, compiled.clone(), inputs) {
        crate::errln!("runtime submit error: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = runtime.tick_all() {
        crate::errln!("runtime tick error: {e}");
        return ExitCode::FAILURE;
    }

    let counters = runtime.counters_snapshot();
    let traces = runtime.drain_trace();
    crate::outln!(
        "run {}: submitted={} completed={} failed={} steps={}",
        run_id.get(),
        counters.runs_submitted,
        counters.runs_completed,
        counters.runs_failed,
        counters.steps_executed
    );
    for trace in &traces {
        print_trace_event(trace);
    }

    if counters.runs_failed != 0 {
        crate::errln!("run failed");
        return ExitCode::FAILURE;
    }
    if counters.runs_completed != 0 {
        crate::outln!("run completed");
    } else {
        crate::outln!("run accepted but not terminal after one runtime tick");
    }

    ExitCode::SUCCESS
}

fn runtime_journal_for_mode(
    durability: DurabilityMode,
    db: Option<&Path>,
) -> Result<SharedRuntimeJournal, ExitCode> {
    match durability {
        DurabilityMode::None => Ok(vb_runtime::journal::NoopRuntimeJournal::shared()),
        DurabilityMode::Journaled => open_storage_runtime_journal(db, false),
        DurabilityMode::Strict => open_storage_runtime_journal(db, true),
    }
}

fn open_storage_runtime_journal(
    db: Option<&Path>,
    strict: bool,
) -> Result<SharedRuntimeJournal, ExitCode> {
    let Some(path) = db else {
        crate::errln!("--db is required when --durability is strict or journaled");
        return Err(ExitCode::FAILURE);
    };
    let journal = match vb_storage::FjallJournal::open(path, None) {
        Ok(journal) => std::sync::Arc::new(journal),
        Err(e) => {
            crate::errln!("error opening journal at {}: {e}", path.display());
            return Err(ExitCode::FAILURE);
        }
    };
    if strict {
        return Ok(vb_runtime::journal::StorageRuntimeJournal::shared_strict(
            journal,
        ));
    }
    Ok(vb_runtime::journal::StorageRuntimeJournal::shared_journaled(
        journal,
    ))
}

pub(crate) fn print_trace_event(event: &TraceEvent) {
    match event {
        TraceEvent::StepStarted { step, .. } => {
            crate::outln!("  trace: StepStarted step={}", step.get());
        }
        TraceEvent::StepEnded { step, .. } => {
            crate::outln!("  trace: StepEnded step={}", step.get());
        }
        TraceEvent::SlotWritten { slot, .. } => {
            crate::outln!("  trace: SlotWritten slot={}", slot.get());
        }
        TraceEvent::ActionScheduled { step, .. } => {
            crate::outln!("  trace: ActionScheduled step={}", step.get());
        }
        TraceEvent::ActionCompleted { step, .. } => {
            crate::outln!("  trace: ActionCompleted step={}", step.get());
        }
        TraceEvent::ActionFailed { step, .. } => {
            crate::outln!("  trace: ActionFailed step={}", step.get());
        }
        TraceEvent::AskAnswered { step, slot, .. } => {
            crate::outln!("  trace: AskAnswered step={} slot={}", step.get(), slot.get());
        }
        TraceEvent::RunSubmitted { .. } => {
            crate::outln!("  trace: RunSubmitted");
        }
        TraceEvent::RunFinished { .. } => {
            crate::outln!("  trace: RunFinished");
        }
        TraceEvent::RunFailed { .. } => {
            crate::outln!("  trace: RunFailed");
        }
        TraceEvent::RunCancelled { .. } => {
            crate::outln!("  trace: RunCancelled");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_mapping_error_decode_failed_displays_correct_message() {
        assert_eq!(
            InputMappingError::DecodeFailed.to_string(),
            "INPUT_MAPPING_FAILED: input-bin decode failed"
        );
    }

    #[test]
    fn input_mapping_error_slot_count_exceeded_displays_correct_message() {
        assert_eq!(
            InputMappingError::SlotCountExceeded.to_string(),
            "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count"
        );
    }

    #[test]
    fn input_mapping_error_slot_index_out_of_range_displays_correct_message() {
        assert_eq!(
            InputMappingError::SlotIndexOutOfRange.to_string(),
            "INPUT_MAPPING_FAILED: input slot index out of range"
        );
    }

    #[test]
    fn print_trace_event_step_started_does_not_panic() {
        let event = TraceEvent::StepStarted { step: vb_core::StepIdx::new(1) };
        print_trace_event(&event);
    }

    #[test]
    fn print_trace_event_step_ended_does_not_panic() {
        let event = TraceEvent::StepEnded { step: vb_core::StepIdx::new(1) };
        print_trace_event(&event);
    }

    #[test]
    fn print_trace_event_action_scheduled_does_not_panic() {
        let event = TraceEvent::ActionScheduled { step: vb_core::StepIdx::new(1) };
        print_trace_event(&event);
    }

    #[test]
    fn print_trace_event_run_submitted_does_not_panic() {
        let event = TraceEvent::RunSubmitted {};
        print_trace_event(&event);
    }

    #[test]
    fn print_trace_event_run_finished_does_not_panic() {
        let event = TraceEvent::RunFinished {};
        print_trace_event(&event);
    }

    #[test]
    fn print_trace_event_run_failed_does_not_panic() {
        let event = TraceEvent::RunFailed {};
        print_trace_event(&event);
    }

    #[test]
    fn print_trace_event_run_cancelled_does_not_panic() {
        let event = TraceEvent::RunCancelled {};
        print_trace_event(&event);
    }

    #[test]
    fn print_trace_event_slot_written_does_not_panic() {
        let event = TraceEvent::SlotWritten { slot: vb_core::SlotIdx::new(0) };
        print_trace_event(&event);
    }
}
