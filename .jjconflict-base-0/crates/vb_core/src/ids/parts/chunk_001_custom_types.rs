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
    /// Returns `EngineError::InvalidRepeatState` with reason
    /// `"max_attempts_cannot_be_zero"` if value is 0.
    pub fn try_new(value: u16) -> Result<Self, super::errors::EngineError> {
        if value == 0 {
            return Err(super::errors::EngineError::InvalidRepeatState {
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
