#![forbid(unsafe_code)]
#![allow(clippy::should_implement_trait)]

//! Profile name enum for the runtime limits profile matrix.
//!
//! Exactly three variants: `Strict`, `Journaled`, `Relaxed`.
//! No `Other(String)` or `Unknown`.

/// Canonical profile name — exactly three variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProfileName {
    /// Most restrictive limits.
    Strict,
    /// Moderate limits with journaling.
    Journaled,
    /// Most permissive limits.
    Relaxed,
}

impl ProfileName {
    /// Parse a profile name from a static string.
    #[must_use]
    pub fn from_str(s: &'static str) -> Option<Self> {
        match s {
            "strict" | "Strict" => Some(ProfileName::Strict),
            "journaled" | "Journaled" => Some(ProfileName::Journaled),
            "relaxed" | "Relaxed" => Some(ProfileName::Relaxed),
            _ => None,
        }
    }

    /// Returns the human-readable label for this profile name.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            ProfileName::Strict => "Strict",
            ProfileName::Journaled => "Journaled",
            ProfileName::Relaxed => "Relaxed",
        }
    }
}
