use crate::app_impl::prelude::*;

mod checkpoint;
mod output;

use checkpoint::load_latest_retry_checkpoint;
use output::{
    ensure_requested_step, ensure_retryable, observed_failed_step, reject_empty_retry_events,
    report_checkpoint_error, report_retry_error, report_retry_events_error, report_retry_result,
    report_retry_sequence_overflow,
};

struct RetryPlan {
    run: vb_core::RunId,
    requested_step: u16,
    checkpoint: vb_runtime::run_checkpoint::RunCheckpointV1,
    next_seq: vb_storage::EventSeq,
}

pub(crate) fn cmd_retry(
    run_id: &str,
    requested_step: u16,
    db: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    match prepare_retry_execution(run_id, requested_step, db, output) {
        Ok((plan, journal)) => execute_retry(plan, journal, output),
        Err(code) => code,
    }
}

fn prepare_retry_execution(
    run_id: &str,
    requested_step: u16,
    db: &std::path::Path,
    output: OutputFormat,
) -> Result<(RetryPlan, vb_storage::FjallJournal), ExitCode> {
    let (run, journal) = open_retry_context(run_id, db, output)?;
    let events = load_retry_events(&journal, run, run_id, output)?;
    let plan = prepare_retry_plan(&events, run, requested_step, run_id, output)?;
    Ok((plan, journal))
}

fn open_retry_context(
    run_id: &str,
    db: &std::path::Path,
    output: OutputFormat,
) -> Result<(vb_core::RunId, vb_storage::FjallJournal), ExitCode> {
    let run = parse_run_id(run_id, output)?;
    check_journal_dir(db, output)?;
    let journal = open_retry_journal(db, output)?;
    Ok((run, journal))
}

fn open_retry_journal(
    db: &std::path::Path,
    output: OutputFormat,
) -> Result<vb_storage::FjallJournal, ExitCode> {
    vb_storage::FjallJournal::open(db, None).map_err(|error| {
        report_storage_open_error(&error, db, output);
        CliExitCode::StorageError.into()
    })
}

fn load_retry_events(
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
    run_id: &str,
    output: OutputFormat,
) -> Result<Vec<vb_storage::JournalEvent>, ExitCode> {
    let events = journal.events_for_run(run).map_err(|error| {
        report_retry_events_error(run_id, &error, output);
        ExitCode::from(CliExitCode::StorageError)
    })?;
    reject_empty_retry_events(&events, run_id, output)?;
    Ok(events)
}

fn prepare_retry_plan(
    events: &[vb_storage::JournalEvent],
    run: vb_core::RunId,
    requested_step: u16,
    run_id: &str,
    output: OutputFormat,
) -> Result<RetryPlan, ExitCode> {
    let analysis = commands_journal::analyze_retry(events);
    ensure_retryable(&analysis, run_id, output)?;
    let observed = observed_failed_step(&analysis, run_id, requested_step, output)?;
    ensure_requested_step(&analysis, run_id, observed, requested_step, output)?;
    let checkpoint = load_latest_retry_checkpoint(events, run, requested_step)
        .map_err(|message| report_checkpoint_error(message, observed, requested_step, output))?;
    let next_seq = next_retry_sequence(events).ok_or_else(|| {
        report_retry_sequence_overflow(run_id, output);
        ExitCode::from(CliExitCode::StorageError)
    })?;
    Ok(RetryPlan {
        run,
        requested_step,
        checkpoint,
        next_seq,
    })
}

fn next_retry_sequence(events: &[vb_storage::JournalEvent]) -> Option<vb_storage::EventSeq> {
    match events.last().map(vb_storage::JournalEvent::seq) {
        Some(sequence) => sequence.get().checked_add(1).map(vb_storage::EventSeq::new),
        None => Some(vb_storage::EventSeq::ZERO),
    }
}

fn execute_retry(
    plan: RetryPlan,
    journal: vb_storage::FjallJournal,
    output: OutputFormat,
) -> ExitCode {
    let mut runtime = match build_retry_runtime(journal, plan.checkpoint.artifact_digest, output) {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };
    if let Err(error) = runtime.recover_retry(plan.run, plan.checkpoint, plan.next_seq) {
        report_runtime_error(format_args!("retry recovery failed: {error}"), output);
        return CliExitCode::RuntimeFailed.into();
    }
    if let Err(error) = runtime.tick_all() {
        report_runtime_error(format_args!("runtime tick error: {error}"), output);
        return CliExitCode::RuntimeFailed.into();
    }
    report_retry_result(
        plan.run,
        plan.requested_step,
        runtime.counters_snapshot(),
        output,
    )
}

fn build_retry_runtime(
    journal: vb_storage::FjallJournal,
    artifact_digest: vb_core::WorkflowDigest,
    output: OutputFormat,
) -> Result<vb_runtime::runtime::Runtime, ExitCode> {
    let shard_count = required_retry_shard_count(output)?;
    let (journal, shared) = strict_retry_journal(journal);
    let mut runtime = create_retry_runtime(shard_count, shared, output)?;
    crate::run_compiled_runtime::secret_presence::install_stored(
        &mut runtime,
        journal,
        artifact_digest,
    );
    Ok(runtime)
}

fn required_retry_shard_count(output: OutputFormat) -> Result<NonZeroUsize, ExitCode> {
    NonZeroUsize::new(1).ok_or_else(|| {
        report_retry_error(
            "runtime configuration error: shard count must be non-zero",
            output,
        );
        ExitCode::from(CliExitCode::RuntimeFailed)
    })
}

fn strict_retry_journal(
    journal: vb_storage::FjallJournal,
) -> (
    Arc<vb_storage::FjallJournal>,
    vb_runtime::journal::SharedRuntimeJournal,
) {
    let journal = Arc::new(journal);
    let shared = vb_runtime::journal::StorageRuntimeJournal::shared_strict(Arc::clone(&journal));
    (journal, shared)
}

fn create_retry_runtime(
    shard_count: NonZeroUsize,
    journal: vb_runtime::journal::SharedRuntimeJournal,
    output: OutputFormat,
) -> Result<vb_runtime::runtime::Runtime, ExitCode> {
    vb_runtime::runtime::Runtime::new(
        shard_count,
        vb_runtime::shard::ShardConfig::default(),
        journal,
    )
    .map_err(|error| {
        report_runtime_error(format_args!("runtime construction failed: {error}"), output);
        ExitCode::from(CliExitCode::RuntimeFailed)
    })
}
