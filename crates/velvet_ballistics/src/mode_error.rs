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
        | Command::AgentContext { .. }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{Command, DurabilityMode, EmitTarget, OutputFormat, VerifyProfile};
    use std::path::PathBuf;

    fn dummy_path() -> PathBuf {
        PathBuf::from("/tmp/test.wf")
    }

    #[test]
    fn command_mode_returns_pure_for_validate() {
        let cmd = Command::Validate {
            workflow: dummy_path(),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_returns_pure_for_verify() {
        let cmd = Command::Verify {
            workflow: dummy_path(),
            profile: VerifyProfile::Quick,
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_returns_pure_for_explain() {
        let cmd = Command::Explain {
            workflow: dummy_path(),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_returns_pure_for_compile() {
        let cmd = Command::Compile {
            workflow: dummy_path(),
            emit: EmitTarget::Ir,
            out: dummy_path(),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_returns_pure_for_graph() {
        let cmd = Command::Graph {
            workflow: dummy_path(),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_returns_pure_for_help() {
        assert_eq!(command_mode(&Command::Help), CommandMode::Pure);
    }

    #[test]
    fn command_mode_returns_pure_for_version() {
        assert_eq!(command_mode(&Command::Version), CommandMode::Pure);
    }

    #[test]
    fn command_mode_returns_storage_for_run() {
        let cmd = Command::Run {
            workflow: dummy_path(),
            input_bin: dummy_path(),
            durability: DurabilityMode::None,
            db: None,
            step: None,
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_returns_storage_for_inspect() {
        let cmd = Command::Inspect {
            run_id: "1".into(),
            db: dummy_path(),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_returns_storage_for_cancel() {
        let cmd = Command::Cancel {
            run_id: "1".into(),
            reason: None,
            db: dummy_path(),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_returns_storage_for_diff() {
        let cmd = Command::Diff {
            run_a: "1".into(),
            run_b: "2".into(),
            db: dummy_path(),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_returns_runtime_for_ipc_serve() {
        let cmd = Command::IpcServe {
            socket: dummy_path(),
            db: dummy_path(),
        };
        assert_eq!(command_mode(&cmd), CommandMode::Runtime);
    }

    #[test]
    fn command_mode_ui_variant_is_reserved() {
        let mode = CommandMode::UI;
        assert!(matches!(mode, CommandMode::UI));
    }

    #[test]
    fn mode_error_display_invalid_mode() {
        assert_eq!(ModeError::InvalidMode.to_string(), "invalid command mode");
    }

    #[test]
    fn mode_error_display_storage_init_failed() {
        let err = ModeError::StorageInitFailed {
            path: PathBuf::from("/tmp/db"),
            cause: "disk full".into(),
        };
        assert_eq!(err.to_string(), "storage init failed at /tmp/db: disk full");
    }

    #[test]
    fn mode_error_display_runtime_init_failed() {
        let err = ModeError::RuntimeInitFailed {
            cause: "no shards".into(),
        };
        assert_eq!(err.to_string(), "runtime init failed: no shards");
    }

    #[test]
    fn mode_error_display_pure_command_storage_access() {
        let err = ModeError::PureCommandStorageAccessAttempted {
            command: "validate".into(),
        };
        assert_eq!(
            err.to_string(),
            "pure command attempted storage access: validate"
        );
    }

    #[test]
    fn mode_error_converts_to_cli_exit_code_invalid_mode() {
        let code: CliExitCode = ModeError::InvalidMode.into();
        assert_eq!(code, CliExitCode::ValidationFailed);
    }

    #[test]
    fn mode_error_converts_to_cli_exit_code_storage_init_failed() {
        let err = ModeError::StorageInitFailed {
            path: dummy_path(),
            cause: "err".into(),
        };
        let code: CliExitCode = err.into();
        assert_eq!(code, CliExitCode::StorageError);
    }

    #[test]
    fn mode_error_converts_to_cli_exit_code_runtime_init_failed() {
        let err = ModeError::RuntimeInitFailed {
            cause: "err".into(),
        };
        let code: CliExitCode = err.into();
        assert_eq!(code, CliExitCode::RuntimeFailed);
    }

    #[test]
    fn mode_error_converts_to_cli_exit_code_ui_init_failed() {
        let err = ModeError::UiInitFailed {
            cause: "err".into(),
        };
        let code: CliExitCode = err.into();
        assert_eq!(code, CliExitCode::ActionPolicyError);
    }

    #[test]
    fn mode_error_converts_to_cli_exit_code_pure_command_storage_access() {
        let err = ModeError::PureCommandStorageAccessAttempted {
            command: "validate".into(),
        };
        let code: CliExitCode = err.into();
        assert_eq!(code, CliExitCode::StorageError);
    }
}
