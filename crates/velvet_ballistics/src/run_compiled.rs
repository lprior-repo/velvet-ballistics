//! Module: run_compiled

use crate::app_impl::prelude::*;

pub(crate) fn cmd_run_compiled(
    vbir_path: &std::path::Path,
    input_bin: &std::path::Path,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> ExitCode {
    let input_data = match read_file(input_bin, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let ir_bytes = match read_file(vbir_path, output, CliExitCode::CompileFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled: vb_core::CompiledWorkflow =
        match postcard::from_bytes::<vb_core::WorkflowParts>(&ir_bytes) {
            Ok(parts) => match vb_core::CompiledWorkflow::try_from_parts(parts) {
                Ok(c) => c,
                Err(e) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "error": format!("compiled IR validation error: {e}")
                            }),
                            output,
                        );
                    } else {
                        errln!("compiled IR validation error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            },
            Err(e) => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("error deserializing compiled IR: {e}")
                        }),
                        output,
                    );
                } else {
                    errln!("error deserializing compiled IR: {e}");
                }
                return CliExitCode::CompileFailed.into();
            }
        };

    let inputs = match map_runtime_inputs(&compiled, &input_data) {
        Ok(inputs) => inputs,
        Err(error) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": error.to_string()
                    }),
                    output,
                );
            } else {
                errln!("{error}");
            }
            return CliExitCode::CompileFailed.into();
        }
    };

    run_compiled_workflow(&compiled, inputs, durability, db, output)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputMappingError {
    DecodeFailed,
    SlotCountExceeded,
    SlotIndexOutOfRange,
    KindMismatch {
        slot: vb_core::SlotIdx,
        expected_kind: vb_core::InputSlotKind,
    },
}

impl std::fmt::Display for InputMappingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::DecodeFailed => INPUT_MAPPING_DECODE_FAILED_MESSAGE,
            Self::SlotCountExceeded => INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
            Self::SlotIndexOutOfRange => INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
            Self::KindMismatch { slot, expected_kind } => {
                write!(
                    formatter,
                    "INPUT_MAPPING_FAILED: slot {slot} value kind mismatch, expected {expected_kind}"
                )
            }
        })
    }
}

pub(crate) fn map_runtime_inputs(
    compiled: &vb_core::CompiledWorkflow,
    input_data: &[u8],
) -> Result<Box<[(vb_core::SlotIdx, vb_core::SlotValue)]>, InputMappingError> {
    let input_slots = compiled.input_slots();
    if input_slots.is_empty() {
        return Ok(Box::from([]));
    }
    if input_data.is_empty() {
        return Ok(Box::from([]));
    }
    let values = postcard::from_bytes::<Box<[vb_core::SlotValue]>>(input_data)
        .map_err(|_| InputMappingError::DecodeFailed)?;
    if values.len() > input_slots.len() {
        return Err(InputMappingError::SlotCountExceeded);
    }
    input_slots
        .iter()
        .zip(values.iter())
        .map(|(slot, value)| {
            let value_kind = match value {
                vb_core::SlotValue::Null => vb_core::InputSlotKind::Null,
                vb_core::SlotValue::Bool(_) => vb_core::InputSlotKind::Bool,
                vb_core::SlotValue::I64(_) => vb_core::InputSlotKind::I64,
                vb_core::SlotValue::F64(_) => vb_core::InputSlotKind::F64,
                vb_core::SlotValue::Symbol(_) => vb_core::InputSlotKind::Symbol,
                vb_core::SlotValue::List(_) => vb_core::InputSlotKind::List,
                vb_core::SlotValue::Object(_) => vb_core::InputSlotKind::Object,
                vb_core::SlotValue::Blob(_) => vb_core::InputSlotKind::Blob,
            };
            if value_kind != slot.kind {
                return Err(InputMappingError::KindMismatch {
                    slot: slot.slot,
                    expected_kind: slot.kind,
                });
            }
            Ok((slot.slot, *value))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

pub(crate) fn runtime_journal_for_mode(
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> Result<vb_runtime::journal::SharedRuntimeJournal, ExitCode> {
    match durability {
        DurabilityMode::None => Ok(vb_runtime::journal::NoopRuntimeJournal::shared()),
        DurabilityMode::Journaled => open_storage_runtime_journal(db, false, output),
        DurabilityMode::Strict => open_storage_runtime_journal(db, true, output),
    }
}

pub(crate) fn runtime_config_for_durability(
    durability: DurabilityMode,
) -> vb_runtime::shard::ShardConfig {
    let mut config = vb_runtime::shard::ShardConfig::default();
    if durability == DurabilityMode::None {
        config.policy = vb_core::policy::RuntimePolicy::Relaxed;
    }
    config
}
