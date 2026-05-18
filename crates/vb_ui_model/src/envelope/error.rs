//! Envelope error types.

use core::fmt;

/// Errors that can occur during envelope construction and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnvelopeError {
    /// The schema version value is outside the valid range.
    InvalidSchemaVersion {
        /// The invalid version value.
        value: u16,
    },
    /// A Success-kind envelope cannot carry a diagnostic entry.
    SuccessCannotHaveDiagnostic,
    /// An Error-kind envelope must carry a diagnostic entry.
    ErrorMustHaveDiagnostic,
    /// An envelope cannot have both a data payload and a diagnostic entry.
    DiagnosticAndPayloadMutuallyExclusive,
    /// A DiagnosticReport envelope must have at least one diagnostic entry.
    DiagnosticReportMustHaveDiagnostics,
    /// The `data` field is not permitted for DiagnosticReport envelopes.
    DataFieldNotAllowedForDiagnosticReport,
    /// The number of diagnostic entries exceeds the configured limit.
    DiagnosticLimitExceeded {
        /// Actual number of entries provided.
        len: usize,
        /// Maximum number of entries allowed.
        max: usize,
    },
    /// A string field exceeds the maximum allowed length.
    MessageTooLong {
        /// Actual string length provided.
        len: usize,
        /// Maximum string length allowed.
        max: usize,
    },
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvelopeError::InvalidSchemaVersion { value } => {
                write!(
                    f,
                    "schema version {} is out of valid range 1..=65535",
                    value
                )
            }
            EnvelopeError::SuccessCannotHaveDiagnostic => {
                write!(f, "Success envelope cannot have a diagnostic")
            }
            EnvelopeError::ErrorMustHaveDiagnostic => {
                write!(f, "Error envelope must have a diagnostic")
            }
            EnvelopeError::DiagnosticAndPayloadMutuallyExclusive => {
                write!(f, "envelope cannot have both diagnostic and payload")
            }
            EnvelopeError::DiagnosticReportMustHaveDiagnostics => {
                write!(f, "DiagnosticReport envelope must have diagnostics")
            }
            EnvelopeError::DataFieldNotAllowedForDiagnosticReport => {
                write!(f, "data field is not allowed for DiagnosticReport kind")
            }
            EnvelopeError::DiagnosticLimitExceeded { len, max } => {
                write!(f, "diagnostic entry count {} exceeds maximum {}", len, max)
            }
            EnvelopeError::MessageTooLong { len, max } => {
                write!(f, "string length {} exceeds maximum {}", len, max)
            }
        }
    }
}
