use serde::{Deserialize, Serialize};

/// 32-byte gate digest, in the style of [`crate::policy::PolicyDigest`].
/// Callers supply the actual digest of the artifact they computed; never
/// fabricated or default-filled with zeros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GateDigest([u8; 32]);

impl GateDigest {
    /// Creates a digest from verified bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Numeric gate identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GateId(u64);

impl GateId {
    /// Creates a gate identifier from a validated integer.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
