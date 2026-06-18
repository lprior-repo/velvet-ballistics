//! Index identifiers for compiled IR: `ExprIdx`, `AccessorIdx`, and `ConstIdx`.
//!
//! These wrap `u16` and provide checked `as_usize` conversion for safe
//! slice access in the hot path.

#![forbid(unsafe_code)]

use core::num::ParseIntError;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

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
        impl $name {
            /// Returns the index as `usize` for checked slice access.
            #[must_use]
            pub fn as_usize(self) -> usize {
                usize::from(self.0)
            }
        }
    };
}

numeric_id!(ExprIdx, u16, get);
numeric_id!(AccessorIdx, u16, get);
numeric_id!(ConstIdx, u16, get);

checked_index!(ExprIdx);
checked_index!(AccessorIdx);
checked_index!(ConstIdx);

// ── ConstIdx additional methods ────────────────────────────────────────

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
