#![forbid(unsafe_code)]

use std::process::ExitCode;

/// Stable exit codes for the velvet-ballistics CLI.
///
/// Each variant maps to a distinct byte value so that callers and
/// integration tests can match on the process exit status.
///
/// vb-k8ut.5: derives `Serialize`/`Deserialize` so the typed
/// `cli_postcard::DiagnosticReport` can carry it directly. Variant names
/// are the serde tags (e.g. `"ValidationFailed"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Input mapping failure: input bin could not be mapped to workflow
    /// slot values.
    InputMappingFailed = 9,
}

impl From<CliExitCode> for ExitCode {
    fn from(code: CliExitCode) -> Self {
        // `#[repr(u8)]` guarantees the discriminant fits in u8; use the
        // explicit From impl below to keep the cast fallibility explicit.
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
            CliExitCode::InputMappingFailed => 9,
        }
    }
}

impl From<vb_core::errors::CoreError> for CliExitCode {
    fn from(err: vb_core::errors::CoreError) -> Self {
        // The From trait requires returning a concrete CliExitCode
        // without propagating the error. All CoreError variants map to
        // RuntimeFailed because the CLI surface cannot carry domain error
        // payloads in its stable exit-code contract.
        drop(err);
        CliExitCode::RuntimeFailed
    }
}

impl From<vb_storage::error::JournalError> for CliExitCode {
    fn from(err: vb_storage::error::JournalError) -> Self {
        // The From trait requires returning a concrete CliExitCode
        // without propagating the error. All JournalError variants map to
        // StorageError because the CLI surface cannot carry domain error
        // payloads in its stable exit-code contract.
        drop(err);
        CliExitCode::StorageError
    }
}

impl From<vb_storage::recovery::RecoveryError> for CliExitCode {
    fn from(err: vb_storage::recovery::RecoveryError) -> Self {
        recovery_error_exit_code(&err)
    }
}

pub(crate) fn recovery_error_exit_code(err: &vb_storage::recovery::RecoveryError) -> CliExitCode {
    match err {
        vb_storage::recovery::RecoveryError::ReplayDivergence { .. } => {
            CliExitCode::ReplayDivergence
        }
        vb_storage::recovery::RecoveryError::Journal(_) => CliExitCode::StorageError,
        _ => CliExitCode::VerificationFailed,
    }
}

impl From<&vb_storage::recovery::RecoveryError> for CliExitCode {
    fn from(err: &vb_storage::recovery::RecoveryError) -> Self {
        recovery_error_exit_code(err)
    }
}

#[cfg(test)]
mod tests {
    use super::{CliExitCode, recovery_error_exit_code};
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
        assert_eq!(u8::from(CliExitCode::InputMappingFailed), 9);
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
        assert_eq!(
            ExitCode::from(CliExitCode::InputMappingFailed),
            ExitCode::from(9u8)
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
    fn from_recovery_error_replay_divergence_maps_to_replay_divergence() {
        let err = vb_storage::recovery::RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: String::from("state trajectory diverged"),
        };

        assert_eq!(CliExitCode::from(err), CliExitCode::ReplayDivergence);
    }

    #[test]
    fn borrowed_recovery_error_maps_without_message_inference() {
        let err = vb_storage::recovery::RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: String::from("journal storage validation compile runtime text"),
        };

        assert_eq!(
            recovery_error_exit_code(&err),
            CliExitCode::ReplayDivergence
        );
        assert_eq!(CliExitCode::from(&err), CliExitCode::ReplayDivergence);
    }

    #[test]
    fn from_recovery_error_journal_maps_to_storage_error() {
        let err = vb_storage::recovery::RecoveryError::Journal(
            vb_storage::error::JournalError::QueueFull,
        );

        assert_eq!(CliExitCode::from(err), CliExitCode::StorageError);
    }

    #[test]
    fn from_recovery_error_other_maps_to_verification_failed() {
        let err = vb_storage::recovery::RecoveryError::NoRecoveryData {
            run: vb_core::RunId::new(42),
        };

        assert_eq!(CliExitCode::from(err), CliExitCode::VerificationFailed);
    }

    #[test]
    fn all_variants_are_distinct() {
        let values: [u8; 10] = [
            u8::from(CliExitCode::Success),
            u8::from(CliExitCode::ValidationFailed),
            u8::from(CliExitCode::VerificationFailed),
            u8::from(CliExitCode::CompileFailed),
            u8::from(CliExitCode::RuntimeFailed),
            u8::from(CliExitCode::StorageError),
            u8::from(CliExitCode::IpcError),
            u8::from(CliExitCode::ActionPolicyError),
            u8::from(CliExitCode::ReplayDivergence),
            u8::from(CliExitCode::InputMappingFailed),
        ];
        let mut sorted: Vec<u8> = values.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), values.len(), "duplicate discriminant found");
    }

    #[test]
    fn all_variants_are_public_range_0_to_9() {
        let values: [u8; 10] = [
            u8::from(CliExitCode::Success),
            u8::from(CliExitCode::ValidationFailed),
            u8::from(CliExitCode::VerificationFailed),
            u8::from(CliExitCode::CompileFailed),
            u8::from(CliExitCode::RuntimeFailed),
            u8::from(CliExitCode::StorageError),
            u8::from(CliExitCode::IpcError),
            u8::from(CliExitCode::ActionPolicyError),
            u8::from(CliExitCode::ReplayDivergence),
            u8::from(CliExitCode::InputMappingFailed),
        ];

        assert!(values.iter().all(|value| *value <= 9));
    }
}
