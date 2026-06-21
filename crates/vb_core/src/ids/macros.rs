//! Shared macro definitions for the IDs submodules.
//!
//! Centralises `numeric_id!` and `checked_index!` so that all four ID
//! submodules (`workflow_ids`, `index_ids`, `storage_ids`, `symbol_ids`)
//! share a single source of truth.

/// Declares a transparent newtype wrapping a numeric primitive and
/// implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`,
/// `PartialOrd`, `Ord`, `Hash`, `Serialize`, `Deserialize`,
/// `new`, the accessor getter, and `FromStr`.
#[macro_export]
macro_rules! numeric_id {
    ($name:ident, $inner:ty, $accessor:ident) => {
        #[doc = concat!(stringify!($name), " numeric identifier.")]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
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

        impl core::str::FromStr for $name {
            type Err = core::num::ParseIntError;

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
#[macro_export]
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
