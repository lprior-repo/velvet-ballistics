use vb_core::DiagnosticCode;

/// Non-critical issues detected during storage admission.
///
/// These do not prevent admission but should be reported to the caller
/// for logging, monitoring, or informational purposes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum VerificationWarning {
    /// Record schema version is older than current but still compatible.
    #[error(
        "record schema version {found} is older than current {current} — migration may be required"
    )]
    SchemaVersionMismatch {
        /// Found schema version.
        found: u16,
        /// Current schema version.
        current: u16,
    },
}

/// Diagnostic code for schema version mismatch warning.
pub const VERIFICATION_WARNING_SCHEMA_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x5001);

impl VerificationWarning {
    /// Returns the stable diagnostic code for this warning.
    #[must_use]
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::SchemaVersionMismatch { .. } => VERIFICATION_WARNING_SCHEMA_MISMATCH_CODE,
        }
    }
}

/// Container for multiple verification warnings.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct AdmissionWarnings {
    /// List of warnings collected during admission.
    warnings: Vec<VerificationWarning>,
}

impl AdmissionWarnings {
    /// Creates a new empty warnings container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if there are no warnings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Returns the number of warnings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.warnings.len()
    }

    /// Adds a warning to the container.
    pub fn push(&mut self, warning: VerificationWarning) {
        self.warnings.push(warning);
    }

    /// Returns an iterator over the warnings.
    pub fn iter(&self) -> std::slice::Iter<'_, VerificationWarning> {
        self.warnings.iter()
    }
}

impl IntoIterator for AdmissionWarnings {
    type Item = VerificationWarning;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.warnings.into_iter()
    }
}

impl<'a> IntoIterator for &'a AdmissionWarnings {
    type Item = &'a VerificationWarning;
    type IntoIter = std::slice::Iter<'a, VerificationWarning>;

    fn into_iter(self) -> Self::IntoIter {
        self.warnings.iter()
    }
}
