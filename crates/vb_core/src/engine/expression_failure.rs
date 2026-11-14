//! Ordinary data evaluation failures that a FirstSuccess lane may recover from.
use crate::errors::EngineError;

/// Classifies value-domain failures, excluding broken references and resource failures.
#[must_use]
pub const fn is_ordinary_expression_failure(error: &EngineError) -> bool {
    matches!(
        error,
        EngineError::DivisionByZero
            | EngineError::NonFiniteNumber
            | EngineError::TypeMismatch { .. }
            | EngineError::ObjectFieldNotFound { .. }
            | EngineError::ListIndexOutOfBounds { .. }
            | EngineError::UnsupportedAccessorTraversal { .. }
    )
}
