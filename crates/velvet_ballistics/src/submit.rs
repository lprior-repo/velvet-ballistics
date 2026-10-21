//! CLI submission prepares every record before its atomic publication boundary.

use crate::app_impl::prelude::*;
use vb_storage::admission::prepare_compiled_artifact;

type TypedInputs = Box<[(vb_core::SlotIdx, vb_core::SlotValue)]>;

struct PreparedSubmission {
    admission: vb_storage::RunSubmission,
    run: vb_core::RunId,
    digest: vb_core::WorkflowDigest,
    step_count: u16,
}

pub(crate) fn cmd_submit(
    workflow: &std::path::Path,
    input_bin: &std::path::Path,
    db: &std::path::Path,
    durability: DurabilityMode,
    output: OutputFormat,
) -> ExitCode {
    let prepared = match prepare_submission(workflow, input_bin, durability, output) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
    if let Err(error) = commit_submission(db, &prepared.admission, durability) {
        write_failure_message(&error.to_string(), output, CliExitCode::StorageError);
        return CliExitCode::StorageError.into();
    }
    emit_submission(&prepared, durability, output)
}

fn prepare_submission(
    workflow: &std::path::Path,
    input_bin: &std::path::Path,
    durability: DurabilityMode,
    output: OutputFormat,
) -> Result<PreparedSubmission, ExitCode> {
    let input = read_file(input_bin, output, CliExitCode::ValidationFailed)?;
    let source = read_file(workflow, output, CliExitCode::ValidationFailed)?;
    let compiled = compile_submission(&source, output)?;
    let inputs = map_runtime_inputs(&compiled, &input).map_err(|error| {
        write_failure_message(&error.to_string(), output, CliExitCode::ValidationFailed);
        ExitCode::from(CliExitCode::ValidationFailed)
    })?;
    prepare_records(source, &compiled, inputs, durability).map_err(|error| {
        write_failure_message(&error.to_string(), output, CliExitCode::ValidationFailed);
        CliExitCode::ValidationFailed.into()
    })
}

fn compile_submission(
    source: &[u8],
    output: OutputFormat,
) -> Result<vb_core::CompiledWorkflow, ExitCode> {
    vb_compile::compile_workflow(source).map_err(|errors| {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false, "error": "compilation failed",
                    "errors": errors.0.iter().map(ToString::to_string).collect::<Vec<_>>()
                }),
                output,
            );
        } else {
            for error in &errors.0 {
                errln!("compile error: {error}");
            }
        }
        CliExitCode::CompileFailed.into()
    })
}

fn prepare_records(
    source: Vec<u8>,
    compiled: &vb_core::CompiledWorkflow,
    inputs: TypedInputs,
    durability: DurabilityMode,
) -> Result<PreparedSubmission, vb_storage::JournalError> {
    let artifact = prepare_compiled_artifact(compiled, submission_policy(durability), &[])?;
    let run = vb_core::RunId::new(generate_submit_run_id());
    let source_record = source_record(source);
    let retained = vb_storage::RunInputsRecord::new(
        run,
        artifact.digest,
        source_record.digest,
        compiled,
        inputs,
    )?;
    let header = submission_header(run, compiled.digest());
    let admission = vb_storage::RunSubmission::new(source_record, artifact, retained, header)?;
    Ok(PreparedSubmission {
        admission,
        run,
        digest: compiled.digest(),
        step_count: compiled.node_count(),
    })
}

fn source_record(source: Vec<u8>) -> vb_storage::WorkflowSourceRecord {
    let digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(&source).into());
    vb_storage::WorkflowSourceRecord { digest, source }
}

fn submission_header(
    run: vb_core::RunId,
    compiled_digest: vb_core::WorkflowDigest,
) -> vb_storage::RunHeaderRecord {
    let accepted_at_ms = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => u64::try_from(duration.as_millis()).map_or(0, |value| value),
        Err(_) => 0,
    };
    vb_storage::RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest,
        status: vb_storage::records::RunHeaderStatus::PENDING.as_byte(),
        accepted_at_ms,
    }
}

fn commit_submission(
    db: &std::path::Path,
    submission: &vb_storage::RunSubmission,
    durability: DurabilityMode,
) -> Result<(), vb_storage::JournalError> {
    let journal = vb_storage::FjallJournal::open(db, None)?;
    journal.commit_run_submission(submission, submission_policy(durability))
}

fn submission_policy(mode: DurabilityMode) -> vb_core::RuntimePolicy {
    match mode {
        DurabilityMode::Strict => vb_core::RuntimePolicy::Strict,
        DurabilityMode::Journaled => vb_core::RuntimePolicy::Journaled,
        DurabilityMode::None => vb_core::RuntimePolicy::Relaxed,
    }
}

fn emit_submission(
    prepared: &PreparedSubmission,
    durability: DurabilityMode,
    output: OutputFormat,
) -> ExitCode {
    let digest_hex = workflow_digest_hex(prepared.digest);
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "run_id": prepared.run.get(), "digest": digest_hex,
                "status": "submitted", "step_count": prepared.step_count,
            }),
            output
        );
    } else {
        emit_submission_text(prepared, durability, &digest_hex);
    }
    CliExitCode::Success.into()
}

fn workflow_digest_hex(digest: vb_core::WorkflowDigest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn emit_submission_text(
    prepared: &PreparedSubmission,
    durability: DurabilityMode,
    digest_hex: &str,
) {
    outln!("submitted run {}", prepared.run.get());
    outln!("  digest:     {digest_hex}");
    outln!("  steps:      {}", prepared.step_count);
    outln!("  durability: {}", durability_as_str(durability));
    outln!("  status:     submitted");
}

pub(crate) fn durability_as_str(mode: DurabilityMode) -> &'static str {
    match mode {
        DurabilityMode::Strict => "strict",
        DurabilityMode::Journaled => "journaled",
        DurabilityMode::None => "none",
    }
}

pub(crate) fn generate_submit_run_id() -> u64 {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return u64::MAX;
    };
    match u64::try_from(now.as_nanos()) {
        Ok(value) => value,
        Err(_) => now.as_secs(),
    }
}
