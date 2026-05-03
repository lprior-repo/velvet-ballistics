#![forbid(unsafe_code)]

//! Compact numeric identifiers used by the hot runtime.

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
numeric_id!(SeqNo, u64, get);

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

impl WorkflowId {
    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl BlobId {
    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl RunId {
    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl SeqNo {
    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    pub fn as_u64(self) -> u64 {
        self.0
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

#[cfg(test)]
mod tests {
    use super::{RunId, SlotIdx, StepIdx, WorkflowId};

    #[test]
    fn workflow_id_get_returns_inner_value() {
        let id = WorkflowId::new(42);
        assert_eq!(id.get(), 42);
    }

    #[test]
    fn run_id_get_returns_inner_value() {
        let id = RunId::new(12345);
        assert_eq!(id.get(), 12345);
    }

    #[test]
    fn step_idx_as_usize_returns_inner_value() {
        let idx = StepIdx::new(7);
        assert_eq!(idx.as_usize(), 7);
    }

    #[test]
    fn slot_idx_as_usize_returns_inner_value() {
        let idx = SlotIdx::new(15);
        assert_eq!(idx.as_usize(), 15);
    }

    // =========================================================================
    // Adversarial BDD tests — ID boundary and overflow edge cases
    // =========================================================================

    #[test]
    fn step_idx_zero_is_valid() {
        let idx = StepIdx::new(0);
        assert_eq!(idx.get(), 0);
        assert_eq!(idx.as_usize(), 0);
    }

    #[test]
    fn step_idx_max_u16_is_valid() {
        let idx = StepIdx::new(u16::MAX);
        assert_eq!(idx.get(), u16::MAX);
    }

    #[test]
    fn step_idx_checked_add_overflow_returns_none() {
        let idx = StepIdx::new(u16::MAX);
        assert_eq!(idx.checked_add(1), None);
    }

    #[test]
    fn step_idx_checked_add_zero_is_identity() {
        let idx = StepIdx::new(100);
        assert_eq!(idx.checked_add(0), Some(StepIdx::new(100)));
    }

    #[test]
    fn step_idx_checked_add_exact_max_saturates() {
        let idx = StepIdx::new(0);
        assert_eq!(idx.checked_add(u16::MAX), Some(StepIdx::new(u16::MAX)));
    }

    #[test]
    fn slot_idx_zero_is_valid() {
        let idx = SlotIdx::new(0);
        assert_eq!(idx.get(), 0);
        assert_eq!(idx.as_usize(), 0);
    }

    #[test]
    fn slot_idx_max_u16_is_valid() {
        let idx = SlotIdx::new(u16::MAX);
        assert_eq!(idx.get(), u16::MAX);
    }

    #[test]
    fn slot_idx_checked_add_overflow_returns_none() {
        let idx = SlotIdx::new(u16::MAX);
        assert_eq!(idx.checked_add(1), None);
    }

    #[test]
    fn slot_idx_checked_add_exact_max() {
        let idx = SlotIdx::new(0);
        assert_eq!(idx.checked_add(u16::MAX), Some(SlotIdx::new(u16::MAX)));
    }

    #[test]
    fn slot_idx_min_is_zero() {
        assert_eq!(SlotIdx::MIN.get(), 0);
    }

    #[test]
    fn slot_idx_max_is_u16_max() {
        assert_eq!(SlotIdx::MAX.get(), u16::MAX);
    }

    #[test]
    fn slot_idx_zero_constant_is_zero() {
        assert_eq!(SlotIdx::ZERO.get(), 0);
    }

    #[test]
    fn const_idx_checked_add_overflow_returns_none() {
        use super::ConstIdx;
        let idx = ConstIdx::new(u16::MAX);
        assert_eq!(idx.checked_add(1), None);
    }

    #[test]
    fn const_idx_checked_add_success() {
        use super::ConstIdx;
        let idx = ConstIdx::new(10);
        assert_eq!(idx.checked_add(5), Some(ConstIdx::new(15)));
    }

    #[test]
    fn seq_no_zero_is_valid() {
        use super::SeqNo;
        assert_eq!(SeqNo::ZERO.get(), 0);
    }

    #[test]
    fn seq_no_min_is_zero() {
        use super::SeqNo;
        assert_eq!(SeqNo::MIN.get(), 0);
    }

    #[test]
    fn seq_no_max_is_u64_max() {
        use super::SeqNo;
        assert_eq!(SeqNo::MAX.get(), u64::MAX);
    }

    #[test]
    fn seq_no_checked_add_overflow_returns_none() {
        use super::SeqNo;
        let seq = SeqNo::new(u64::MAX);
        assert_eq!(seq.checked_add(1), None);
    }

    #[test]
    fn seq_no_checked_add_exact_max() {
        use super::SeqNo;
        let seq = SeqNo::new(0);
        assert_eq!(seq.checked_add(u64::MAX), Some(SeqNo::new(u64::MAX)));
    }

    #[test]
    fn run_id_zero_constant() {
        assert_eq!(RunId::ZERO.get(), 0);
    }

    #[test]
    fn run_id_max_u64() {
        let id = RunId::new(u64::MAX);
        assert_eq!(id.get(), u64::MAX);
    }

    #[test]
    fn symbol_id_zero_is_valid() {
        use super::SymbolId;
        let id = SymbolId::new(0);
        assert_eq!(id.get(), 0);
    }

    #[test]
    fn symbol_id_max_u32_is_valid() {
        use super::SymbolId;
        let id = SymbolId::new(u32::MAX);
        assert_eq!(id.get(), u32::MAX);
    }

    #[test]
    fn list_id_max_u32_is_valid() {
        use super::ListId;
        let id = ListId::new(u32::MAX);
        assert_eq!(id.get(), u32::MAX);
    }

    #[test]
    fn object_id_max_u32_is_valid() {
        use super::ObjectId;
        let id = ObjectId::new(u32::MAX);
        assert_eq!(id.get(), u32::MAX);
    }

    #[test]
    fn blob_id_max_u64_is_valid() {
        use super::BlobId;
        let id = BlobId::new(u64::MAX);
        assert_eq!(id.get(), u64::MAX);
    }

    #[test]
    fn workflow_id_zero_is_valid() {
        let id = WorkflowId::new(0);
        assert_eq!(id.get(), 0);
    }

    #[test]
    fn workflow_id_max_u32() {
        let id = WorkflowId::new(u32::MAX);
        assert_eq!(id.get(), u32::MAX);
    }

    #[test]
    fn action_id_zero_is_valid() {
        use super::ActionId;
        let id = ActionId::new(0);
        assert_eq!(id.get(), 0);
    }

    #[test]
    fn accessor_idx_as_usize() {
        use super::AccessorIdx;
        let idx = AccessorIdx::new(42);
        assert_eq!(idx.as_usize(), 42);
    }

    #[test]
    fn expr_idx_as_usize() {
        use super::ExprIdx;
        let idx = ExprIdx::new(13);
        assert_eq!(idx.as_usize(), 13);
    }

    #[test]
    fn ids_from_str_valid() -> Result<(), String> {
        let step: StepIdx = "42".parse().map_err(|_| String::from("parse failed"))?;
        if step.get() != 42 {
            return Err(String::from("expected 42"));
        }
        Ok(())
    }

    #[test]
    fn ids_from_str_invalid() {
        use super::SymbolId;
        let result: Result<SymbolId, _> = "not_a_number".parse();
        assert!(result.is_err(), "non-numeric string must fail to parse");
    }

    #[test]
    fn workflow_digest_roundtrip() {
        use super::WorkflowDigest;
        let bytes = [0xAB_u8; 32];
        let digest = WorkflowDigest::from_bytes(bytes);
        assert_eq!(digest.as_bytes(), bytes);
    }

    #[test]
    fn workflow_digest_zero_array() {
        use super::WorkflowDigest;
        let digest = WorkflowDigest::from_bytes([0u8; 32]);
        assert_eq!(digest.as_bytes(), [0u8; 32]);
    }
}
