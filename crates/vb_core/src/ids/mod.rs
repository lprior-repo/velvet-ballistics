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
numeric_id!(EventSeq, u64, get);
numeric_id!(SeqNo, u64, get);

checked_index!(StepIdx);
checked_index!(SlotIdx);
checked_index!(ExprIdx);
checked_index!(AccessorIdx);
checked_index!(ConstIdx);

/// Branch index within a `Together` block.
///
/// First branch is 0, second is 1, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct BranchIdx(u16);

impl BranchIdx {
    /// Creates a branch index from a raw value.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the raw branch index value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Returns true if this is the first branch (index 0).
    #[must_use]
    pub const fn is_first(self) -> bool {
        self.0 == 0
    }
}

impl From<u16> for BranchIdx {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

/// Fanout limit for `ForEach` iteration.
///
/// Enforces the maximum number of items that can be iterated in a single
/// `ForEach` invocation. A limit of 0 means no items are allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct FanoutLimit(u32);

impl FanoutLimit {
    /// Creates a fanout limit from a raw value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw limit value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Converts to `usize` for checked comparison with collection sizes.
    ///
    /// On platforms where `usize` is at least 32 bits this always succeeds.
    /// On exotic narrower platforms the value saturates to `usize::MAX`.
    #[must_use]
    pub fn as_usize(self) -> usize {
        match usize::try_from(self.0) {
            Ok(v) => v,
            Err(_) => usize::MAX,
        }
    }
}

impl From<u32> for FanoutLimit {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

/// Maximum number of retry/repeat attempts.
///
/// Must be at least 1 - a repeat with max_attempts=0 is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct MaxAttempts(u16);

impl MaxAttempts {
    /// Creates a max attempts value, validating that it is non-zero.
    ///
    /// # Errors
    /// Returns `EngineError::InvalidRepeatState` if value is 0.
    pub fn try_new(value: u16) -> Result<Self, super::errors::EngineError> {
        if value == 0 {
            return Err(super::errors::EngineError::InternalInvariantViolation {
                reason: "max_attempts_cannot_be_zero",
            });
        }
        Ok(Self(value))
    }

    /// Returns the raw max attempts value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Current attempt counter within a retry/repeat loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RetryCount(u16);

impl RetryCount {
    /// Creates a retry count from a raw value.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the raw count value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Returns the next count value, saturating at `u16::MAX`.
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Number of branches in a `Together` block.
///
/// Unlike `BranchIdx` which is an index (0, 1, 2...), `BranchCount`
/// represents the total count of branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct BranchCount(u16);

impl BranchCount {
    /// Creates a branch count from a raw value.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the raw count value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl From<u16> for BranchCount {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

impl RunId {
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
    use super::{RunId, SeqNo, SlotIdx, StepIdx, WorkflowId};

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

    // =========================================================================
    // BLACKHAT security regression tests — IDs
    // =========================================================================

    // --- FanoutLimit::as_usize does not use unwrap_or ---

    #[test]
    fn fanout_limit_as_usize_zero() {
        use super::FanoutLimit;
        let limit = FanoutLimit::new(0);
        assert_eq!(limit.as_usize(), 0);
    }

    #[test]
    fn fanout_limit_as_usize_max_u32() {
        use super::FanoutLimit;
        let limit = FanoutLimit::new(u32::MAX);
        // On all current platforms u32 fits in usize
        assert_eq!(limit.as_usize(), u32::MAX as usize);
    }

    #[test]
    fn fanout_limit_as_usize_typical_value() {
        use super::FanoutLimit;
        let limit = FanoutLimit::new(1000);
        assert_eq!(limit.as_usize(), 1000);
    }

    // =========================================================================
    // Edge-case tests — ID types: ordering, FromStr, BranchIdx, MaxAttempts,
    // RetryCount, BranchCount, FanoutLimit, WorkflowDigest
    // =========================================================================

    // --- Ordering comparisons ---

    #[test]
    fn step_idx_ordering() {
        let a = StepIdx::new(0);
        let b = StepIdx::new(1);
        let c = StepIdx::new(u16::MAX);
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
        assert!(a <= a);
        assert!(c >= c);
    }

    #[test]
    fn slot_idx_ordering() {
        let a = SlotIdx::new(0);
        let b = SlotIdx::new(100);
        let c = SlotIdx::new(u16::MAX);
        assert!(a < b);
        assert!(b < c);
        assert!(a != c);
    }

    #[test]
    fn seq_no_ordering() {
        let a = SeqNo::new(0);
        let b = SeqNo::new(u64::MAX);
        assert!(a < b);
    }

    #[test]
    fn run_id_ordering() {
        let a = RunId::new(0);
        let b = RunId::new(u64::MAX);
        assert!(a < b);
    }

    #[test]
    fn workflow_id_ordering() {
        let a = WorkflowId::new(0);
        let b = WorkflowId::new(u32::MAX);
        assert!(a < b);
    }

    // --- FromStr parsing edge cases ---

    #[test]
    fn from_str_parses_zero() -> Result<(), String> {
        let idx: StepIdx = "0".parse().map_err(|_| String::from("parse failed"))?;
        if idx.get() != 0 {
            return Err(String::from("expected 0"));
        }
        Ok(())
    }

    #[test]
    fn from_str_parses_max_u16() -> Result<(), String> {
        let idx: SlotIdx = "65535".parse().map_err(|_| String::from("parse failed"))?;
        if idx.get() != u16::MAX {
            return Err(String::from("expected u16::MAX"));
        }
        Ok(())
    }

    #[test]
    fn from_str_parses_max_u32() -> Result<(), String> {
        let id: WorkflowId = "4294967295"
            .parse()
            .map_err(|_| String::from("parse failed"))?;
        if id.get() != u32::MAX {
            return Err(String::from("expected u32::MAX"));
        }
        Ok(())
    }

    #[test]
    fn from_str_parses_max_u64() -> Result<(), String> {
        let id: RunId = "18446744073709551615"
            .parse()
            .map_err(|_| String::from("parse failed"))?;
        if id.get() != u64::MAX {
            return Err(String::from("expected u64::MAX"));
        }
        Ok(())
    }

    #[test]
    fn from_str_rejects_empty_string() {
        let result: Result<StepIdx, _> = "".parse();
        assert!(result.is_err(), "empty string must fail to parse");
    }

    #[test]
    fn from_str_rejects_negative() {
        let result: Result<StepIdx, _> = "-1".parse();
        assert!(result.is_err(), "negative string must fail to parse");
    }

    #[test]
    fn from_str_rejects_overflow_for_u16() {
        let result: Result<StepIdx, _> = "65536".parse();
        assert!(result.is_err(), "u16 overflow must fail to parse");
    }

    #[test]
    fn from_str_rejects_overflow_for_u32() {
        let result: Result<WorkflowId, _> = "4294967296".parse();
        assert!(result.is_err(), "u32 overflow must fail to parse");
    }

    #[test]
    fn from_str_rejects_leading_whitespace() {
        let result: Result<StepIdx, _> = " 42".parse();
        assert!(result.is_err(), "leading whitespace must fail");
    }

    // --- BranchIdx edge cases ---

    #[test]
    fn branch_idx_zero_is_first() {
        use super::BranchIdx;
        let idx = BranchIdx::new(0);
        assert!(idx.is_first());
        assert_eq!(idx.get(), 0);
    }

    #[test]
    fn branch_idx_one_is_not_first() {
        use super::BranchIdx;
        let idx = BranchIdx::new(1);
        assert!(!idx.is_first());
    }

    #[test]
    fn branch_idx_max_value() {
        use super::BranchIdx;
        let idx = BranchIdx::new(u16::MAX);
        assert!(!idx.is_first());
        assert_eq!(idx.get(), u16::MAX);
    }

    #[test]
    fn branch_idx_from_u16() {
        use super::BranchIdx;
        let idx = BranchIdx::from(7u16);
        assert_eq!(idx.get(), 7);
    }

    // --- MaxAttempts edge cases ---

    #[test]
    fn max_attempts_one_is_valid() -> Result<(), String> {
        use super::MaxAttempts;
        let attempts = MaxAttempts::try_new(1).map_err(|e| e.to_string())?;
        assert_eq!(attempts.get(), 1);
        Ok(())
    }

    #[test]
    fn max_attempts_max_u16_is_valid() -> Result<(), String> {
        use super::MaxAttempts;
        let attempts = MaxAttempts::try_new(u16::MAX).map_err(|e| e.to_string())?;
        assert_eq!(attempts.get(), u16::MAX);
        Ok(())
    }

    #[test]
    fn max_attempts_zero_is_rejected() {
        use super::MaxAttempts;
        let result = MaxAttempts::try_new(0);
        assert!(result.is_err(), "max_attempts=0 must be rejected");
    }

    // --- RetryCount edge cases ---

    #[test]
    fn retry_count_zero_is_valid() {
        use super::RetryCount;
        let count = RetryCount::new(0);
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn retry_count_next_increments() {
        use super::RetryCount;
        let count = RetryCount::new(0);
        let next = count.next();
        assert_eq!(next.get(), 1);
    }

    #[test]
    fn retry_count_next_saturates_at_max() {
        use super::RetryCount;
        let count = RetryCount::new(u16::MAX);
        let next = count.next();
        assert_eq!(next.get(), u16::MAX);
    }

    #[test]
    fn retry_count_max_u16() {
        use super::RetryCount;
        let count = RetryCount::new(u16::MAX);
        assert_eq!(count.get(), u16::MAX);
    }

    // --- BranchCount edge cases ---

    #[test]
    fn branch_count_zero_is_valid() {
        use super::BranchCount;
        let count = BranchCount::new(0);
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn branch_count_max_u16() {
        use super::BranchCount;
        let count = BranchCount::new(u16::MAX);
        assert_eq!(count.get(), u16::MAX);
    }

    #[test]
    fn branch_count_from_u16() {
        use super::BranchCount;
        let count = BranchCount::from(5u16);
        assert_eq!(count.get(), 5);
    }

    // --- FanoutLimit edge cases ---

    #[test]
    fn fanout_limit_from_u32() {
        use super::FanoutLimit;
        let limit = FanoutLimit::from(100u32);
        assert_eq!(limit.get(), 100);
    }

    #[test]
    fn fanout_limit_zero_get() {
        use super::FanoutLimit;
        let limit = FanoutLimit::new(0);
        assert_eq!(limit.get(), 0);
    }

    // --- WorkflowDigest edge cases ---

    #[test]
    fn workflow_digest_equality() {
        use super::WorkflowDigest;
        let a = WorkflowDigest::from_bytes([0xFF; 32]);
        let b = WorkflowDigest::from_bytes([0xFF; 32]);
        assert_eq!(a, b);
    }

    #[test]
    fn workflow_digest_inequality() {
        use super::WorkflowDigest;
        let a = WorkflowDigest::from_bytes([0x00; 32]);
        let b = WorkflowDigest::from_bytes([0xFF; 32]);
        assert_ne!(a, b);
    }

    #[test]
    fn workflow_digest_single_byte_difference() {
        use super::WorkflowDigest;
        let mut bytes_a = [0u8; 32];
        let bytes_b = [0u8; 32];
        bytes_a[31] = 1;
        let a = WorkflowDigest::from_bytes(bytes_a);
        let b = WorkflowDigest::from_bytes(bytes_b);
        assert_ne!(a, b);
    }

    // --- AccessorIdx checked arithmetic ---

    #[test]
    fn accessor_idx_as_usize_boundary() {
        use super::AccessorIdx;
        let idx = AccessorIdx::new(0);
        assert_eq!(idx.as_usize(), 0);
        let idx_max = AccessorIdx::new(u16::MAX);
        assert_eq!(idx_max.as_usize(), usize::from(u16::MAX));
    }

    // --- ExprIdx checked arithmetic ---

    #[test]
    fn expr_idx_as_usize_boundary() {
        use super::ExprIdx;
        let idx = ExprIdx::new(0);
        assert_eq!(idx.as_usize(), 0);
        let idx_max = ExprIdx::new(u16::MAX);
        assert_eq!(idx_max.as_usize(), usize::from(u16::MAX));
    }

    // --- ConstIdx as_usize boundary ---

    #[test]
    fn const_idx_as_usize_boundary() {
        use super::ConstIdx;
        let idx = ConstIdx::new(0);
        assert_eq!(idx.as_usize(), 0);
        let idx_max = ConstIdx::new(u16::MAX);
        assert_eq!(idx_max.as_usize(), usize::from(u16::MAX));
    }

    // --- Copy and Clone for all ID types ---

    #[test]
    fn id_types_copy_trait() {
        let step = StepIdx::new(42);
        let step_copy = step;
        assert_eq!(step, step_copy);

        let slot = SlotIdx::new(7);
        let slot_copy = slot;
        assert_eq!(slot, slot_copy);

        let run = RunId::new(99);
        let run_copy = run;
        assert_eq!(run, run_copy);
    }

    // --- Hash consistency for WorkflowDigest ---

    #[test]
    fn workflow_digest_hash_consistency() {
        use super::WorkflowDigest;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let a = WorkflowDigest::from_bytes([0xAB; 32]);
        let b = WorkflowDigest::from_bytes([0xAB; 32]);

        let mut hasher_a = DefaultHasher::new();
        let mut hasher_b = DefaultHasher::new();
        a.hash(&mut hasher_a);
        b.hash(&mut hasher_b);

        assert_eq!(hasher_a.finish(), hasher_b.finish());
    }

    // --- New tests for constants, Debug, Ord, FromStr, Hash ---

    #[test]
    fn step_idx_zero_constant_is_zero() {
        assert_eq!(StepIdx::ZERO.get(), 0);
        assert_eq!(StepIdx::ZERO.as_usize(), 0);
    }

    #[test]
    fn step_idx_min_is_zero() {
        assert_eq!(StepIdx::MIN.get(), 0);
    }

    #[test]
    fn step_idx_max_is_u16_max() {
        assert_eq!(StepIdx::MAX.get(), u16::MAX);
    }

    #[test]
    fn action_id_max_u16_is_valid() {
        use super::ActionId;
        let id = ActionId::new(u16::MAX);
        assert_eq!(id.get(), u16::MAX);
    }

    #[test]
    fn expr_idx_max_u16_is_valid() {
        use super::ExprIdx;
        let idx = ExprIdx::new(u16::MAX);
        assert_eq!(idx.get(), u16::MAX);
        assert_eq!(idx.as_usize(), usize::from(u16::MAX));
    }

    #[test]
    fn const_idx_max_u16_is_valid() {
        use super::ConstIdx;
        let idx = ConstIdx::new(u16::MAX);
        assert_eq!(idx.get(), u16::MAX);
        assert_eq!(idx.as_usize(), usize::from(u16::MAX));
    }

    #[test]
    fn debug_trait_contains_inner_value() {
        use super::ExprIdx;
        let idx = ExprIdx::new(42);
        let debug = format!("{idx:?}");
        assert!(
            debug.contains("42"),
            "Debug output must contain inner value 42, got: {debug}"
        );
    }

    #[test]
    fn ord_comparison_expr_idx() {
        use super::ExprIdx;
        let a = ExprIdx::new(0);
        let b = ExprIdx::new(100);
        let c = ExprIdx::new(u16::MAX);
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
        assert!(a <= a);
        assert!(c >= c);
    }

    #[test]
    fn ord_comparison_action_id() {
        use super::ActionId;
        let a = ActionId::new(0);
        let b = ActionId::new(1);
        let c = ActionId::new(u16::MAX);
        assert!(a < b);
        assert!(b < c);
        assert!(a != c);
    }

    #[test]
    fn from_str_parses_max_u16_for_action_id() -> Result<(), String> {
        use super::ActionId;
        let id: ActionId = "65535".parse().map_err(|_| String::from("parse failed"))?;
        if id.get() != u16::MAX {
            return Err(String::from("expected u16::MAX"));
        }
        Ok(())
    }

    #[test]
    fn hash_consistency_for_equal_step_idx() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let a = StepIdx::new(42);
        let b = StepIdx::new(42);
        let mut hasher_a = DefaultHasher::new();
        let mut hasher_b = DefaultHasher::new();
        a.hash(&mut hasher_a);
        b.hash(&mut hasher_b);
        assert_eq!(hasher_a.finish(), hasher_b.finish());
    }
}

#[cfg(kani)]
pub mod kani_id_bounds;

#[cfg(kani)]
pub mod kani_id_arbitrary;

#[cfg(kani)]
pub mod kani_shard_index_bounds;
