#![forbid(unsafe_code)]

//! Compact numeric identifiers used by the hot runtime.
//!
//! This module exposes the numeric identifier types used by the runtime:
//!
//! - Generic numeric IDs produced by the `numeric_id!` macro and its
//!   `checked_index!` extension (WorkflowId, StepIdx, SlotIdx, ExprIdx,
//!   ActionId, AccessorIdx, ConstIdx, SymbolId, ListId, ObjectId, BlobId,
//!   RunId, EventSeq, SeqNo).
//! - Hand-written domain identifiers (BranchIdx, FanoutLimit, MaxAttempts,
//!   RetryCount, BranchCount) and the `WorkflowDigest` content hash.
//! - Kani harnesses for ID boundary and proof obligations
//!   (`kani_id_bounds`, `kani_id_arbitrary`, `kani_shard_index_bounds`).
//!
//! The hand-written domain types live in `parts/chunk_001_custom_types.rs`
//! and are `include!`-d into this module to keep the public surface and
//! macro/derived-type definitions in a single, scannable file.

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
numeric_id!(BlobId, u64, get);
numeric_id!(RunId, u64, get);
numeric_id!(EventSeq, u64, get);
numeric_id!(SeqNo, u64, get);

checked_index!(StepIdx);
checked_index!(SlotIdx);
checked_index!(ExprIdx);
checked_index!(AccessorIdx);
checked_index!(ConstIdx);

include!("parts/chunk_001_custom_types.rs");

#[cfg(test)]
#[path = "tests_and_verification.rs"]
mod tests_and_verification;

#[cfg(kani)]
pub mod kani_id_bounds;

#[cfg(kani)]
pub mod kani_id_arbitrary;

#[cfg(kani)]
pub mod kani_shard_index_bounds;
