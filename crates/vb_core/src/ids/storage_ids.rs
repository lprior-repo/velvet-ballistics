//! Storage identifiers: `BlobId` and `ActionId`.

#![forbid(unsafe_code)]

use core::num::ParseIntError;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

/// Blob identifier — wraps `u64`.
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

numeric_id!(BlobId, u64, get);
numeric_id!(ActionId, u16, get);

// ── BlobId additional methods ──────────────────────────────────────────

impl BlobId {
    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}
