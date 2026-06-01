#![forbid(unsafe_code)]
//! Run compiled workflow command.

fn cmd_run_compiled(
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
enum InputMappingError {
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

