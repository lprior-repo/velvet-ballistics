//! Run commands for velvet-ballistics.
#![forbid(unsafe_code)]

use crate::args::{DurabilityMode, OutputFormat};
use crate::exit_code::CliExitCode;
use crate::file_io::{
    read_file as read_file_with_output, report_storage_open_error, write_failure_message,
};
use crate::run_compiled_runtime::admitted_workflow_for_durability;
use crate::run_compiled_runtime::{
    map_runtime_inputs as runtime_map_inputs, run_compiled_workflow,
};
use crate::run_id::generate_run_id_from_clock;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use vb_core::{
    CompiledWorkflow, RunId, SlotIdx, SlotValue, WorkflowDigest, WorkflowId, WorkflowParts,
};
use vb_runtime::{InputMappingFailureKind, RuntimeError};

pub(crate) const INPUT_MAPPING_DECODE_FAILED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input-bin decode failed";
pub(crate) const INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count";
pub(crate) const INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot index out of range";

pub(crate) fn cmd_validate(workflow: &Path) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            crate::errln!("file is not valid UTF-8: {e}");
            return ExitCode::FAILURE;
        }
    };

    match vb_yaml::parse_workflow_source(text) {
        Ok(_ast) => {}
        Err(e) => {
            crate::errln!("YAML parse error: {e}");
            return ExitCode::FAILURE;
        }
    }

    match vb_compile::compile_workflow(&bytes) {
        Ok(_compiled) => {}
        Err(errors) => {
            for err in &errors.0 {
                crate::errln!("compile error: {err}");
            }
            return ExitCode::FAILURE;
        }
    }

    crate::outln!("valid");
    ExitCode::SUCCESS
}

pub(crate) fn cmd_compile(workflow: &Path, emit: crate::args::EmitTarget, out: &Path) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            for err in &errors.0 {
                crate::errln!("compile error: {err}");
            }
            return ExitCode::FAILURE;
        }
    };

    match emit {
        crate::args::EmitTarget::Ir => {
            let parts = compiled.to_parts();
            let encoded = match postcard::to_allocvec(&parts) {
                Ok(data) => data,
                Err(e) => {
                    crate::errln!("IR serialization error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = std::fs::write(out, &encoded) {
                crate::errln!("error writing {}: {e}", out.display());
                return ExitCode::FAILURE;
            }
            crate::outln!("compiled IR written to {}", out.display());
        }
        crate::args::EmitTarget::Yaml | crate::args::EmitTarget::Postcard => {
            crate::errln!("legacy compile runner supports only --emit ir");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

pub(crate) fn cmd_run(
    workflow: &Path,
    input_bin: &Path,
    durability: DurabilityMode,
    db: Option<&Path>,
    output: OutputFormat,
) -> ExitCode {
    let input_data = match read_file_with_output(input_bin, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let bytes = match read_file_with_output(workflow, output, CliExitCode::CompileFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            if output == OutputFormat::Text {
                errors
                    .0
                    .iter()
                    .for_each(|err| crate::errln!("compile error: {err}"));
            } else {
                let message = crate::output_utils::compile_errors_message(&errors.0);
                write_failure_message(&message, output, CliExitCode::CompileFailed);
            }
            return CliExitCode::CompileFailed.into();
        }
    };

    let inputs = match runtime_map_inputs(&compiled, &input_data) {
        Ok(inputs) => inputs,
        Err(error) => {
            write_failure_message(&error.to_string(), output, CliExitCode::InputMappingFailed);
            return CliExitCode::InputMappingFailed.into();
        }
    };

    let run_id = generate_run_id_from_clock();
    let admitted_workflow = match admitted_workflow_for_durability(&compiled, durability, output) {
        Ok(workflow) => workflow,
        Err(code) => return code,
    };

    if durability != DurabilityMode::None
        && let Some(db_path) = db
        && let Err(code) =
            persist_durable_run_records(run_id, &bytes, &admitted_workflow, db_path, output)
    {
        return code;
    }

    run_compiled_workflow(run_id, admitted_workflow, inputs, durability, db, output)
}

pub(crate) fn cmd_run_compiled(
    vbir_path: &Path,
    input_bin: &Path,
    durability: DurabilityMode,
    db: Option<&Path>,
) -> ExitCode {
    let input_data = match read_file(input_bin) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let ir_bytes = match read_file(vbir_path) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled: CompiledWorkflow = match postcard::from_bytes::<WorkflowParts>(&ir_bytes) {
        Ok(parts) => match CompiledWorkflow::try_from_parts(parts) {
            Ok(c) => c,
            Err(e) => {
                crate::errln!("compiled IR validation error: {e}");
                return ExitCode::FAILURE;
            }
        },
        Err(e) => {
            crate::errln!("error deserializing compiled IR: {e}");
            return ExitCode::FAILURE;
        }
    };

    let inputs = match map_runtime_inputs(&compiled, &input_data) {
        Ok(inputs) => inputs,
        Err(error) => {
            crate::errln!("{error}");
            return CliExitCode::InputMappingFailed.into();
        }
    };

    let run_id = generate_run_id_from_clock();
    let admitted_workflow =
        match admitted_workflow_for_durability(&compiled, durability, OutputFormat::Text) {
            Ok(workflow) => workflow,
            Err(code) => return code,
        };

    run_compiled_workflow(
        run_id,
        admitted_workflow,
        inputs,
        durability,
        db,
        OutputFormat::Text,
    )
}

pub(crate) fn map_runtime_inputs(
    compiled: &CompiledWorkflow,
    input_data: &[u8],
) -> Result<Box<[(SlotIdx, SlotValue)]>, RuntimeError> {
    runtime_map_inputs(compiled, input_data)
}

pub(crate) fn read_file(path: &std::path::Path) -> Result<Vec<u8>, ExitCode> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            crate::errln!("error reading {}: {e}", path.display());
            Err(ExitCode::FAILURE)
        }
    }
}

fn persist_durable_run_records(
    run_id: RunId,
    bytes: &[u8],
    admitted_workflow: &CompiledWorkflow,
    db: &Path,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => journal,
        Err(error) => {
            report_storage_open_error(&error, db, output);
            return Err(CliExitCode::StorageError.into());
        }
    };

    let source_digest = WorkflowDigest::from_bytes(blake3::hash(bytes).into());
    let source_record = vb_storage::WorkflowSourceRecord {
        digest: source_digest,
        source: bytes.to_vec(),
    };
    if let Err(error) = vb_storage::put_workflow_source(&journal, &source_record) {
        write_failure_message(
            &format!("workflow source write error: {error}"),
            output,
            CliExitCode::StorageError,
        );
        return Err(CliExitCode::StorageError.into());
    }

    let header = vb_storage::RunHeaderRecord {
        run: run_id,
        workflow_id: WorkflowId::new(0),
        compiled_digest: admitted_workflow.digest(),
        status: 0,
        accepted_at_ms: sample_accepted_at_ms(),
    };
    if let Err(error) = vb_storage::put_run_header(&journal, &header) {
        write_failure_message(
            &format!("run header write error: {error}"),
            output,
            CliExitCode::StorageError,
        );
        return Err(CliExitCode::StorageError.into());
    }

    Ok(())
}

fn sample_accepted_at_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .map_or(0, |ms| ms)
}
