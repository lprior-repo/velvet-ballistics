//! Command dispatch: routes parsed `Command` to handler modules.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::process::ExitCode;

use crate::args::{parse_args, Command};
use crate::exit_code::CliExitCode;
use crate::io_helpers::{exit_from_io, write_help_stdout, write_parse_error_stderr, write_version_stdout};

pub(crate) fn run_from_env() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let parsed = parse_args(&args);

    match parsed {
        Ok(Command::Help) => exit_from_io(&write_help_stdout(), ExitCode::SUCCESS),
        Ok(Command::Version) => exit_from_io(&write_version_stdout(), ExitCode::SUCCESS),
        Ok(Command::AgentContext { deliver }) => crate::agent_io::cmd_agent_context(deliver.as_deref()),
        Ok(Command::AiContext { run_id, db, output }) => {
            crate::commands_ai_context::handle(&run_id, &db, output)
        }
        Ok(Command::Status { options, output }) => crate::agent_io::cmd_status(options, output),
        Ok(Command::SystemStatus { options, output }) => crate::agent_io::cmd_system_status(options, output),
        Ok(Command::ActionList { output, registry }) => crate::agent_io::cmd_action_list(output, registry),
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
        Ok(Command::Validate { workflow, output }) => crate::validate::cmd_validate(&workflow, output),
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
        }) => crate::run_compiled::cmd_run_compiled(&workflow, &input_bin, durability, db.as_deref(), output),
        Ok(Command::IpcServe { socket, db }) => crate::ipc_serve::cmd_ipc_serve(&socket, &db),
        Ok(Command::Inspect { run_id, db, output }) => crate::inspect::cmd_inspect(&run_id, &db, output),
        Ok(Command::Events {
            run_id,
            db,
            output,
            status,
            limit,
        }) => crate::events::cmd_events(&run_id, &db, output, status, limit),
        Ok(Command::Replay { run_id, db, output }) => crate::replay::cmd_replay(&run_id, &db, output),
        Ok(Command::Trace {
            run_id,
            db,
            output,
            filters,
        }) => crate::trace::cmd_trace(&run_id, &db, output, filters),
        Ok(Command::Retry { run_id, db, output }) => crate::run_ops::cmd_retry(&run_id, &db, output),
        Ok(Command::Resume { run_id, db, output }) => crate::replay::cmd_resume(&run_id, &db, output),
        Ok(Command::BenchRun { workflow, output }) => crate::bench_run::cmd_bench_run(&workflow, output),
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
        Ok(Command::Incident { run_id, db, output }) => crate::incident_diff::cmd_incident(&run_id, &db, output),
        Ok(Command::Submit {
            workflow,
            input_bin,
            db,
            durability,
            output,
        }) => crate::submit::cmd_submit(&workflow, &input_bin, &db, durability, output),
        Ok(Command::Simulate { workflow, output }) => crate::simulate::cmd_simulate(&workflow, output),
        Ok(Command::Cancel {
            run_id,
            db,
            reason,
            output,
        }) => crate::run_cancel_ops::cmd_cancel(&run_id, &db, reason, output),
        Err(e) => exit_from_io(
            &write_parse_error_stderr(&e, crate::args::OutputFormat::Text),
            CliExitCode::ValidationFailed.into(),
        ),
    }
}
