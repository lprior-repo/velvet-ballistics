//! Symbol identifiers used by the runtime symbol table: `SymbolId`, `ListId`,
//! and `ObjectId`.

#![forbid(unsafe_code)]

use core::num::ParseIntError;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

/// Symbol identifier — wraps `u32`.
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

numeric_id!(SymbolId, u32, get);
numeric_id!(ListId, u32, get);
numeric_id!(ObjectId, u32, get);
