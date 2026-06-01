//! Run commands for velvet-ballistics.
#![forbid(unsafe_code)]

use crate::args::DurabilityMode;
use crate::workflow::InputMappingError;
pub(crate) use crate::workflow::run_compiled_workflow;
use std::path::Path;
use std::process::ExitCode;
use vb_core::{CompiledWorkflow, SlotIdx, SlotValue, WorkflowParts};

pub(crate) const INPUT_MAPPING_DECODE_FAILED_MESSAGE: &str = "INPUT_MAPPING_FAILED: input-bin decode failed";
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
) -> ExitCode {
    let input_data = match read_file(input_bin) {
        Ok(b) => b,
        Err(code) => return code,
    };

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

    let inputs = match map_runtime_inputs(&compiled, &input_data) {
        Ok(inputs) => inputs,
        Err(error) => {
            crate::errln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    run_compiled_workflow(&compiled, inputs, durability, db)
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
            return ExitCode::FAILURE;
        }
    };

    run_compiled_workflow(&compiled, inputs, durability, db)
}

pub(crate) fn map_runtime_inputs(
    compiled: &CompiledWorkflow,
    input_data: &[u8],
) -> Result<Box<[(SlotIdx, SlotValue)]>, InputMappingError> {
    if input_data.is_empty() {
        return Ok(Box::from([]));
    }
    let values =
        postcard::from_bytes::<Box<[SlotValue]>>(input_data).map_err(|_| InputMappingError::DecodeFailed)?;
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

pub(crate) fn read_file(path: &std::path::Path) -> Result<Vec<u8>, ExitCode> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            crate::errln!("error reading {}: {e}", path.display());
            Err(ExitCode::FAILURE)
        }
    }
}
