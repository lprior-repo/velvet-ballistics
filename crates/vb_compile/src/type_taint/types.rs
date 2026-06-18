#![forbid(unsafe_code)]
//! Value type and taint fact types for compile-time type taint analysis.

use vb_validate::type_taint::Taint;

/// The static type of a value in the workflow language.
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

/// A combined value type + taint fact for flow analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValueFact {
    pub(crate) value_type: ValueType,
    pub(crate) taint: Taint,
}

impl ValueFact {
    pub(crate) const fn clean(value_type: ValueType) -> Self {
        Self {
            value_type,
            taint: Taint::Clean,
        }
    }

    pub(crate) const fn merge(self, other: Self) -> Self {
        let taint = match (self.taint, other.taint) {
            (Taint::Secret, _) | (_, Taint::Secret) => Taint::Secret,
            (Taint::DerivedFromSecret, _) | (_, Taint::DerivedFromSecret) => {
                Taint::DerivedFromSecret
            }
            (Taint::Clean, Taint::Clean) => Taint::Clean,
            // SAFETY: Taint is marked #[non_exhaustive]. This arm handles any
            // future variants conservatively as Secret (most restrictive).
            #[allow(unreachable_code)]
            (_, _) => Taint::Secret,
        };
        Self {
            value_type: self.value_type,
            taint,
        }
    }
}
