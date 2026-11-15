//! Command dispatch: routes parsed `Command` to handler modules.
#![forbid(unsafe_code)]

use crate::args::{Command, ParseError, parse_args};
use crate::exit_code::CliExitCode;
use crate::io_helpers::{exit_from_io, write_parse_error_stderr};
use std::ffi::OsString;
use std::ops::ControlFlow;
use std::process::ExitCode;
mod execution;
mod journal;
mod metadata;
mod source;

type Route = ControlFlow<ExitCode, Command>;

pub(crate) fn run_from_env() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    match parse_args(&args) {
        Ok(command) => match dispatch(command) {
            ControlFlow::Break(exit) => exit,
            ControlFlow::Continue(command) => unrouted_command(&command),
        },
        Err(error) => parse_failure(&error),
    }
}

fn dispatch(command: Command) -> Route {
    let command = metadata::information(command)?;
    let command = metadata::inspection(command)?;
    let command = source::analysis(command)?;
    let command = source::artifacts(command)?;
    let command = source::simulations(command)?;
    let command = source::comparison(command)?;
    let command = execution::source(command)?;
    let command = execution::artifact(command)?;
    let command = execution::submission(command)?;
    let command = execution::server(command)?;
    let command = journal::reading(command)?;
    let command = journal::tracing(command)?;
    let command = journal::replay(command)?;
    let command = journal::retry(command)?;
    let command = journal::answer(command)?;
    let command = journal::cancel(command)?;
    let command = journal::maintenance(command)?;
    journal::incidents(command)
}

fn parse_failure(error: &ParseError) -> ExitCode {
    exit_from_io(
        &write_parse_error_stderr(error, crate::args::OutputFormat::Text),
        CliExitCode::ValidationFailed.into(),
    )
}

fn unrouted_command(command: &Command) -> ExitCode {
    parse_failure(&ParseError::InvalidArgument(format!(
        "unrouted command: {command:?}"
    )))
}
