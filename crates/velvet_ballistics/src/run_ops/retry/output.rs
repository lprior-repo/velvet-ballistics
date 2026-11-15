use crate::app_impl::prelude::*;

pub(super) fn report_retry_events_error(
    run_id: &str,
    error: &impl std::fmt::Display,
    output: OutputFormat,
) {
    let message = format!("error reading run {run_id}: {error}");
    report_runtime_error(format_args!("{message}"), output);
}

pub(super) fn reject_empty_retry_events(
    events: &[vb_storage::JournalEvent],
    run_id: &str,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    if events.is_empty() {
        if output == OutputFormat::Text {
            errln!("run {run_id}: no events found");
        } else {
            report_retry_error(&format!("run {run_id} not found"), output);
        }
        return Err(CliExitCode::StorageError.into());
    }
    Ok(())
}

pub(super) fn report_retry_sequence_overflow(run_id: &str, output: OutputFormat) {
    if output == OutputFormat::Text {
        errln!("run {run_id}: journal sequence overflow");
    } else {
        report_retry_error(&format!("run {run_id} journal sequence overflow"), output);
    }
}

pub(super) fn ensure_retryable(
    analysis: &commands_journal::RetryAnalysis,
    run_id: &str,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    if analysis.can_retry {
        return Ok(());
    }
    report_retry_error(&format!("run {run_id} {}", analysis.reason), output);
    Err(CliExitCode::LifecycleError.into())
}

pub(super) fn observed_failed_step(
    analysis: &commands_journal::RetryAnalysis,
    run_id: &str,
    requested_step: u16,
    output: OutputFormat,
) -> Result<u16, ExitCode> {
    match analysis.failed_at_step {
        Some(step) => Ok(step),
        None => {
            report_missing_failed_step(run_id, requested_step, output);
            Err(CliExitCode::LifecycleError.into())
        }
    }
}

fn report_missing_failed_step(run_id: &str, requested_step: u16, output: OutputFormat) {
    let message = format!("run {run_id} failed but no observed failed step");
    let payload = serde_json::json!({
        "success": false,
        "error": message,
        "observed_failed_step": Option::<u16>::None,
        "requested_step": requested_step
    });
    if output != OutputFormat::Text {
        json_error(&payload, output);
    } else {
        errln!("run {run_id} failed but no observed failed step");
    }
}

pub(super) fn ensure_requested_step(
    analysis: &commands_journal::RetryAnalysis,
    run_id: &str,
    observed: u16,
    requested: u16,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    if observed == requested {
        return Ok(());
    }
    report_stale_step(analysis, run_id, observed, requested, output);
    Err(CliExitCode::LifecycleError.into())
}

fn report_stale_step(
    analysis: &commands_journal::RetryAnalysis,
    run_id: &str,
    observed: u16,
    requested: u16,
    output: OutputFormat,
) {
    let message = format!(
        "run {run_id} failed at observed step {observed}; requested step {requested} is stale"
    );
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
                "observed_failed_step": observed,
                "last_successful_step": analysis.last_successful_step,
                "requested_step": requested
            }),
            output,
        );
    } else {
        errln!("{message}");
    }
}

pub(super) fn report_checkpoint_error(
    message: String,
    observed: u16,
    requested: u16,
    output: OutputFormat,
) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
                "observed_failed_step": observed,
                "requested_step": requested
            }),
            output,
        );
    } else {
        errln!("{message}");
    }
    CliExitCode::LifecycleError.into()
}

pub(super) fn report_retry_error(message: &str, output: OutputFormat) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({ "success": false, "error": message }),
            output,
        );
    } else {
        errln!("{message}");
    }
}

pub(super) fn report_retry_result(
    run: vb_core::RunId,
    requested_step: u16,
    counters: vb_runtime::counters::CounterSnapshot,
    output: OutputFormat,
) -> ExitCode {
    if output != OutputFormat::Text {
        report_retry_json(run, requested_step, counters, output);
    } else {
        report_retry_text(run, requested_step, counters);
    }
    if counters.runs_failed != 0 {
        CliExitCode::RuntimeFailed.into()
    } else {
        ExitCode::SUCCESS
    }
}

fn report_retry_json(
    run: vb_core::RunId,
    requested_step: u16,
    counters: vb_runtime::counters::CounterSnapshot,
    output: OutputFormat,
) {
    let status = retry_status(counters);
    json_error(
        &serde_json::json!({
            "success": counters.runs_failed == 0,
            "run_id": run.get(),
            "requested_step": requested_step,
            "status": status,
            "runtime": {
                "completed": counters.runs_completed,
                "failed": counters.runs_failed
            }
        }),
        output,
    );
}

fn report_retry_text(
    run: vb_core::RunId,
    requested_step: u16,
    counters: vb_runtime::counters::CounterSnapshot,
) {
    outln!(
        "run {}: retried step {} completed={} failed={}",
        run.get(),
        requested_step,
        counters.runs_completed,
        counters.runs_failed
    );
    retry_text_status(counters);
}

fn retry_text_status(counters: vb_runtime::counters::CounterSnapshot) {
    if counters.runs_failed != 0 {
        errln!("run failed after retry");
    } else if counters.runs_completed != 0 {
        outln!("run completed");
    } else {
        outln!("run accepted but not terminal after one runtime tick");
    }
}

fn retry_status(counters: vb_runtime::counters::CounterSnapshot) -> &'static str {
    if counters.runs_failed != 0 {
        "failed"
    } else if counters.runs_completed != 0 {
        "completed"
    } else {
        "accepted"
    }
}
