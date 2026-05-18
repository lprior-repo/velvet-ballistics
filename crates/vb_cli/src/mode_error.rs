#![forbid(unsafe_code)]

use std::fmt;
use std::path::PathBuf;

use crate::args::Command;
use crate::exit_code::CliExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandMode {
    Pure,
    Storage,
    Runtime,
    UI,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModeError {
    InvalidMode,
    StorageInitFailed { path: PathBuf, cause: String },
    RuntimeInitFailed { cause: String },
    UiInitFailed { cause: String },
    PureCommandStorageAccessAttempted { command: String },
}

impl From<ModeError> for CliExitCode {
    fn from(error: ModeError) -> Self {
        match error {
            ModeError::InvalidMode => CliExitCode::ValidationFailed,
            ModeError::StorageInitFailed { .. } => CliExitCode::StorageError,
            ModeError::RuntimeInitFailed { .. } => CliExitCode::RuntimeFailed,
            ModeError::UiInitFailed { .. } => CliExitCode::ActionPolicyError,
            ModeError::PureCommandStorageAccessAttempted { .. } => CliExitCode::StorageError,
        }
    }
}

impl fmt::Display for ModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModeError::InvalidMode => write!(f, "invalid command mode"),
            ModeError::StorageInitFailed { path, cause } => {
                write!(f, "storage init failed at {}: {cause}", path.display())
            }
            ModeError::RuntimeInitFailed { cause } => write!(f, "runtime init failed: {cause}"),
            ModeError::UiInitFailed { cause } => write!(f, "ui init failed: {cause}"),
            ModeError::PureCommandStorageAccessAttempted { command } => {
                write!(f, "pure command attempted storage access: {command}")
            }
        }
    }
}

pub(crate) fn command_mode(command: &Command) -> CommandMode {
    match command {
        Command::Validate { .. }
        | Command::Verify { .. }
        | Command::Explain { .. }
        | Command::Compile { .. }
        | Command::Graph { .. }
        | Command::Simulate { .. }
        | Command::BenchRun { .. }
        | Command::AgentContext
        | Command::Status { .. }
        | Command::SystemStatus { .. }
        | Command::ActionList { .. }
        | Command::ActionInspect { .. }
        | Command::Help
        | Command::Version => CommandMode::Pure,
        Command::Run { .. }
        | Command::RunCompiled { .. }
        | Command::Submit { .. }
        | Command::Inspect { .. }
        | Command::Events { .. }
        | Command::Replay { .. }
        | Command::Trace { .. }
        | Command::Retry { .. }
        | Command::Resume { .. }
        | Command::Doctor { .. }
        | Command::Answer { .. }
        | Command::Diff { .. }
        | Command::Incident { .. }
        | Command::Cancel { .. }
        | Command::AiContext { .. } => CommandMode::Storage,
        Command::IpcServe { .. } => CommandMode::Runtime,
    }
}
