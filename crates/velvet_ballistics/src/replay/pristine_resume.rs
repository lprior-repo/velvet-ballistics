use crate::app_impl::prelude::*;
use crate::run_compiled_runtime::completion_output::{
    completion_after_execution, completion_json, completion_text, matching_completion,
};
use vb_runtime::runtime::completion_observation::{
    CompletionObservation, CompletionObservationSlot,
};

pub(super) fn pristine_resume_candidate(error: &vb_core::errors::CoreError) -> bool {
    matches!(
        error,
        vb_core::errors::CoreError::LifecycleDuplicateRequest {
            command: Some("resume"),
            ..
        } | vb_core::errors::CoreError::LifecycleInvalidTransition {
            command: Some("resume"),
            ..
        }
    )
}

pub(super) fn select_pristine_resume(
    run: vb_core::RunId,
    original: vb_core::errors::CoreError,
    events: Result<Vec<vb_storage::JournalEvent>, vb_storage::JournalError>,
) -> Result<(), vb_core::errors::CoreError> {
    use vb_storage::{JournalEvent, RunInputBinding, RunPolicyPin};
    if pristine_resume_candidate(&original)
        && let Ok(events) = events
        && let [
            JournalEvent::RunAccepted {
                run: accepted,
                seq: first,
                ..
            },
            JournalEvent::RunAdmission {
                run: admitted,
                seq: second,
                policy: vb_core::RuntimePolicy::Strict,
                input_digest: RunInputBinding::Digest(_),
                policy_pin: RunPolicyPin::Pinned(_),
                ..
            },
        ] = events.as_slice()
        && *accepted == run
        && *admitted == run
        && first.get() == 0
        && second.get() == 1
    {
        return Ok(());
    }
    Err(original)
}

fn recover_pristine_once(
    runtime: &mut vb_runtime::runtime::Runtime,
    run: vb_core::RunId,
) -> Result<Option<CompletionObservation>, String> {
    use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
    match runtime
        .recover_product(run)
        .map_err(|error| format!("recovery: {error}"))?
    {
        RuntimeRecoveryProduct::Resumable(product)
            if product.boundary_kind() == RecoveredRunBoundaryKind::None => {}
        other => {
            return Err(format!(
                "pristine recovery requires a boundary-free product: {other:?}"
            ));
        }
    }
    runtime
        .recover_and_resume(run)
        .map_err(|error| format!("recovery activation: {error}"))?;
    let mut slots = [CompletionObservationSlot::default()];
    let observed = runtime
        .tick_all_observed(&mut slots)
        .map_err(|error| format!("observation preflight: {error:?}"))?;
    let [slot] = slots;
    completion_after_execution(observed, slot).map_err(|error| format!("runtime tick: {error}"))
}

fn retain_resume_failure(failure: &mut Option<String>, detail: String) {
    match failure {
        Some(primary) => {
            primary.push_str("; cleanup: ");
            primary.push_str(&detail);
        }
        None => *failure = Some(detail),
    }
}

fn finalize_pristine_resume(
    mut runtime: vb_runtime::runtime::Runtime,
    journal: Arc<vb_storage::FjallJournal>,
    completed: bool,
    failure: &mut Option<String>,
) {
    let pending = runtime.pending_boundary_snapshot(1);
    if pending.command_queue_depth() != 0 {
        retain_resume_failure(
            failure,
            "queued commands remain; shutdown was not driven".into(),
        );
    } else {
        match runtime.shutdown_graceful() {
            Ok(report) => {
                let remaining = report.remaining();
                if report.commands_drained() != 0
                    || remaining.command_queue_depth() != 0
                    || remaining.active_run_count() != pending.active_run_count()
                    || remaining.pending_ask_count() != pending.pending_ask_count()
                    || remaining.pending_action_count() != pending.pending_action_count()
                    || remaining.pending_timer_count() != pending.pending_timer_count()
                    || (completed
                        && (remaining.active_run_count() != 0
                            || remaining.pending_ask_count() != 0
                            || remaining.pending_action_count() != 0
                            || remaining.pending_timer_count() != 0))
                    || (remaining.pending_ask_count() == 1 && remaining.active_run_count() != 1)
                {
                    retain_resume_failure(
                        failure,
                        format!("unexpected shutdown report: {report:?}"),
                    );
                }
            }
            Err(error) => retain_resume_failure(failure, format!("shutdown: {error:?}")),
        }
    }
    drop(runtime);
    match Arc::try_unwrap(journal) {
        Ok(mut owned) => {
            if let Err(error) = owned.close() {
                retain_resume_failure(failure, format!("journal close: {error}"));
            }
            drop(owned);
        }
        Err(shared) => {
            retain_resume_failure(failure, "journal owner survived runtime shutdown".into());
            drop(shared);
        }
    }
}

pub(super) fn run_pristine_resume(
    run_id: &str,
    run: vb_core::RunId,
    journal: vb_storage::FjallJournal,
    output: OutputFormat,
) -> ExitCode {
    let journal = Arc::new(journal);
    let mut runtime = vb_runtime::runtime::Runtime::new(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
        vb_runtime::journal::StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    );
    let (candidate, mut failure) = match recover_pristine_once(&mut runtime, run) {
        Ok(candidate) => (candidate, None),
        Err(error) => (None, Some(error)),
    };
    let counters = runtime.counters_snapshot();
    let completed = counters.runs_completed == 1 && counters.runs_failed == 0;
    let completion = matching_completion(candidate, completed, run, core::num::NonZeroU16::MIN);
    let success = completed && completion.is_some();
    let status = if counters.runs_failed != 0 {
        "failed"
    } else if completed {
        "completed"
    } else {
        "waiting"
    };
    finalize_pristine_resume(runtime, journal, completed, &mut failure);
    if let Some(error) = failure {
        crate::run_compiled_runtime::report_runtime_error(
            format_args!("resume runtime: {error}"),
            output,
        );
        return CliExitCode::RuntimeFailed.into();
    }
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "success": success,
                "run_id": run_id,
                "status": status,
                "runtime": {
                    "submitted": counters.runs_submitted,
                    "completed": counters.runs_completed,
                    "failed": counters.runs_failed,
                    "steps": counters.steps_executed
                },
                "completion": completion_json(completion)
            }),
            output,
        );
    } else {
        outln!(
            "run {run_id}: submitted={} completed={} failed={} steps={}",
            counters.runs_submitted,
            counters.runs_completed,
            counters.runs_failed,
            counters.steps_executed
        );
        if let Some(line) = completion_text(completion) {
            outln!("{line}");
        }
        if success {
            outln!("run completed");
        } else {
            outln!("run {status}; no successful completion observed");
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        CliExitCode::RuntimeFailed.into()
    }
}

#[cfg(test)]
#[path = "pristine_resume_tests.rs"]
mod tests;
