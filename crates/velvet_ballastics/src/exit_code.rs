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
        let byte = match code {
            CliExitCode::Success => 0u8,
            CliExitCode::ValidationFailed => 1,
            CliExitCode::VerificationFailed => 2,
            CliExitCode::CompileFailed => 3,
            CliExitCode::RuntimeFailed => 4,
            CliExitCode::StorageError => 5,
            CliExitCode::IpcError => 6,
            CliExitCode::ActionPolicyError => 7,
            CliExitCode::ReplayDivergence => 8,
        };
        ExitCode::from(byte)
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

    fn to_byte(code: CliExitCode) -> u8 {
        match code {
            CliExitCode::Success => 0,
            CliExitCode::ValidationFailed => 1,
            CliExitCode::VerificationFailed => 2,
            CliExitCode::CompileFailed => 3,
            CliExitCode::RuntimeFailed => 4,
            CliExitCode::StorageError => 5,
            CliExitCode::IpcError => 6,
            CliExitCode::ActionPolicyError => 7,
            CliExitCode::ReplayDivergence => 8,
        }
    }

    #[test]
    fn discriminant_values_match_spec() {
        assert_eq!(to_byte(CliExitCode::Success), 0);
        assert_eq!(to_byte(CliExitCode::ValidationFailed), 1);
        assert_eq!(to_byte(CliExitCode::VerificationFailed), 2);
        assert_eq!(to_byte(CliExitCode::CompileFailed), 3);
        assert_eq!(to_byte(CliExitCode::RuntimeFailed), 4);
        assert_eq!(to_byte(CliExitCode::StorageError), 5);
        assert_eq!(to_byte(CliExitCode::IpcError), 6);
        assert_eq!(to_byte(CliExitCode::ActionPolicyError), 7);
        assert_eq!(to_byte(CliExitCode::ReplayDivergence), 8);
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
            to_byte(CliExitCode::Success),
            to_byte(CliExitCode::ValidationFailed),
            to_byte(CliExitCode::VerificationFailed),
            to_byte(CliExitCode::CompileFailed),
            to_byte(CliExitCode::RuntimeFailed),
            to_byte(CliExitCode::StorageError),
            to_byte(CliExitCode::IpcError),
            to_byte(CliExitCode::ActionPolicyError),
            to_byte(CliExitCode::ReplayDivergence),
        ];
        let mut sorted: Vec<u8> = values.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), values.len(), "duplicate discriminant found");
    }
}
