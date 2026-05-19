//! CLI error types that are distinct from exit codes.
//!
//! These represent semantic error conditions that may map to exit codes
//! or be displayed to the user in structured form.

#![forbid(unsafe_code)]

use std::process::ExitCode;

/// Semantic errors produced by CLI commands.
///
/// These are distinct from exit codes which represent only process exit status values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliError {
    /// The provided workspace path does not exist or is not a valid
    /// Velvet workspace directory.
    InvalidWorkspace {
        path: std::path::PathBuf,
        reason: InvalidWorkspaceReason,
    },
    /// The `--workspace` flag was not provided to a command that requires it.
    MissingWorkspaceFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InvalidWorkspaceReason {
    /// Path does not exist.
    DoesNotExist,
    /// Path exists but is not a directory.
    IsNotDirectory,
    /// Path exists but the process lacks permission to access it.
    PermissionDenied,
    /// Path is an empty string.
    Empty,
    /// Path contains parent-directory traversal (`..`).
    TraversalAttempt,
    /// Path contains non-UTF-8 bytes.
    NonUtf8,
    /// Path is missing required workspace markers.
    MissingMarkers,
}

/// Exit code value for validation failed (2).
/// Defined here to avoid circular dependency with exit_code module.
const VALIDATION_FAILED_EXIT_CODE: u8 = 2;

impl CliError {
    /// Returns the appropriate exit code for this error.
    pub(crate) fn exit_code(&self) -> ExitCode {
        match self {
            Self::InvalidWorkspace { .. } | Self::MissingWorkspaceFlag { .. } => {
                ExitCode::from(VALIDATION_FAILED_EXIT_CODE)
            }
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidWorkspace { path, reason } => {
                write!(formatter, "invalid workspace '{}': {reason}", path.display())
            }
            Self::MissingWorkspaceFlag => {
                write!(formatter, "missing argument: --workspace")
            }
        }
    }
}

impl std::fmt::Display for InvalidWorkspaceReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DoesNotExist => write!(formatter, "path does not exist"),
            Self::IsNotDirectory => write!(formatter, "path is not a directory"),
            Self::PermissionDenied => write!(formatter, "permission denied"),
            Self::Empty => write!(formatter, "path is empty"),
            Self::TraversalAttempt => write!(formatter, "path traverses parent directory"),
            Self::NonUtf8 => write!(formatter, "path contains non-UTF-8 characters"),
            Self::MissingMarkers => write!(formatter, "workspace marker files are missing"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn cli_error_invalid_workspace_display_includes_path_and_reason() {
        let err = CliError::InvalidWorkspace {
            path: PathBuf::from("/tmp/test"),
            reason: InvalidWorkspaceReason::DoesNotExist,
        };
        let display = err.to_string();

        assert!(
            display.contains("/tmp/test"),
            "display should contain path, got: {}",
            display
        );
        assert!(
            display.contains("does not exist"),
            "display should contain reason, got: {}",
            display
        );
    }

    #[test]
    fn cli_error_missing_workspace_flag_display() {
        let err = CliError::MissingWorkspaceFlag;
        let display = err.to_string();

        assert!(
            display.contains("missing argument: --workspace"),
            "display should mention missing --workspace, got: {}",
            display
        );
    }

    #[test]
    fn cli_error_exit_code_is_validation_failed_for_invalid_workspace() {
        let err = CliError::InvalidWorkspace {
            path: PathBuf::from("/tmp/test"),
            reason: InvalidWorkspaceReason::DoesNotExist,
        };
        let exit_code = err.exit_code();

        // ValidationFailed = 2; ExitCode::from(2u8) creates exit code 2
        let expected = ExitCode::from(2u8);
        assert_eq!(
            exit_code, expected,
            "InvalidWorkspace should produce exit code 2"
        );
    }

    #[test]
    fn cli_error_exit_code_is_validation_failed_for_missing_workspace_flag() {
        let err = CliError::MissingWorkspaceFlag;
        let exit_code = err.exit_code();

        let expected = ExitCode::from(2u8);
        assert_eq!(
            exit_code, expected,
            "MissingWorkspaceFlag should produce exit code 2"
        );
    }

    #[test]
    fn invalid_workspace_reason_display_messages() {
        let cases = vec![
            (InvalidWorkspaceReason::DoesNotExist, "path does not exist"),
            (InvalidWorkspaceReason::IsNotDirectory, "path is not a directory"),
            (InvalidWorkspaceReason::PermissionDenied, "permission denied"),
            (InvalidWorkspaceReason::Empty, "path is empty"),
            (InvalidWorkspaceReason::TraversalAttempt, "path traverses parent directory"),
            (InvalidWorkspaceReason::NonUtf8, "path contains non-UTF-8 characters"),
            (InvalidWorkspaceReason::MissingMarkers, "workspace marker files are missing"),
        ];

        for (reason, expected_substring) in cases {
            let display = reason.to_string();
            assert!(
                display.contains(expected_substring),
                "reason {:?} should contain '{}', got: {}",
                reason,
                expected_substring,
                display
            );
        }
    }

    #[test]
    fn cli_error_equality() {
        let path = PathBuf::from("/tmp/test");
        let err1 = CliError::InvalidWorkspace {
            path: path.clone(),
            reason: InvalidWorkspaceReason::DoesNotExist,
        };
        let err2 = CliError::InvalidWorkspace {
            path: path.clone(),
            reason: InvalidWorkspaceReason::DoesNotExist,
        };
        let err3 = CliError::InvalidWorkspace {
            path: path.clone(),
            reason: InvalidWorkspaceReason::IsNotDirectory,
        };

        assert_eq!(err1, err2, "same error values should be equal");
        assert_ne!(err1, err3, "different error values should not be equal");
    }
}
