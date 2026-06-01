//! CLI dispatcher and shared implementation prelude.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::process::ExitCode;

use crate::args::{Command, OutputFormat};
use crate::exit_code::CliExitCode;

pub(crate) mod prelude {
    pub(crate) use std::num::NonZeroUsize;
    pub(crate) use std::process::ExitCode;
    pub(crate) use std::sync::Arc;
    pub(crate) use std::time::{Instant, SystemTime, UNIX_EPOCH};

    pub(crate) use crate::action::{
        write_action_inspect, write_action_registry, write_action_registry_uninitialized,
    };
    pub(crate) use crate::action_specs::{
        action_contract_detail, action_table_rows, registered_cli_actions,
    };
    pub(crate) use crate::app_impl::{
        INPUT_MAPPING_DECODE_FAILED_MESSAGE, INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
        INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE, compile_errors_message,
    };
    pub(crate) use crate::args::{
        self, ActionRegistryMode, DurabilityMode, EmitTarget, EventStatus, OutputFormat, StepTarget,
    };
    pub(crate) use crate::constants::VERSION;
    pub(crate) use crate::exit_code::CliExitCode;
    pub(crate) use crate::file_io::{
        parse_run_id, read_file, read_journal_events, report_storage_open_error,
    };
    pub(crate) use crate::io_helpers::{
        exit_from_io, unique_doctor_run_id, write_help_stdout, write_parse_error_stderr,
        write_version_stdout,
    };
    pub(crate) use crate::output::{
        json_error, json_out_exit, output_error_exit, write_contract_error_json,
        write_failure_message, write_json_pretty_stdout,
    };
    pub(crate) use crate::run::report_compiled_ir_store_error;
    pub(crate) use crate::run_cancel::run_is_terminal;
    pub(crate) use crate::run_compiled::{
        map_runtime_inputs, runtime_config_for_durability, runtime_journal_for_mode,
    };
    pub(crate) use crate::run_compiled_runtime::{
        open_storage_runtime_journal, run_compiled_workflow,
    };
    pub(crate) use crate::step_helpers::{
        compile_bytes_json, decode_step_inputs, execute_step_isolated,
    };
    pub(crate) use crate::step_helpers_display::{error_name, print_step_result};
    pub(crate) use crate::{
        agent_context, cli_envelope, commands_diff, commands_incident, commands_journal,
        commands_status, commands_system_status, commands_workflow, deliver_sink,
    };
    pub(crate) use vb_ipc::client::IpcClient;
    pub(crate) use vb_ipc::{IpcCommand, IpcPayload};
    pub(crate) use vb_runtime::action::ActionRegistry;
}

pub(crate) const INPUT_MAPPING_DECODE_FAILED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input-bin decode failed";
pub(crate) const INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count";
pub(crate) const INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot index out of range";

pub(crate) mod explain_repair {}

pub(crate) fn run_from_env() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let requested_output = output_format_from_args(&args);
    match crate::args::parse_args(&args) {
        Ok(Command::Help) => {
            prelude::exit_from_io(&prelude::write_help_stdout(), ExitCode::SUCCESS)
        }
        Ok(Command::Version) => {
            prelude::exit_from_io(&prelude::write_version_stdout(), ExitCode::SUCCESS)
        }
        Ok(Command::AgentContext { deliver }) => {
            crate::agent_io::cmd_agent_context(deliver.as_deref())
        }
        Ok(Command::AiContext { run_id, db, output }) => {
            crate::commands_ai_context::handle(&run_id, &db, output)
        }
        Ok(Command::Status { options, output }) => crate::agent_io::cmd_status(options, output),
        Ok(Command::SystemStatus { options, output }) => {
            crate::agent_io::cmd_system_status(options, output)
        }
        Ok(Command::ActionList { output, registry }) => {
            crate::agent_io::cmd_action_list(output, registry)
        }
        Ok(Command::ActionInspect {
            action_name,
            output,
            registry,
        }) => crate::agent_io::cmd_action_inspect(action_name, output, registry),
        Ok(Command::Verify {
            workflow,
            profile,
            output,
        }) => crate::verify::cmd_verify(&workflow, profile, output),
        Ok(Command::Validate { workflow, output }) => {
            crate::validate::cmd_validate(&workflow, output)
        }
        Ok(Command::Explain { workflow, output }) => crate::explain::cmd_explain(&workflow, output),
        Ok(Command::Compile {
            workflow,
            emit,
            out,
            output,
        }) => crate::compile::cmd_compile(&workflow, emit, &out, output),
        Ok(Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            step,
            output,
        }) => match step {
            Some(target) => crate::run_step::cmd_run_step(&workflow, durability, &target, output),
            None => crate::run::cmd_run(&workflow, &input_bin, durability, db.as_deref(), output),
        },
        Ok(Command::RunCompiled {
            workflow,
            input_bin,
            durability,
            db,
            output,
        }) => crate::run_compiled::cmd_run_compiled(
            &workflow,
            &input_bin,
            durability,
            db.as_deref(),
            output,
        ),
        Ok(Command::IpcServe { socket, db }) => crate::ipc_serve::cmd_ipc_serve(&socket, &db),
        Ok(Command::Inspect { run_id, db, output }) => {
            crate::inspect::cmd_inspect(&run_id, &db, output)
        }
        Ok(Command::Events {
            run_id,
            db,
            output,
            status,
            limit,
        }) => crate::events::cmd_events(&run_id, &db, output, status, limit),
        Ok(Command::Replay { run_id, db, output }) => {
            crate::replay::cmd_replay(&run_id, &db, output)
        }
        Ok(Command::Trace {
            run_id,
            db,
            output,
            filters,
        }) => crate::trace::cmd_trace(&run_id, &db, output, filters),
        Ok(Command::Retry { run_id, db, output }) => {
            crate::run_ops::cmd_retry(&run_id, &db, output)
        }
        Ok(Command::Resume { run_id, db, output }) => {
            crate::replay::cmd_resume(&run_id, &db, output)
        }
        Ok(Command::BenchRun { workflow, output }) => {
            crate::bench_run::cmd_bench_run(&workflow, output)
        }
        Ok(Command::Doctor { db, output }) => crate::doctor::cmd_doctor(db.as_deref(), output),
        Ok(Command::Answer {
            run_id,
            step,
            value_file,
            db,
            output,
        }) => crate::run_cancel::cmd_answer(&run_id, step, &value_file, &db, output),
        Ok(Command::Graph { workflow, output }) => crate::graph::cmd_graph(&workflow, output),
        Ok(Command::Diff {
            run_a,
            run_b,
            db,
            output,
        }) => crate::incident_ops::cmd_diff(&run_a, &run_b, &db, output),
        Ok(Command::Incident { run_id, db, output }) => {
            crate::incident_diff::cmd_incident(&run_id, &db, output)
        }
        Ok(Command::Submit {
            workflow,
            input_bin,
            db,
            durability,
            output,
        }) => crate::submit::cmd_submit(&workflow, &input_bin, &db, durability, output),
        Ok(Command::Simulate { workflow, output }) => {
            crate::simulate::cmd_simulate(&workflow, output)
        }
        Ok(Command::Cancel {
            run_id,
            db,
            reason,
            output,
        }) => crate::run_cancel_ops::cmd_cancel(&run_id, &db, reason, output),
        Err(error) => prelude::exit_from_io(
            &prelude::write_parse_error_stderr(&error, requested_output),
            CliExitCode::ValidationFailed.into(),
        ),
    }
}

fn output_format_from_args(args: &[OsString]) -> OutputFormat {
    args.iter()
        .position(|arg| arg == "--emit")
        .and_then(|index| args.get(index.checked_add(1)?))
        .and_then(|value| value.to_str())
        .map_or(OutputFormat::Text, parse_emit_output_format)
}

fn parse_emit_output_format(raw: &str) -> OutputFormat {
    match raw {
        "yaml" => OutputFormat::Yaml,
        "postcard" => OutputFormat::Postcard,
        _ => OutputFormat::Text,
    }
}

pub(crate) fn compile_errors_message(errors: &[vb_compile::CompileError]) -> String {
    errors
        .iter()
        .fold(String::from("compilation failed"), |mut message, error| {
            message.push_str("; compile error: ");
            message.push_str(&error.to_string());
            message
        })
}
