//! Module: compile

use crate::app_impl::prelude::*;

pub(crate) fn cmd_compile(
    workflow: &std::path::Path,
    emit: EmitTarget,
    out: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::CompileFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            if output != OutputFormat::Text {
                write_failure_message(
                    &compile_errors_message(&errors.0),
                    output,
                    CliExitCode::CompileFailed,
                );
            } else {
                for err in &errors.0 {
                    errln!("compile error: {err}");
                }
            }
            return CliExitCode::CompileFailed.into();
        }
    };
    if let Err(code) = reject_unknown_actions(&compiled, output, CliExitCode::CompileFailed) {
        return code;
    }

    match emit {
        EmitTarget::Ir => {
            let encoded = match vb_compile::encode_compiled_ir(&compiled) {
                Ok(data) => data,
                Err(e) => {
                    if output != OutputFormat::Text {
                        write_failure_message(
                            &format!("IR serialization error: {e}"),
                            output,
                            CliExitCode::CompileFailed,
                        );
                    } else {
                        errln!("IR serialization error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, &encoded) {
                if output != OutputFormat::Text {
                    write_failure_message(
                        &format!("error writing {}: {e}", out.display()),
                        output,
                        CliExitCode::CompileFailed,
                    );
                } else {
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                emit_json_or_return!(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "ir"
                    }),
                    output,
                );
            } else {
                outln!("compiled IR written to {}", out.display());
            }
        }
        EmitTarget::Yaml => {
            let yaml_str = match encode_yaml(&compiled) {
                Ok(s) => s,
                Err(e) => {
                    if output != OutputFormat::Text {
                        write_failure_message(
                            &format!("YAML serialization error: {e}"),
                            output,
                            CliExitCode::CompileFailed,
                        );
                    } else {
                        errln!("YAML serialization error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, yaml_str.as_bytes()) {
                if output != OutputFormat::Text {
                    write_failure_message(
                        &format!("error writing {}: {e}", out.display()),
                        output,
                        CliExitCode::CompileFailed,
                    );
                } else {
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                emit_json_or_return!(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "yaml"
                    }),
                    output,
                );
            } else {
                outln!("compiled YAML written to {}", out.display());
            }
        }
        EmitTarget::Postcard => {
            let encoded = match vb_compile::encode_compiled_ir(&compiled) {
                Ok(data) => data,
                Err(e) => {
                    if output != OutputFormat::Text {
                        write_failure_message(
                            &format!("postcard serialization error: {e}"),
                            output,
                            CliExitCode::CompileFailed,
                        );
                    } else {
                        errln!("postcard serialization error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, &encoded) {
                if output != OutputFormat::Text {
                    write_failure_message(
                        &format!("error writing {}: {e}", out.display()),
                        output,
                        CliExitCode::CompileFailed,
                    );
                } else {
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                emit_json_or_return!(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "postcard"
                    }),
                    output,
                );
            } else {
                outln!("compiled postcard written to {}", out.display());
            }
        }
    }

    ExitCode::SUCCESS
}

fn encode_yaml(workflow: &vb_core::CompiledWorkflow) -> Result<String, serde_saphyr::ser::Error> {
    match workflow.to_versioned_parts() {
        vb_core::workflow::VersionedWorkflowParts::V1(parts) => serde_saphyr::to_string(&parts),
        vb_core::workflow::VersionedWorkflowParts::V2(ir) => serde_saphyr::to_string(&ir),
    }
}

#[cfg(test)]
#[path = "versioned_test_support.rs"]
pub(crate) mod versioned_test_support;

#[cfg(test)]
#[path = "compile_versioned_tests.rs"]
mod versioned_tests;
