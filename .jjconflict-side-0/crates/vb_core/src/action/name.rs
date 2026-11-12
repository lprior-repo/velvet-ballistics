use serde::{Deserialize, Serialize};

/// Maximum length for an action name.
const MAX_ACTION_NAME_LENGTH: usize = 64;

/// Error type for invalid action names.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionNameError {
    /// Action name is empty or whitespace-only.
    #[error("action name is empty")]
    Empty,
    /// Action name exceeds maximum length of 64 characters.
    #[error("action name exceeds maximum length of {MAX_ACTION_NAME_LENGTH} characters")]
    TooLong,
    /// Action name contains whitespace.
    #[error("action name contains whitespace")]
    ContainsWhitespace,
}

/// A validated action name.
///
/// An action name is a non-empty string with no whitespace and at most 64 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionName(String);

impl ActionName {
    /// Creates a new validated action name.
    ///
    /// Returns `Err(ActionNameError)` if the name is empty, too long, or contains whitespace.
    pub fn new(s: impl Into<String>) -> Result<Self, ActionNameError> {
        let s = s.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }

    /// Creates a new action name from a string slice, skipping validation.
    ///
    /// Use this constructor ONLY when the input is known at compile time or
    /// is a hardcoded literal that the caller has verified to be non-empty,
    /// free of whitespace, and within `MAX_ACTION_NAME_LENGTH`. Examples:
    /// test fixtures with hardcoded names, `const` tables of action names,
    /// or generated code emitting validated literals.
    ///
    /// Unlike [`ActionName::new`], this constructor does NOT validate the
    /// input and is purely infallible (no Result, no panic). The caller
    /// bears the responsibility for upholding the validation invariants.
    /// The naming convention `from_static_infallible` makes the trust
    /// boundary explicit at the call site.
    ///
    /// For runtime/derived input where the value is not statically known,
    /// use the fallible [`ActionName::new`] and propagate the error.
    pub fn from_static_infallible(s: impl Into<String>) -> Self {
        ActionName(s.into())
    }

    /// Validates an action name string.
    fn validate(s: &str) -> Result<(), ActionNameError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ActionNameError::Empty);
        }
        if trimmed.len() > MAX_ACTION_NAME_LENGTH {
            return Err(ActionNameError::TooLong);
        }
        if trimmed.chars().any(char::is_whitespace) {
            return Err(ActionNameError::ContainsWhitespace);
        }
        Ok(())
    }

    /// Returns the action name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.trim()
    }
}

impl std::fmt::Display for ActionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for ActionName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
