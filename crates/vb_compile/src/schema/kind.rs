#![forbid(unsafe_code)]
//! Schema kind enumeration and parsing.
//!
//! Maps YAML shorthand/longhand tokens to the six canonical schema types.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaKind {
    Text,
    Number,
    Boolean,
    Object,
    List,
    Any,
}

impl SchemaKind {
    pub(crate) fn from_long_form(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "number" => Some(Self::Number),
            "boolean" => Some(Self::Boolean),
            "object" => Some(Self::Object),
            "list" => Some(Self::List),
            "any" => Some(Self::Any),
            _ => None,
        }
    }

    pub(crate) fn from_list_element(value: &str) -> Option<Self> {
        match value {
            "any" => Some(Self::Any),
            "text" => Some(Self::Text),
            "number" => Some(Self::Number),
            "boolean" => Some(Self::Boolean),
            "object" => Some(Self::Object),
            _ => None,
        }
    }

    /// Returns whether this kind represents a text type (for bound-kind checks).
    pub(crate) fn is_text(self) -> bool {
        matches!(self, Self::Text)
    }

    /// Returns whether this kind accepts `min`/`max` numeric or list-length bounds.
    pub(crate) fn accepts_numeric_bounds(self) -> bool {
        matches!(self, Self::Number | Self::List)
    }
}
