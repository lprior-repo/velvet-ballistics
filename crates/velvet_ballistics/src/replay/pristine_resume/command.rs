use super::{finalize_pristine_resume, recover_pristine_once};
use crate::app_impl::prelude::*;
use crate::run_compiled_runtime::completion_output::{
    completion_json, completion_text, matching_completion,
};
use vb_runtime::runtime::completion_observation::CompletionObservation;

struct PristineResumeOutcome {
    counters: vb_runtime::counters::CounterSnapshot,
    completion: Option<CompletionObservation>,
    completed: bool,
    success: bool,
    status: &'static str,
    failure: Option<String>,
}

pub(crate) fn run_pristine_resume(
    run_id: &str,
    run: vb_core::RunId,
    journal: vb_storage::FjallJournal,
    output: OutputFormat,
) -> ExitCode {
    let journal = Arc::new(journal);
    let mut runtime = match create_pristine_runtime(&journal, output) {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };
    install_stored_names(&journal, &mut runtime, run);
    let mut outcome = execute_pristine_resume(&mut runtime, run);
    finalize_pristine_resume(runtime, journal, outcome.completed, &mut outcome.failure);
    if report_resume_failure(outcome.failure.as_deref(), output) {
        return CliExitCode::RuntimeFailed.into();
    }
    let report_code = report_pristine_resume(run_id, &outcome, output);
    if report_code != ExitCode::SUCCESS {
        return report_code;
    }
    pristine_resume_exit(outcome.success)
}

fn create_pristine_runtime(
    journal: &Arc<vb_storage::FjallJournal>,
    output: OutputFormat,
) -> Result<vb_runtime::runtime::Runtime, ExitCode> {
    match vb_runtime::runtime::Runtime::new(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
        vb_runtime::journal::StorageRuntimeJournal::shared_strict(Arc::clone(journal)),
    ) {
        Ok(runtime) => Ok(runtime),
        Err(error) => {
            crate::run_compiled_runtime::report_runtime_error(
                format_args!("resume runtime construction failed: {error}"),
                output,
            );
            Err(CliExitCode::RuntimeFailed.into())
        }
    }
}

fn install_stored_names(
    journal: &Arc<vb_storage::FjallJournal>,
    runtime: &mut vb_runtime::runtime::Runtime,
    run: vb_core::RunId,
) {
    if let Ok(events) = journal.events_for_run_full(run)
        && let Some(admission) = vb_runtime::recovery::hydrate_run_admission_from_events(&events)
    {
        crate::run_compiled_runtime::secret_presence::install_stored(
            runtime,
            Arc::clone(journal),
            admission.artifact_digest(),
        );
    }
}

fn execute_pristine_resume(
    runtime: &mut vb_runtime::runtime::Runtime,
    run: vb_core::RunId,
) -> PristineResumeOutcome {
    let (candidate, failure) = match recover_pristine_once(runtime, run) {
        Ok(candidate) => (candidate, None),
        Err(error) => (None, Some(error)),
    };
    let counters = runtime.counters_snapshot();
    let completed = counters.runs_completed == 1 && counters.runs_failed == 0;
    let completion = matching_completion(candidate, completed, run, core::num::NonZeroU16::MIN);
    let success = completed && completion.is_some();
    let status = pristine_status(counters);
    PristineResumeOutcome {
        counters,
        completion,
        completed,
        success,
        status,
        failure,
    }
}

fn pristine_status(counters: vb_runtime::counters::CounterSnapshot) -> &'static str {
    if counters.runs_failed != 0 {
        "failed"
    } else if counters.runs_completed == 1 {
        "completed"
    } else {
        "waiting"
    }
}

fn report_resume_failure(failure: Option<&str>, output: OutputFormat) -> bool {
    if let Some(error) = failure {
        crate::run_compiled_runtime::report_runtime_error(
            format_args!("resume runtime: {error}"),
            output,
        );
        true
    } else {
        false
    }
}

fn report_pristine_resume(
    run_id: &str,
    outcome: &PristineResumeOutcome,
    output: OutputFormat,
) -> ExitCode {
    if output != OutputFormat::Text {
        report_pristine_resume_json(run_id, outcome, output)
    } else {
        report_pristine_resume_text(run_id, outcome);
        ExitCode::SUCCESS
    }
}

fn report_pristine_resume_json(
    run_id: &str,
    outcome: &PristineResumeOutcome,
    output: OutputFormat,
) -> ExitCode {
    let counters = outcome.counters;
    emit_json_or_return!(
        &serde_json::json!({
            "success": outcome.success,
            "run_id": run_id,
            "status": outcome.status,
            "runtime": {
                "submitted": counters.runs_submitted,
                "completed": counters.runs_completed,
                "failed": counters.runs_failed,
                "steps": counters.steps_executed
            },
            "completion": completion_json(outcome.completion)
        }),
        output,
    );
    ExitCode::SUCCESS
}

fn report_pristine_resume_text(run_id: &str, outcome: &PristineResumeOutcome) {
    let counters = outcome.counters;
    outln!(
        "run {run_id}: submitted={} completed={} failed={} steps={}",
        counters.runs_submitted,
        counters.runs_completed,
        counters.runs_failed,
        counters.steps_executed
    );
    if let Some(line) = completion_text(outcome.completion) {
        outln!("{line}");
    }
    report_pristine_final_status(outcome.success, outcome.status);
}

fn report_pristine_final_status(success: bool, status: &str) {
    if success {
        outln!("run completed");
    } else {
        outln!("run {status}; no successful completion observed");
    }
}

fn pristine_resume_exit(success: bool) -> ExitCode {
    if success {
        ExitCode::SUCCESS
    } else {
        CliExitCode::RuntimeFailed.into()
    }
}
