use super::{Command, Route};
use crate::io_helpers::{exit_from_io, write_subcommand_help, write_version_stdout};
use std::ops::ControlFlow::{Break, Continue};
use std::process::ExitCode;

pub(super) fn information(command: Command) -> Route {
    match command {
        Command::Help { command } => Break(exit_from_io(
            &write_subcommand_help(command.as_deref()),
            ExitCode::SUCCESS,
        )),
        Command::Version => Break(exit_from_io(&write_version_stdout(), ExitCode::SUCCESS)),
        Command::AgentContext { deliver } => {
            Break(crate::agent_io::cmd_agent_context(deliver.as_deref()))
        }
        Command::AiContext { run_id, db, output } => {
            Break(crate::commands_ai_context::handle(&run_id, &db, output))
        }
        other => Continue(other),
    }
}

pub(super) fn inspection(command: Command) -> Route {
    match command {
        Command::Status { options, output } => Break(crate::agent_io::cmd_status(options, output)),
        Command::SystemStatus { options, output } => {
            Break(crate::agent_io::cmd_system_status(options, output))
        }
        Command::ActionList { output, registry } => {
            Break(crate::agent_io::cmd_action_list(output, registry))
        }
        Command::ActionInspect {
            action_name,
            output,
            registry,
        } => Break(crate::agent_io::cmd_action_inspect(
            action_name,
            output,
            registry,
        )),
        other => Continue(other),
    }
}
