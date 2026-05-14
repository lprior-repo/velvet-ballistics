#![forbid(unsafe_code)]

use std::process::ExitCode;

/// Stable exit codes for the velvet-ballastics CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
pub(crate) enum CliExitCode {
    Success = 0,
    ValidationFailed = 1,
    VerificationFailed = 2,
    CompileFailed = 3,
    RuntimeFailed = 4,
    StorageError = 5,
    IpcError = 6,
    ActionPolicyError = 7,
    ReplayDivergence = 8,
}

impl From<CliExitCode> for ExitCode {
    fn from(code: CliExitCode) -> Self {
        #[allow(clippy::as_conversions)]
        ExitCode::from(code as u8)
    }
}

impl From<vb_core::errors::CoreError> for CliExitCode {
    fn from(err: vb_core::errors::CoreError) -> Self {
        let _ = err;
        CliExitCode::RuntimeFailed
    }
}

impl From<vb_storage::error::JournalError> for CliExitCode {
    fn from(err: vb_storage::error::JournalError) -> Self {
        let _ = err;
        CliExitCode::StorageError
    }
}
