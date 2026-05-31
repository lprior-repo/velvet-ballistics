//! Velvet Ballastics CLI application implementation.
//!
//! Holzman Rust: thin imperative shell over functional core.
//! Each submodule is bounded to 300 lines per architectural drift policy.
//!
#![forbid(unsafe_code)]

#![allow(clippy::too_many_arguments, clippy::too_many_lines)]
#![allow(clippy::match_single_binding, clippy::match_wildcard_for_single_variants)]

mod constants;
mod file_io;
mod agent_io;
mod action;
mod action_specs;
mod verify;
mod validate;
mod compile;
mod run;
mod submit;
mod run_step;
mod step_helpers;
mod run_compiled;
mod run_compiled_runtime;
mod ipc_serve;
mod inspect;
mod events;
mod replay;
mod trace;
mod run_ops;
mod run_ops2;
mod incident_diff;
mod explain;
mod explain_reports;
mod explain_errors;
mod explain_repair;
mod explain_validation;
mod explain_validation2;
mod explain_validation3;
mod explain_validation4;
mod graph;
mod simulate;
mod bench_run;
mod doctor;
mod doctor_helpers;
mod io_helpers;
mod output_utils;
mod output;
mod dispatcher;

use std::ffi::OsString;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::args;
use crate::args::parse_args;
use crate::args::{
    ActionRegistryMode, Command, DurabilityMode, EmitTarget, EventStatus, OutputFormat, ParseError,
    StepTarget, VALID_COMMANDS, VerifyProfile,
};

#[cfg(test)]
pub(crate) use crate::commands_ai_context::{
    RunStatus, redacted_slot_value, suggested_ai_commands,
};

use crate::exit_code::CliExitCode;
use crate::constants::{HELP, VERSION};
use crate::dispatcher::run_from_env;
use crate::file_io::read_file;
use crate::file_io::parse_run_id;
use crate::file_io::read_journal_events;
use crate::file_io::report_storage_open_error;
use crate::agent_io::{cmd_agent_context, cmd_status, cmd_system_status};
use crate::agent_io::{cmd_action_list, cmd_action_inspect};
use crate::action::write_action_registry_uninitialized;
use crate::action::write_action_registry;
use crate::action::write_action_inspect;
use crate::action::write_action_registry_error;
use crate::action_specs::{registered_cli_actions, cli_action_specs, action_contract};
use crate::verify::cmd_verify;
use crate::validate::cmd_validate;
use crate::compile::cmd_compile;
use crate::run::cmd_run;
use crate::submit::cmd_submit;
use crate::run_step::{cmd_run_step, compile_bytes_json};
use crate::step_helpers::{build_step_frame, print_step_result};
use crate::run_compiled::cmd_run_compiled;
use crate::run_compiled_runtime::{run_compiled_workflow, runtime_config_for_durability, map_runtime_inputs};
use crate::run_compiled_runtime::{store_compiled_artifact, report_runtime_error, print_trace_event};
use crate::ipc_serve::cmd_ipc_serve;
use crate::inspect::cmd_inspect;
use crate::events::cmd_events;
use crate::replay::cmd_replay;
use crate::trace::cmd_trace;
use crate::run_ops::{cmd_retry, cmd_resume, cmd_answer};
use crate::run_ops2::{format_cancel_output, write_cancel_event, cmd_cancel};
use crate::incident_diff::{cmd_incident, cmd_diff};
use crate::explain::cmd_explain;
use crate::graph::cmd_graph;
use crate::simulate::cmd_simulate;
use crate::bench_run::cmd_bench_run;
use crate::doctor::cmd_doctor;
use crate::doctor_helpers::cmd_doctor_without_db;
use crate::io_helpers::exit_from_io;
use crate::io_helpers::write_help_stdout;
use crate::io_helpers::write_version_stdout;
use crate::output_utils::write_parse_error_stderr;
use crate::output::{json_out, OutputError, output_error_exit, json_out_exit};
use crate::output::{write_stdout_line, write_stderr_line, write_structured_stderr};
use crate::output::{write_stderr_best_effort, json_error, write_contract_error_json};

use crate::{
    agent_context, cli_envelope, cli_postcard, commands_ai_context, commands_diff,
    commands_incident, commands_journal, commands_status, commands_system_status, commands_verify,
    commands_workflow, deliver_sink,
};
use vb_ipc::client::IpcClient;
use vb_ipc::{IpcCommand, IpcPayload};
use vb_runtime::action::ActionRegistry;

const INPUT_MAPPING_DECODE_FAILED_MESSAGE: &str = "INPUT_MAPPING_FAILED: input-bin decode failed";
const INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count";
const INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot index out of range";

macro_rules! outln {
    ($($arg:tt)*) => {{
        write_stdout_line(format_args!($($arg)*));
    }};
}

macro_rules! errln {
    ($($arg:tt)*) => {{
        write_stderr_line(format_args!($($arg)*));
    }};
}

macro_rules! emit_json_or_return {
    ($value:expr, $format:expr $(,)?) => {{
        if let Err(error) = json_out($value, $format) {
            return output_error_exit(&error);
        }
    }};
}

pub(crate) fn run_from_env() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let requested_output = output_format_from_args(&args);
    let parsed = parse_args(&args);

    match parsed {
        Ok(Command::Help) => exit_from_io(&write_help_stdout(), ExitCode::SUCCESS),
        Ok(Command::Version) => exit_from_io(&write_version_stdout(), ExitCode::SUCCESS),
        Ok(Command::AgentContext { deliver }) => cmd_agent_context(deliver.as_deref()),
        Ok(Command::AiContext { run_id, db, output }) => {
            commands_ai_context::handle(&run_id, &db, output)
        }
        Ok(Command::Status { options, output }) => cmd_status(options, output),
        Ok(Command::SystemStatus { options, output }) => cmd_system_status(options, output),
        Ok(Command::ActionList { output, registry }) => cmd_action_list(output, registry),
        Ok(Command::ActionInspect {
            action_name,
            output,
            registry,
        }) => cmd_action_inspect(action_name, output, registry),
        Ok(Command::Verify {
            workflow,
            profile,
            output,
        }) => cmd_verify(&workflow, profile, output),
        Ok(Command::Validate { workflow, output }) => cmd_validate(&workflow, output),
        Ok(Command::Explain { workflow, output }) => cmd_explain(&workflow, output),
        Ok(Command::Compile {
            workflow,
            emit,
            out,
            output,
        }) => cmd_compile(&workflow, emit, &out, output),
        Ok(Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            step,
            output,
        }) => match step {
            Some(target) => cmd_run_step(&workflow, durability, &target, output),
            None => cmd_run(&workflow, &input_bin, durability, db.as_deref(), output),
        },
        Ok(Command::RunCompiled {
            workflow,
            input_bin,
            durability,
            db,
            output,
        }) => cmd_run_compiled(&workflow, &input_bin, durability, db.as_deref(), output),
        Ok(Command::IpcServe { socket, db }) => cmd_ipc_serve(&socket, &db),
        Ok(Command::Inspect { run_id, db, output }) => cmd_inspect(&run_id, &db, output),
        Ok(Command::Events {
            run_id,
            db,
            output,
            status,
            limit,
        }) => cmd_events(&run_id, &db, output, status, limit),
        Ok(Command::Replay { run_id, db, output }) => cmd_replay(&run_id, &db, output),
        Ok(Command::Trace {
            run_id,
            db,
            output,
            filters,
        }) => cmd_trace(&run_id, &db, output, filters),
        Ok(Command::Retry { run_id, db, output }) => cmd_retry(&run_id, &db, output),
        Ok(Command::Resume { run_id, db, output }) => cmd_resume(&run_id, &db, output),
        Ok(Command::BenchRun { workflow, output }) => cmd_bench_run(&workflow, output),
        Ok(Command::Doctor { db, output }) => cmd_doctor(db.as_deref(), output),
        Ok(Command::Answer {
            run_id,
            step,
            value_file,
            db,
            output,
        }) => cmd_answer(&run_id, step, &value_file, &db, output),
        Ok(Command::Graph { workflow, output }) => cmd_graph(&workflow, output),
        Ok(Command::Diff {
            run_a,
            run_b,
            db,
            output,
        }) => cmd_diff(&run_a, &run_b, &db, output),
        Ok(Command::Incident { run_id, db, output }) => cmd_incident(&run_id, &db, output),
        Ok(Command::Submit {
            workflow,
            input_bin,
            db,
            durability,
            output,
        }) => cmd_submit(&workflow, &input_bin, &db, durability, output),
        Ok(Command::Simulate { workflow, output }) => cmd_simulate(&workflow, output),
        Ok(Command::Cancel {
            run_id,
            db,
            reason,
            output,
        }) => cmd_cancel(&run_id, &db, reason, output),
        Err(e) => exit_from_io(
            &write_parse_error_stderr(&e, requested_output),
            CliExitCode::ValidationFailed.into(),
        ),
    }
}

#[path = "app_impl_tests.rs"]
mod tests;
