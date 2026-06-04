#![forbid(unsafe_code)]
//! CLI command dispatcher.

use crate::action_specs::{
    action_contract, cli_action_specs, registered_cli_actions, write_action_inspect,
    write_action_registry, write_action_registry_error, write_action_registry_uninitialized,
};
use crate::agent_io::{
    cmd_action_inspect, cmd_action_list, cmd_agent_context, cmd_status, cmd_system_status,
};
use crate::args::*;
use crate::bench_run::cmd_bench_run;
use crate::commands_ai_context::*;
use crate::compile::cmd_compile;
use crate::doctor::cmd_doctor;
use crate::doctor_helpers::cmd_doctor_without_db;
use crate::events::cmd_events;
use crate::exit_code::CliExitCode;
use crate::explain::cmd_explain;
use crate::file_io::*;
use crate::graph::cmd_graph;
use crate::incident_diff::{cmd_diff, cmd_diff_workflow_against, cmd_incident};
use crate::inspect::cmd_inspect;
use crate::io_helpers::*;
use crate::ipc_serve::cmd_ipc_serve;
use crate::output::*;
use crate::output_utils::*;
use crate::replay::cmd_replay;
use crate::run::cmd_run;
use crate::run_compiled::cmd_run_compiled;
use crate::run_compiled_runtime::*;
use crate::run_ops::*;
use crate::run_step::{cmd_run_step, compile_bytes_json};
use crate::simulate::cmd_simulate;
use crate::step_helpers::{build_step_frame, print_step_result};
use crate::submit::cmd_submit;
use crate::trace::cmd_trace;
use crate::validate::cmd_validate;
use crate::verify::cmd_verify;
use std::ffi::OsString;
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use vb_ipc::client::IpcClient;
use vb_ipc::{IpcCommand, IpcPayload};
use vb_runtime::action::ActionRegistry;

pub fn run_from_env() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let requested_output = output_format_from_args(&args);
    let parsed = parse_args(&args);

    match parsed {
        Ok(Command::Help) => exit_from_io(&write_help_stdout(), ExitCode::SUCCESS),
        Ok(Command::Version) => exit_from_io(&write_version_stdout(), ExitCode::SUCCESS),
        Ok(Command::AgentContext { deliver }) => cmd_agent_context(deliver.as_deref()),
        Ok(Command::AiContext { run_id, db, output }) => {
            crate::commands_ai_context::handle(&run_id, &db, output)
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
        Ok(Command::Retry {
            run_id,
            step,
            db,
            output,
        }) => cmd_retry(&run_id, step.as_ref(), &db, output),
        Ok(Command::Resume { run_id, db, output }) => cmd_resume(&run_id, &db, output),
        Ok(Command::BenchRun { workflow, output }) => cmd_bench_run(&workflow, output),
        Ok(Command::Doctor { db, output }) => cmd_doctor(db.as_deref(), output),
        Ok(Command::Answer {
            run_id,
            slot,
            value,
            db,
            output,
        }) => cmd_answer(&run_id, slot, &value, &db, output),
        Ok(Command::Graph { workflow, output }) => cmd_graph(&workflow, output),
        Ok(Command::Diff {
            diff_mode,
            output,
        }) => match diff_mode {
            DiffMode::WorkflowAgainst {
                workflow,
                against,
                db,
            } => cmd_diff_workflow_against(&workflow, &against, &db, output),
            DiffMode::RunAgainst {
                run_a,
                run_b,
                db,
            } => cmd_diff(&run_a, &run_b, &db, output),
        },
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
