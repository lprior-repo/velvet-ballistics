use serde::{Deserialize, Serialize};

/// Maximum length in bytes for an approver identity or role name.
pub const MAX_APPROVER_TEXT_LEN: usize = 128;

/// Approver role required by a gate, e.g. `deploy-approver`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproverRole {
    /// Role name; empty and over-long names are rejected by [`ApproverRole::new`].
    pub name: String,
}

impl ApproverRole {
    /// Creates a role, or `None` when the name is empty or over-long.
    #[must_use]
    pub fn new(name: String) -> Option<Self> {
        if name.is_empty() || name.len() > MAX_APPROVER_TEXT_LEN {
            None
        } else {
            Some(Self { name })
        }
    }

    /// Returns the role name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns true when the name is non-empty and bounded.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty() && self.name.len() <= MAX_APPROVER_TEXT_LEN
    }
}

/// Identity of a requester or approver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproverId {
    /// Identity string; empty and over-long values are rejected by [`ApproverId::new`].
    pub value: String,
}

impl ApproverId {
    /// Creates an identity, or `None` when the value is empty or over-long.
    #[must_use]
    pub fn new(value: String) -> Option<Self> {
        if value.is_empty() || value.len() > MAX_APPROVER_TEXT_LEN {
            None
        } else {
            Some(Self { value })
        }
    }

    /// Returns the identity string.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns true when the value is non-empty and bounded.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.value.is_empty() && self.value.len() <= MAX_APPROVER_TEXT_LEN
    }
}
