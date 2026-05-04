#![forbid(unsafe_code)]

use std::process::ExitCode;

/// Stable exit codes for the velvet-ballistics CLI.
///
/// Each variant maps to a distinct byte value so that callers and
/// integration tests can match on the process exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
pub(crate) enum CliExitCode {
    /// Operation completed successfully.
    Success = 0,
    /// Input validation or argument parsing failed.
    ValidationFailed = 1,
    /// Workflow verification (e.g. step isolation precondition) failed.
    VerificationFailed = 2,
    /// Workflow compilation or code generation failed.
    CompileFailed = 3,
    /// Runtime execution or step evaluation failed.
    RuntimeFailed = 4,
    /// Storage, journal, or persistence operation failed.
    StorageError = 5,
    /// IPC server operation failed.
    IpcError = 6,
    /// Action policy violation.
    ActionPolicyError = 7,
    /// Replay divergence detected.
    ReplayDivergence = 8,
}

impl From<CliExitCode> for ExitCode {
    fn from(code: CliExitCode) -> Self {
        // SAFETY: #[repr(u8)] guarantees the discriminant fits in u8.
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

#[cfg(test)]
mod tests {
    use super::CliExitCode;
    use std::process::ExitCode;

    #[test]
    fn discriminant_values_match_spec() {
        assert_eq!(CliExitCode::Success as u8, 0);
        assert_eq!(CliExitCode::ValidationFailed as u8, 1);
        assert_eq!(CliExitCode::VerificationFailed as u8, 2);
        assert_eq!(CliExitCode::CompileFailed as u8, 3);
        assert_eq!(CliExitCode::RuntimeFailed as u8, 4);
        assert_eq!(CliExitCode::StorageError as u8, 5);
        assert_eq!(CliExitCode::IpcError as u8, 6);
        assert_eq!(CliExitCode::ActionPolicyError as u8, 7);
        assert_eq!(CliExitCode::ReplayDivergence as u8, 8);
    }

    #[test]
    fn from_cli_exit_code_to_exit_code() {
        assert_eq!(
            ExitCode::from(CliExitCode::Success),
            ExitCode::SUCCESS
        );
        assert_eq!(
            ExitCode::from(CliExitCode::ValidationFailed),
            ExitCode::from(1u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::VerificationFailed),
            ExitCode::from(2u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::CompileFailed),
            ExitCode::from(3u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::RuntimeFailed),
            ExitCode::from(4u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::StorageError),
            ExitCode::from(5u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::IpcError),
            ExitCode::from(6u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::ActionPolicyError),
            ExitCode::from(7u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::ReplayDivergence),
            ExitCode::from(8u8)
        );
    }

    #[test]
    fn from_core_error_maps_to_runtime_failed() {
        let err = vb_core::errors::CoreError::InvalidProgramCounter {
            step: vb_core::StepIdx::ZERO,
        };
        assert_eq!(CliExitCode::from(err), CliExitCode::RuntimeFailed);
    }

    #[test]
    fn from_journal_error_maps_to_storage_error() {
        let err = vb_storage::error::JournalError::KeyCapacity;
        assert_eq!(CliExitCode::from(err), CliExitCode::StorageError);
    }

    #[test]
    fn all_variants_are_distinct() {
        let values: [u8; 9] = [
            CliExitCode::Success as u8,
            CliExitCode::ValidationFailed as u8,
            CliExitCode::VerificationFailed as u8,
            CliExitCode::CompileFailed as u8,
            CliExitCode::RuntimeFailed as u8,
            CliExitCode::StorageError as u8,
            CliExitCode::IpcError as u8,
            CliExitCode::ActionPolicyError as u8,
            CliExitCode::ReplayDivergence as u8,
        ];
        let mut sorted: Vec<u8> = values.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), values.len(), "duplicate discriminant found");
    }
}
