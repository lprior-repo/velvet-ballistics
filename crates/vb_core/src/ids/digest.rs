//! Digest of source workflow or compiled IR bytes.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Digest of source workflow or compiled IR bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct WorkflowDigest([u8; 32]);

impl WorkflowDigest {
    /// Creates a digest from already-computed bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the digest bytes.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}
