//! Mode error types for CLI exit code mapping.
//!
//! Implements POST-001, POST-002, POST-003, INV-001 through INV-005:
//! - ModeError with all 5 variants from the contract
//! - From<ModeError> for CliExitCode with correct exit codes
//! - command_mode() classifies all 25 Command variants per Mode Activation Matrix

#![forbid(unsafe_code)]

use crate::args::{Command, DurabilityMode};

/// Command activation mode classification.
/// Determines which subsystems a command activates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandMode {
    /// Pure mode: no storage, no runtime, no UI
    Pure,
    /// Storage mode: FjallJournal required
    Storage,
    /// Runtime mode: Runtime + Storage required
    Runtime,
    /// UI mode: Makepad required (not yet implemented)
    UI,
}

/// Mode-specific errors from CLI command handlers.
#[derive(Debug, Clone)]
pub(crate) enum ModeError {
    /// Defensive: unrecognized command variant in main match arm.
    InvalidMode,
    /// Storage initialization failed (FjallJournal::open error).
    StorageInitFailed {
        path: std::path::PathBuf,
        cause: String,
    },
    /// Runtime initialization failed.
    RuntimeInitFailed { cause: String },
    /// UI initialization failed (Makepad).
    UiInitFailed { cause: String },
    /// DEFECT: pure command attempted to access storage.
    PureCommandStorageAccessAttempted { command: String },
}

impl std::fmt::Display for ModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMode => write!(f, "invalid command mode"),
            Self::StorageInitFailed { path, cause } => {
                write!(f, "storage init failed at {}: {}", path.display(), cause)
            }
            Self::RuntimeInitFailed { cause } => write!(f, "runtime init failed: {}", cause),
            Self::UiInitFailed { cause } => write!(f, "UI init failed: {}", cause),
            Self::PureCommandStorageAccessAttempted { command } => {
                write!(
                    f,
                    "pure command '{command}' attempted storage access (contract violation)"
                )
            }
        }
    }
}

/// Maps ModeError to CliExitCode per Error Taxonomy:
///
/// | ModeError variant                      | CliExitCode            | Exit code |
/// |---------------------------------------|------------------------|-----------|
/// | InvalidMode (defensive)               | ValidationFailed       | 1         |
/// | StorageInitFailed { path, cause }     | StorageError          | 5         |
/// | RuntimeInitFailed { cause }          | RuntimeFailed         | 4         |
/// | UiInitFailed { cause }                | ActionPolicyError     | 7         |
/// | PureCommandStorageAccessAttempted     | StorageError          | 5         |
impl From<ModeError> for crate::CliExitCode {
    fn from(err: ModeError) -> Self {
        match err {
            ModeError::InvalidMode => crate::CliExitCode::ValidationFailed,
            ModeError::StorageInitFailed { .. } => crate::CliExitCode::StorageError,
            ModeError::RuntimeInitFailed { .. } => crate::CliExitCode::RuntimeFailed,
            ModeError::UiInitFailed { .. } => crate::CliExitCode::ActionPolicyError,
            ModeError::PureCommandStorageAccessAttempted { .. } => crate::CliExitCode::StorageError,
        }
    }
}

/// Classifies each Command variant into its activation mode.
///
/// From the Mode Activation Matrix in contract.md:
///
/// | Command       | Mode     |
/// |---------------|----------|
/// | validate      | Pure     |
/// | verify        | Pure     |
/// | explain       | Pure     |
/// | compile       | Pure     |
/// | graph         | Pure     |
/// | simulate      | Pure     |
/// | bench-run     | Pure     |
/// | agent-context | Pure     |
/// | status       | Pure     |
/// | action list   | Pure     |
/// | action inspect| Pure     |
/// | run (dur=none)| Pure     |
/// | run (dur=*)   | Storage  |
/// | run-compiled  | Storage  |
/// | submit        | Storage  |
/// | inspect       | Storage  |
/// | events        | Storage  |
/// | replay        | Storage  |
/// | trace         | Storage  |
/// | retry         | Storage  |
/// | resume        | Storage  |
/// | doctor        | Storage  |
/// | answer        | Storage  |
/// | diff          | Storage  |
/// | incident      | Storage  |
/// | ai-context    | Storage  |
/// | ipc-serve     | Runtime  |
/// | ui            | UI (not implemented) |
pub(crate) fn command_mode(cmd: &Command) -> CommandMode {
    match cmd {
        // Pure commands (11)
        Command::AgentContext => CommandMode::Pure,
        Command::Validate { .. } => CommandMode::Pure,
        Command::Verify { .. } => CommandMode::Pure,
        Command::Explain { .. } => CommandMode::Pure,
        Command::Compile { .. } => CommandMode::Pure,
        Command::Graph { .. } => CommandMode::Pure,
        Command::Simulate { .. } => CommandMode::Pure,
        Command::BenchRun { .. } => CommandMode::Pure,
        Command::Status { .. } => CommandMode::Pure,
        Command::ActionList { .. } => CommandMode::Pure,
        Command::ActionInspect { .. } => CommandMode::Pure,

        // Storage commands (14)
        // Run with durability != None is Storage; with durability == None is Pure
        Command::Run { durability, .. } => match durability {
            DurabilityMode::None => CommandMode::Pure,
            DurabilityMode::Strict | DurabilityMode::Journaled => CommandMode::Storage,
        },
        Command::RunCompiled { .. } => CommandMode::Storage,
        Command::Submit { .. } => CommandMode::Storage,
        Command::Inspect { .. } => CommandMode::Storage,
        Command::Events { .. } => CommandMode::Storage,
        Command::Replay { .. } => CommandMode::Storage,
        Command::Trace { .. } => CommandMode::Storage,
        Command::Retry { .. } => CommandMode::Storage,
        Command::Resume { .. } => CommandMode::Storage,
        Command::Doctor { .. } => CommandMode::Storage,
        Command::Answer { .. } => CommandMode::Storage,
        Command::Diff { .. } => CommandMode::Storage,
        Command::Incident { .. } => CommandMode::Storage,
        Command::AiContext { .. } => CommandMode::Storage,

        // Runtime commands (1)
        Command::IpcServe { .. } => CommandMode::Runtime,

        // UI commands (0 - not yet implemented)
        // Command::Ui { .. } => CommandMode::UI,

        // Help and Version don't go through command_mode in practice,
        // but classify them as Pure for completeness
        Command::Help | Command::Version => CommandMode::Pure,

        // Cancel command - classify as Storage since it operates on runs
        Command::Cancel { .. } => CommandMode::Storage,
    }
}
