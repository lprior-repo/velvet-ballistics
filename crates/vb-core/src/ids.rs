#![forbid(unsafe_code)]

//! Compact numeric identifiers used by the hot runtime.

use core::num::ParseIntError;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

/// Converts compact indexes to `usize` for checked table access.
pub trait CheckedIndex {
    /// Returns the raw index widened to `usize`.
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    fn as_usize(self) -> usize;
}

macro_rules! numeric_id {
    ($name:ident, $inner:ty, $accessor:ident) => {
        #[doc = concat!(stringify!($name), " numeric identifier.")]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[repr(transparent)]
        pub struct $name($inner);

        impl $name {
            /// Creates an identifier from a validated integer.
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            /// Returns the raw identifier value.
            #[must_use]
            pub const fn $accessor(self) -> $inner {
                self.0
            }
        }

        impl FromStr for $name {
            type Err = ParseIntError;

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                input.parse::<$inner>().map(Self)
            }
        }
    };
}

macro_rules! checked_index {
    ($name:ident) => {
        impl CheckedIndex for $name {
            fn as_usize(self) -> usize {
                usize::from(self.0)
            }
        }

        impl $name {
            /// Returns the index as `usize` for checked slice access.
            #[must_use]
            pub fn as_usize(self) -> usize {
                <Self as CheckedIndex>::as_usize(self)
            }
        }
    };
}

numeric_id!(WorkflowId, u32, get);
numeric_id!(StepIdx, u16, get);
numeric_id!(SlotIdx, u16, get);
numeric_id!(ExprIdx, u16, get);
numeric_id!(ActionId, u16, get);
numeric_id!(AccessorIdx, u16, get);
numeric_id!(ConstIdx, u16, get);
numeric_id!(SymbolId, u32, get);
numeric_id!(ListId, u32, get);
numeric_id!(ObjectId, u32, get);
numeric_id!(BlobId, u64, as_u64);
numeric_id!(RunId, u64, as_u64);
numeric_id!(SeqNo, u64, as_u64);

checked_index!(StepIdx);
checked_index!(SlotIdx);
checked_index!(ExprIdx);
checked_index!(AccessorIdx);
checked_index!(ConstIdx);

impl RunId {
    /// Zero run identifier.
    pub const ZERO: Self = Self(0);
}

impl SeqNo {
    /// Zero sequence number.
    pub const ZERO: Self = Self(0);
    /// Minimum sequence number.
    pub const MIN: Self = Self(0);
    /// Maximum sequence number.
    pub const MAX: Self = Self(u64::MAX);

    /// Adds without overflow.
    #[must_use]
    pub const fn checked_add(self, rhs: u64) -> Option<Self> {
        match self.0.checked_add(rhs) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl StepIdx {
    /// Zero step index.
    pub const ZERO: Self = Self(0);
    /// Minimum step index.
    pub const MIN: Self = Self(0);
    /// Maximum step index.
    pub const MAX: Self = Self(u16::MAX);

    /// Adds without overflow.
    #[must_use]
    pub const fn checked_add(self, rhs: u16) -> Option<Self> {
        match self.0.checked_add(rhs) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl SlotIdx {
    /// Zero slot index.
    pub const ZERO: Self = Self(0);
    /// Minimum slot index.
    pub const MIN: Self = Self(0);
    /// Maximum slot index.
    pub const MAX: Self = Self(u16::MAX);

    /// Adds without overflow.
    #[must_use]
    pub const fn checked_add(self, rhs: u16) -> Option<Self> {
        match self.0.checked_add(rhs) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl ConstIdx {
    /// Adds without overflow.
    #[must_use]
    pub const fn checked_add(self, rhs: u16) -> Option<Self> {
        match self.0.checked_add(rhs) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

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
