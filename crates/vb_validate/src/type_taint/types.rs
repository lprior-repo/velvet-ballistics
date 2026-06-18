#![forbid(unsafe_code)]
//! Core value types and taint lattice for workflow type/taint validation.
//!
//! Provides the domain types that model workflow values, the taint propagation
//! lattice, and combined value facts that travel through steps.

use crate::ValidationResult;

// ---------------------------------------------------------------------------
// Public value types
// ---------------------------------------------------------------------------

/// Supported value types for type checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValueType {
    /// Null type.
    Null,
    /// Boolean type.
    Boolean,
    /// Numeric type (integer or float).
    Number,
    /// Text/string type.
    Text,
    /// Object type.
    Object,
    /// List type.
    List,
    /// Any type (type checking passes for all operations).
    Any,
}

impl ValueType {
    /// Returns the stable type name for diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Text => "text",
            Self::Object => "object",
            Self::List => "list",
            Self::Any => "any",
        }
    }
}

/// Taint marker for secret propagation tracking.
///
/// Lattice: Clean < DerivedFromSecret < Secret
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Taint {
    /// No secret-derived data.
    Clean,
    /// Contains or derives from secret data.
    DerivedFromSecret,
    /// Contains or derives from secret data.
    Secret,
}

impl Taint {
    /// Merges two taint markers; secret taint propagates upward through the lattice.
    ///
    /// Lattice rules (highest taint wins):
    /// - Clean + anything = anything
    /// - DerivedFromSecret + Secret = Secret
    /// - Secret + anything = Secret
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Secret, _) | (_, Self::Secret) => Self::Secret,
            (Self::DerivedFromSecret, _) | (_, Self::DerivedFromSecret) => Self::DerivedFromSecret,
            (Self::Clean, Self::Clean) => Self::Clean,
        }
    }
}

/// Combined type and taint fact for a value or slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueFact {
    /// Inferred value type.
    pub value_type: ValueType,
    /// Secret taint status.
    pub taint: Taint,
}

impl ValueFact {
    /// Creates a clean fact with the given type.
    pub const fn clean(value_type: ValueType) -> Self {
        Self {
            value_type,
            taint: Taint::Clean,
        }
    }

    /// Creates a secret-tainted fact with the given type.
    pub const fn secret(value_type: ValueType) -> Self {
        Self {
            value_type,
            taint: Taint::Secret,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal: step requirement helper
// ---------------------------------------------------------------------------

/// Requires that a value type is compatible with a boolean context.
///
/// `Boolean` and `Any` are accepted; all other types produce a
/// [`ValidationError::TypeMismatch`] error.
pub(super) fn require_boolean(actual: ValueType) -> ValidationResult<()> {
    if matches!(actual, ValueType::Boolean | ValueType::Any) {
        Ok(())
    } else {
        Err(crate::ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: actual.as_str().to_owned(),
        })
    }
}
