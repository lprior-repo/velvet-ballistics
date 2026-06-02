//! Run commands for velvet-ballistics.
#![forbid(unsafe_code)]

use crate::args::{DurabilityMode, OutputFormat};
use crate::exit_code::CliExitCode;
use crate::file_io::{read_file as read_file_with_output, write_failure_message};
use crate::run_compiled_runtime::{
    map_runtime_inputs as runtime_map_inputs, run_compiled_workflow,
};
use crate::workflow::InputMappingError;
use std::path::Path;
use std::process::ExitCode;
use vb_core::{CompiledWorkflow, SlotIdx, SlotValue, WorkflowDigest};

pub(crate) const INPUT_MAPPING_DECODE_FAILED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input-bin decode failed";
pub(crate) const INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count";
pub(crate) const INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot index out of range";

#[non_exhaustive]
pub(crate) enum InputMappingError {
    DecodeFailed,
    SlotCountExceeded,
    SlotIndexOutOfRange,
}

impl std::fmt::Display for InputMappingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::DecodeFailed => INPUT_MAPPING_DECODE_FAILED_MESSAGE,
            Self::SlotCountExceeded => INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
            Self::SlotIndexOutOfRange => INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
        })
    }
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
            write_failure_message(&error.to_string(), output, CliExitCode::CompileFailed);
            return CliExitCode::CompileFailed.into();
        }
    };

    run_compiled_workflow(&compiled, inputs, durability, db, output)
}

fn compile_errors_message(errors: &[vb_compile::CompileError]) -> String {
    errors
        .iter()
        .map(|err| format!("compile error: {err}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn bind_compiled_digest_to_source(
    compiled: CompiledWorkflow,
    source: &[u8],
) -> Result<CompiledWorkflow, vb_core::WorkflowError> {
    let mut parts = compiled.to_parts();
    parts.digest = workflow_source_digest(source);
    CompiledWorkflow::try_from_parts(parts)
}

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
            return ExitCode::FAILURE;
        }
    };

    run_compiled_workflow(&compiled, inputs, durability, db, OutputFormat::Text)
}

pub(crate) fn map_runtime_inputs(
    compiled: &CompiledWorkflow,
    input_data: &[u8],
) -> Result<Box<[(SlotIdx, SlotValue)]>, InputMappingError> {
    if input_data.is_empty() {
        return Ok(Box::from([]));
    }
    let values = postcard::from_bytes::<Box<[SlotValue]>>(input_data)
        .map_err(|_| InputMappingError::DecodeFailed)?;
    if values.len() > usize::from(compiled.slot_count()) {
        return Err(InputMappingError::SlotCountExceeded);
    }
    values
        .iter()
        .copied()
        .enumerate()
        .map(|(index, value)| {
            let slot = u16::try_from(index).map_err(|_| InputMappingError::SlotIndexOutOfRange)?;
            Ok((SlotIdx::new(slot), value))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

pub(crate) fn store_workflow_source_for_run(
    source: &[u8],
    db: Option<&Path>,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    let Some(db) = db else {
        return Ok(());
    };
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => journal,
        Err(error) => {
            report_artifact_store_error(
                format_args!("error opening journal at {}: {error}", db.display()),
                output,
            );
            return Err(CliExitCode::StorageError.into());
        }
    };
    let record = vb_storage::WorkflowSourceRecord {
        digest: workflow_source_digest(source),
        source: source.to_vec(),
    };
    journal.put_workflow_source(&record).map_err(|error| {
        report_artifact_store_error(format_args!("workflow source write error: {error}"), output);
        CliExitCode::StorageError.into()
    })
}

fn report_artifact_store_error(args: std::fmt::Arguments<'_>, output: OutputFormat) {
    if output == OutputFormat::Text {
        crate::errln!("{args}");
    } else {
        write_failure_message(&args.to_string(), output, CliExitCode::StorageError);
    }
}
