#![forbid(unsafe_code)]
//! Core domain types for type taint validation.

use vb_validate::type_taint::Taint;

/// Represents the type of a workflow value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueType {
    Null,
    Boolean,
    Number,
    Text,
    Object,
    List,
    Any,
}

impl ValueType {
    /// Returns the string representation of the value type.
    pub(crate) const fn as_str(self) -> &'static str {
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

/// A fact about a value combining type and taint information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValueFact {
    /// The type of the value.
    pub(crate) value_type: ValueType,
    /// The taint status of the value.
    pub(crate) taint: Taint,
}

impl ValueFact {
    /// Creates a clean (untainted) fact with the given type.
    pub(crate) const fn clean(value_type: ValueType) -> Self {
        Self {
            value_type,
            taint: Taint::Clean,
        }
    }

    /// Merges this fact with another, combining taint status.
    pub(crate) const fn merge(self, other: Self) -> Self {
        let taint = match (self.taint, other.taint) {
            (Taint::Secret, _) | (_, Taint::Secret) => Taint::Secret,
            (Taint::Clean, Taint::Clean) => Taint::Clean,
        };
        Self {
            value_type: self.value_type,
            taint,
        }
    }
}
