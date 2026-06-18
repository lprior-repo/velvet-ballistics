//! Workflow identifiers: `WorkflowId`, `RunId`, `StepIdx`, `SlotIdx`,
//! `EventSeq`, and `SeqNo`.
//!
//! These are the core identifiers that flow through the hot runtime path.

#![forbid(unsafe_code)]

use core::num::ParseIntError;
use core::str::FromStr;
use core::fmt;
use serde::{Deserialize, Serialize};

// ── Macro definitions ──────────────────────────────────────────────────

/// Declares a transparent newtype wrapping a numeric primitive and
/// implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`,
/// `PartialOrd`, `Ord`, `Hash`, `Serialize`, `Deserialize`,
/// `new`, the accessor getter, and `FromStr`.
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

/// Adds an `as_usize` method to an index newtype.
///
/// Uses `usize::from(self.0)` which is infallible because
/// `usize` is at least as wide as the wrapped `u16`.
macro_rules! checked_index {
    ($name:ident) => {
        impl $name {
            /// Returns the index as `usize` for checked slice access.
            #[must_use]
            pub fn as_usize(self) -> usize {
                usize::from(self.0)
            }
        }
    };
}

// ── Type declarations ──────────────────────────────────────────────────

numeric_id!(WorkflowId, u32, get);
numeric_id!(StepIdx, u16, get);
numeric_id!(SlotIdx, u16, get);
numeric_id!(EventSeq, u64, get);
numeric_id!(SeqNo, u64, get);

checked_index!(StepIdx);
checked_index!(SlotIdx);

// ── RunId ──────────────────────────────────────────────────────────────

/// Unique identifier for a single workflow execution run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RunId(u64);

impl RunId {
    /// Creates a run identifier from a raw value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw run identifier value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Zero run identifier.
    pub const ZERO: Self = Self(0);

    /// Returns the shard index for this run.
    ///
    /// Uses `checked_rem` to handle the degenerate case where
    /// `shard_count` is 0, returning 0 in that case.
    #[must_use]
    pub const fn shard_index(self, shard_count: u64) -> u64 {
        match self.0.checked_rem(shard_count) {
            Some(index) => index,
            None => 0,
        }
    }

    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl FromStr for RunId {
    type Err = ParseIntError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        input.parse::<u64>().map(Self)
    }
}

// ── StepIdx ────────────────────────────────────────────────────────────

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

impl fmt::Display for StepIdx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── SlotIdx ────────────────────────────────────────────────────────────

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

// ── SeqNo ──────────────────────────────────────────────────────────────

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

    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

// ── WorkflowId ─────────────────────────────────────────────────────────

impl WorkflowId {
    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    pub fn as_u32(self) -> u32 {
        self.0
    }
}
