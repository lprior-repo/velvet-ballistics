use super::kind::CompileError;
use vb_core::SymbolicCode;

/// Multiple compilation errors collected in one pass (railway programming).
#[derive(Debug, PartialEq, Eq)]
pub struct CompileErrors(pub Vec<CompileError>);

impl CompileErrors {
    /// Returns the first error, or None if empty (should not happen by construction).
    #[must_use]
    pub fn first(&self) -> Option<&CompileError> {
        self.0.first()
    }

    /// Returns all collected errors as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[CompileError] {
        &self.0
    }

    /// Iterates over collected errors in reporting order.
    #[allow(clippy::iter_without_into_iter)]
    pub fn iter(&self) -> std::slice::Iter<'_, CompileError> {
        self.0.iter()
    }

    /// Iterates over stable machine-readable diagnostic codes in reporting order.
    pub fn diagnostic_codes(&self) -> impl Iterator<Item = SymbolicCode> + '_ {
        self.0.iter().map(CompileError::code)
    }

    /// Total number of collected errors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no errors (should never happen by construction).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for CompileErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, error) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "[{i}] {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CompileErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.first() {
            Some(error) => Some(error),
            None => None,
        }
    }
}

/// Appends an error to the collector, if the result is `Err`.
pub(crate) fn collect(errors: &mut Vec<CompileError>, result: Result<(), CompileError>) {
    if let Err(error) = result {
        errors.push(error);
    }
}
