//! Agent context, status, and action registry commands.
use std::process::ExitCode;
use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget, VerifyProfile, StatusOptions, SystemStatusOptions};
use crate::exit_code::CliExitCode;
use crate::constants::VERSION;
use crate::action_specs::{registered_cli_actions, write_action_registry, write_action_registry_uninitialized, write_action_inspect};
use crate::output::{json_error, json_out, output_error_exit, write_failure_message, write_stdout_line, write_json_pretty_stdout};
use crate::output_utils::*;
use crate::file_io::read_file;
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::cli_envelope;
use crate::deliver_sink;
use vb_runtime::action::ActionRegistry;

pub(crate) fn cmd_agent_context(deliver: Option<&str>) -> ExitCode {
    let context = crate::cli_envelope::serialize_with_version(
        &crate::agent_context::build(VERSION),
        crate::cli_envelope::Kind::AgentContext,
    );
    if let Some(raw_target) = deliver {
        return deliver_json_value(raw_target, &context);
    }
    match write_json_pretty_stdout(&context) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => output_error_exit(&error),
    }
}

pub(crate) fn deliver_json_value(raw_target: &str, value: &serde_json::Value) -> ExitCode {
    let target = match deliver_sink::parse_deliver_target(raw_target) {
        Ok(target) => target,
        Err(error) => {
            crate::errln!("deliver failed: {error}");
            return CliExitCode::ValidationFailed.into();
        }
    };
    match deliver_sink::write_json_line(&target, value) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            crate::errln!("deliver failed: {error}");
            deliver_error_exit_code(error).into()
        }
    }
}

pub(crate) fn deliver_error_exit_code(error: deliver_sink::DeliverSinkError) -> CliExitCode {
    match error {
        deliver_sink::DeliverSinkError::Io(_) => CliExitCode::StorageError,
        _ => CliExitCode::ValidationFailed,
    }
}

pub(crate) fn cmd_status(options: StatusOptions, output: OutputFormat) -> ExitCode {
    let requested_output = if options.emit_yaml {
        OutputFormat::Yaml
    } else {
        output
    };
    let status = crate::commands_status::build_status(options);
    match crate::commands_status::print_status(&status, requested_output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => output_error_exit(&error),
    }
}

pub(crate) fn cmd_system_status(options: SystemStatusOptions, output: OutputFormat) -> ExitCode {
    let requested_output = if options.emit_yaml {
        OutputFormat::Yaml
    } else {
        output
    };
    match crate::commands_system_status::print_system_status(options, requested_output, VERSION) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => output_error_exit(&error),
    }
}

pub(crate) fn cmd_action_list(output: OutputFormat, registry_mode: ActionRegistryMode) -> ExitCode {
    match registry_mode {
        ActionRegistryMode::Registered => match registered_cli_actions() {
            Ok(registry) => write_action_registry(&registry, output),
            Err(error) => write_action_registry_error(&error, output),
        },
        ActionRegistryMode::Empty => {
            let registry = ActionRegistry::new();
            write_action_registry(&registry, output)
        }
        ActionRegistryMode::Uninitialized => {
            write_action_registry_uninitialized(output);
            CliExitCode::ValidationFailed.into()
        }
    }
}

pub(crate) fn cmd_action_inspect(
    action_name: String,
    output: OutputFormat,
    registry_mode: ActionRegistryMode,
) -> ExitCode {
    match registry_mode {
        ActionRegistryMode::Registered => match registered_cli_actions() {
            Ok(registry) => write_action_inspect(&registry, action_name, output),
            Err(error) => write_action_registry_error(&error, output),
        },
        ActionRegistryMode::Empty => {
            let registry = ActionRegistry::new();
            write_action_inspect(&registry, action_name, output)
        }
        ActionRegistryMode::Uninitialized => {
            write_action_registry_uninitialized(output);
            CliExitCode::ValidationFailed.into()
        }
    }
}

pub(crate) fn write_action_registry_error(
    error: &vb_core::action::ActionError,
    output: OutputFormat,
) -> ExitCode {
    let message = format!("failed to register CLI action contracts: {error}");
    if output == OutputFormat::Text {
        crate::errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
            }),
            output,
        );
    }
    CliExitCode::ValidationFailed.into()
}
