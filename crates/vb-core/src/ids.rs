//! Compact numeric identifiers used by the hot runtime.

use serde::{Deserialize, Serialize};

/// Numeric workflow identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct WorkflowId(u32);

impl WorkflowId {
    /// Creates a workflow identifier from a validated integer.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw workflow identifier.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Numeric workflow step index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StepIdx(u16);

impl StepIdx {
    /// Creates a step index from a validated integer.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the index as `usize` for checked slice access.
    #[must_use]
    pub fn as_usize(self) -> usize {
        usize::from(self.0)
    }
}

/// Numeric runtime slot index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SlotIdx(u16);

impl SlotIdx {
    /// Creates a slot index from a validated integer.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the index as `usize` for checked slice access.
    #[must_use]
    pub fn as_usize(self) -> usize {
        usize::from(self.0)
    }
}

/// Numeric expression-program index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ExprIdx(u16);

impl ExprIdx {
    /// Creates an expression index from a validated integer.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the index as `usize` for checked slice access.
    #[must_use]
    pub fn as_usize(self) -> usize {
        usize::from(self.0)
    }
}

/// Numeric action-dispatch identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ActionId(u16);

impl ActionId {
    /// Creates an action identifier from a validated integer.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the raw action identifier.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Numeric accessor-program index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct AccessorIdx(u16);

impl AccessorIdx {
    /// Creates an accessor index from a validated integer.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the index as `usize` for checked slice access.
    #[must_use]
    pub fn as_usize(self) -> usize {
        usize::from(self.0)
    }
}

/// Numeric constant-pool index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ConstIdx(u16);

impl ConstIdx {
    /// Creates a constant index from a validated integer.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the index as `usize` for checked slice access.
    #[must_use]
    pub fn as_usize(self) -> usize {
        usize::from(self.0)
    }
}

/// Runtime run identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RunId(u128);

impl RunId {
    /// Creates a run identifier.
    #[must_use]
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Returns the raw integer value.
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }
}

/// Digest of source workflow or compiled IR bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
