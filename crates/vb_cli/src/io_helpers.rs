//! Module: io_helpers

use std::io::{self, Write};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::args::{OutputFormat, ParseError, VALID_COMMANDS};
use crate::constants::{HELP, VERSION};
use crate::exit_code::CliExitCode;

pub(crate) fn unique_doctor_run_id() -> u64 {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return u64::MAX;
    };
    match u64::try_from(now.as_nanos()) {
        Ok(value) => value,
        Err(_) => now.as_secs(),
    }
}

// --- Helpers ---

pub(crate) fn exit_from_io(result: &io::Result<()>, success_code: ExitCode) -> ExitCode {
    match result {
        Ok(()) => success_code,
        Err(_) => CliExitCode::ValidationFailed.into(),
    }
}

pub(crate) fn write_help_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{HELP}")
}

pub(crate) fn write_version_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "velvet-ballistics {VERSION}")
}

pub(crate) fn write_error_stderr(error: &ParseError) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    match error {
        ParseError::InvalidStep(step) => {
            writeln!(handle, "invalid step: {step}\n\n{HELP}")
        }
        ParseError::MissingArgument(name) => {
            writeln!(handle, "missing argument: {name}\n\n{HELP}")
        }
        ParseError::UnknownEmitTarget(target) => {
            writeln!(
                handle,
                "unknown emit target: {target} (expected: ir, yaml, postcard)\n\n{HELP}"
            )
        }
        ParseError::UnknownDurability(mode) => {
            writeln!(
                handle,
                "unknown durability mode: {mode} (expected: strict, journaled, none)\n\n{HELP}"
            )
        }
        ParseError::UnknownCommand(cmd) => {
            writeln!(
                handle,
                "unknown command: {cmd} (expected one of: {VALID_COMMANDS})\n\n{HELP}"
            )
        }
        ParseError::InvalidStatusArgument(reason) => {
            writeln!(handle, "invalid status argument: {reason}\n\n{HELP}")
        }
        ParseError::InvalidTraceArgument(reason) => {
            writeln!(handle, "invalid trace argument: {reason}\n\n{HELP}")
        }
        ParseError::UnknownEventStatus(status) => {
            writeln!(
                handle,
                "unknown event status: {status} (expected: pending, active, waiting_answer, cancelled, completed, failed)\n\n{HELP}"
            )
        }
        ParseError::InvalidAgentContextArgument(reason) => {
            writeln!(handle, "invalid agent-context argument: {reason}\n\n{HELP}")
        }
        ParseError::UnknownActionCommand(cmd) => {
            writeln!(
                handle,
                "unknown action command: {cmd} (expected: list, inspect)\n\n{HELP}"
            )
        }
        ParseError::UnknownActionRegistry(registry) => {
            writeln!(
                handle,
                "unknown action registry: {registry} (expected: registered, empty, uninitialized)\n\n{HELP}"
            )
        }
        ParseError::MissingActionRegistryValue => writeln!(
            handle,
            "missing action-args value for --registry (expected: registered, empty, uninitialized)\n\n{HELP}"
        ),
        ParseError::UnknownActionListFlag(flag) => {
            writeln!(handle, "unknown action list flag: {flag}\n\n{HELP}")
        }
        ParseError::UnexpectedActionListArgument(argument) => writeln!(
            handle,
            "unexpected action list argument: {argument}\n\n{HELP}"
        ),
        ParseError::InvalidActionListArgument(reason) => {
            writeln!(handle, "invalid action list argument: {reason}\n\n{HELP}")
        }
        ParseError::UnknownActionInspectFlag(flag) => {
            writeln!(handle, "unknown action inspect flag: {flag}\n\n{HELP}")
        }
        ParseError::UnexpectedActionInspectArgument(argument) => writeln!(
            handle,
            "unexpected action inspect argument: {argument}\n\n{HELP}"
        ),
        ParseError::InvalidActionInspectArgument(reason) => writeln!(
            handle,
            "invalid action inspect argument: {reason}\n\n{HELP}"
        ),
        ParseError::InvalidActionId(action_id) => {
            writeln!(handle, "invalid action id: {action_id}\n\n{HELP}")
        }
        ParseError::InvalidActionName(name) => {
            writeln!(handle, "invalid action name: {name}\n\n{HELP}")
        }
        ParseError::UnknownFlag { command, flag } => {
            writeln!(handle, "unknown flag for {command}: {flag}\n\n{HELP}")
        }
        ParseError::InvalidArgument(reason) => {
            writeln!(handle, "invalid argument: {reason}\n\n{HELP}")
        }
        ParseError::NoCommand => {
            writeln!(handle, "{HELP}")
        }
        ParseError::UnknownProfile(profile) => {
            writeln!(
                handle,
                "unknown verify profile: {profile} (expected: quick, standard, full)\n\n{HELP}"
            )
        }
        ParseError::ReasonTooLong => {
            writeln!(
                handle,
                "reason exceeds maximum length of 256 characters\n\n{HELP}"
            )
        }
        ParseError::UnknownServerMode(mode) => {
            writeln!(
                handle,
                "unknown server mode: {mode} (expected: none; strict and journaled require a backend probe that is not implemented)\n\n{HELP}"
            )
        }
        ParseError::InvalidSystemStatusArgument(reason) => {
            writeln!(handle, "invalid system status argument: {reason}\n\n{HELP}")
        }
        ParseError::InvalidReplayDigest(reason) => {
            writeln!(handle, "invalid replay digest: {reason}\n\n{HELP}")
        }
    }
}

pub(crate) fn write_parse_error_stderr(error: &ParseError, output: OutputFormat) -> io::Result<()> {
    match output {
        OutputFormat::Text => write_error_stderr(error),
        OutputFormat::Yaml | OutputFormat::Postcard => crate::output::write_structured_stderr(
            &serde_json::json!({
                "schema_version": crate::cli_envelope::SCHEMA_VERSION,
                "kind": crate::cli_envelope::kind::DIAGNOSTIC_REPORT,
                "message": error.to_string(),
                "exit_code": u8::from(CliExitCode::ValidationFailed),
            }),
            output,
        ),
    }
}
