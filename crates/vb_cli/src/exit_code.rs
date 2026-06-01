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
    /// Runtime execution or step evaluation failed.
    RuntimeFailed = 1,
    /// Input validation or argument parsing failed.
    ValidationFailed = 2,
    /// Workflow compilation or code generation failed.
    CompileFailed = 3,
    /// Workflow verification (e.g. step isolation precondition) failed.
    VerificationFailed = 4,
    /// Storage, journal, or persistence operation failed.
    StorageError = 5,
    /// IPC server operation failed.
    IpcError = 6,
    /// Action policy violation.
    ActionPolicyError = 7,
    /// Replay divergence detected, including domain-specific rule divergence
    /// after the internal error has been mapped to a public CLI status.
    ReplayDivergence = 8,
}

impl From<CliExitCode> for ExitCode {
    fn from(code: CliExitCode) -> Self {
        ExitCode::from(u8::from(code))
    }
}

impl From<CliExitCode> for u8 {
    fn from(code: CliExitCode) -> Self {
        match code {
            CliExitCode::Success => 0,
            CliExitCode::RuntimeFailed => 1,
            CliExitCode::ValidationFailed => 2,
            CliExitCode::CompileFailed => 3,
            CliExitCode::VerificationFailed => 4,
            CliExitCode::StorageError => 5,
            CliExitCode::IpcError => 6,
            CliExitCode::ActionPolicyError => 7,
            CliExitCode::ReplayDivergence => 8,
        }
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
        assert_eq!(u8::from(CliExitCode::Success), 0);
        assert_eq!(u8::from(CliExitCode::RuntimeFailed), 1);
        assert_eq!(u8::from(CliExitCode::ValidationFailed), 2);
        assert_eq!(u8::from(CliExitCode::CompileFailed), 3);
        assert_eq!(u8::from(CliExitCode::VerificationFailed), 4);
        assert_eq!(u8::from(CliExitCode::StorageError), 5);
        assert_eq!(u8::from(CliExitCode::IpcError), 6);
        assert_eq!(u8::from(CliExitCode::ActionPolicyError), 7);
        assert_eq!(u8::from(CliExitCode::ReplayDivergence), 8);
    }

    #[test]
    fn from_cli_exit_code_to_exit_code() {
        assert_eq!(ExitCode::from(CliExitCode::Success), ExitCode::SUCCESS);
        assert_eq!(
            ExitCode::from(CliExitCode::RuntimeFailed),
            ExitCode::from(1u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::ValidationFailed),
            ExitCode::from(2u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::CompileFailed),
            ExitCode::from(3u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::VerificationFailed),
            ExitCode::from(4u8)
        );
        assert_eq!(
            ExitCode::from(CliExitCode::StorageError),
            ExitCode::from(5u8)
        );
        assert_eq!(ExitCode::from(CliExitCode::IpcError), ExitCode::from(6u8));
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
            u8::from(CliExitCode::Success),
            u8::from(CliExitCode::ValidationFailed),
            u8::from(CliExitCode::VerificationFailed),
            u8::from(CliExitCode::CompileFailed),
            u8::from(CliExitCode::RuntimeFailed),
            u8::from(CliExitCode::StorageError),
            u8::from(CliExitCode::IpcError),
            u8::from(CliExitCode::ActionPolicyError),
            u8::from(CliExitCode::ReplayDivergence),
        ];
        let mut sorted: Vec<u8> = values.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), values.len(), "duplicate discriminant found");
    }

    #[test]
    fn all_variants_are_public_range_0_to_8() {
        let values: [u8; 9] = [
            u8::from(CliExitCode::Success),
            u8::from(CliExitCode::ValidationFailed),
            u8::from(CliExitCode::VerificationFailed),
            u8::from(CliExitCode::CompileFailed),
            u8::from(CliExitCode::RuntimeFailed),
            u8::from(CliExitCode::StorageError),
            u8::from(CliExitCode::IpcError),
            u8::from(CliExitCode::ActionPolicyError),
            u8::from(CliExitCode::ReplayDivergence),
        ];

        assert!(values.iter().all(|value| *value <= 8));
    }
}
