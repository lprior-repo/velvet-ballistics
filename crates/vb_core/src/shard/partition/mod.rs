//! Shard Partition Math — Verification Model Types
//!
//! **TRUST BOUNDARY**: These types are verification models. They are NOT production
//! types. The production implementation (to be created in State 6/7) must match the
//! mathematical contracts validated by the proof harnesses against this model.
//!
//! # Model Commitments
//! - All arithmetic uses checked operations; no panics
//! - KeyRange invariant: start <= end (enforced by constructor)
//! - ShardCount invariant: 1 <= inner <= MAX_SHARD_COUNT
//! - PartitionPlan invariants: contiguous, disjoint, exhaustive ranges
//! - No unsafe, no unwrap, no expect, no panic

#![forbid(unsafe_code)]

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of shards for a single-server deployment.
pub const MAX_SHARD_COUNT: usize = 65_536;

/// Maximum shard count for Kani bounded verification (TB-005).
pub const KANI_MAX_SHARD_COUNT: usize = 32;

// ============================================================================
// PartitionError
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionError {
    ZeroShardCount,
    ShardCountExceedsMax { requested: u64, maximum: u64 },
    InvalidKeyRange { start: u64, end: u64 },
    ArithmeticOverflow { shard_index: u64 },
}

impl core::fmt::Display for PartitionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroShardCount => write!(f, "shard count must be at least 1"),
            Self::ShardCountExceedsMax { requested, maximum } => {
                write!(f, "shard count {requested} exceeds maximum {maximum}")
            }
            Self::InvalidKeyRange { start, end } => {
                write!(f, "invalid key range: start {start} > end {end}")
            }
            Self::ArithmeticOverflow { shard_index } => {
                write!(f, "arithmetic overflow at shard {shard_index}")
            }
        }
    }
}

// ============================================================================
// KeyRange
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyRange {
    start: u64,
    end: u64,
}

impl KeyRange {
    pub fn new(start: u64, end: u64) -> Result<Self, PartitionError> {
        if start > end {
            return Err(PartitionError::InvalidKeyRange { start, end });
        }
        Ok(Self { start, end })
    }

    #[must_use]
    pub const fn from_single_key(key: u64) -> Self {
        Self {
            start: key,
            end: key,
        }
    }
    #[must_use]
    pub const fn full_keyspace() -> Self {
        Self {
            start: 0,
            end: u64::MAX,
        }
    }
    #[must_use]
    pub const fn start(self) -> u64 {
        self.start
    }
    #[must_use]
    pub const fn end(self) -> u64 {
        self.end
    }
    #[must_use]
    pub const fn contains(self, key: u64) -> bool {
        self.start <= key && key <= self.end
    }
    #[must_use]
    pub const fn size(self) -> u64 {
        self.end.wrapping_sub(self.start)
    }
    #[must_use]
    pub fn count(self) -> Option<u64> {
        self.end.checked_sub(self.start)?.checked_add(1)
    }
    #[must_use]
    pub const fn is_singleton(self) -> bool {
        self.start == self.end
    }

    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let start = if self.start > other.start {
            self.start
        } else {
            other.start
        };
        let end = if self.end < other.end {
            self.end
        } else {
            other.end
        };
        if start <= end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    #[must_use]
    pub fn is_disjoint(self, other: Self) -> bool {
        self.intersection(other).is_none()
    }

    #[must_use]
    pub fn is_adjacent_to(self, other: Self) -> bool {
        if self
            .end
            .checked_add(1)
            .is_some_and(|next| next == other.start)
        {
            return true;
        }
        if other
            .end
            .checked_add(1)
            .is_some_and(|next| next == self.start)
        {
            return true;
        }
        false
    }
}

// ============================================================================
// ShardCount
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShardCount(pub usize);

impl ShardCount {
    pub fn try_new(raw: usize) -> Result<Self, PartitionError> {
        if raw == 0 {
            return Err(PartitionError::ZeroShardCount);
        }
        #[allow(clippy::as_conversions)]
        if raw > MAX_SHARD_COUNT {
            return Err(PartitionError::ShardCountExceedsMax {
                requested: raw as u64,
                maximum: MAX_SHARD_COUNT as u64,
            });
        }
        Ok(Self(raw))
    }

    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
    #[must_use]
    #[allow(clippy::as_conversions)]
    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }
    #[must_use]
    pub const fn is_single_shard(self) -> bool {
        self.0 == 1
    }
}

impl Default for ShardCount {
    fn default() -> Self {
        Self(1)
    }
}

// ============================================================================
// PartitionConfig
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionConfig {
    pub shard_count: ShardCount,
    pub keyspace: KeyRange,
}

impl PartitionConfig {
    #[must_use]
    pub const fn new(shard_count: ShardCount, keyspace: KeyRange) -> Self {
        Self {
            shard_count,
            keyspace,
        }
    }
}

impl Default for PartitionConfig {
    fn default() -> Self {
        Self {
            shard_count: ShardCount::default(),
            keyspace: KeyRange::full_keyspace(),
        }
    }
}

// ============================================================================
// PartitionPlan
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionPlan {
    pub ranges: Box<[KeyRange]>,
}

impl PartitionPlan {
    /// Compute partition plan from validated config using cursor-based
    /// accumulation with checked arithmetic. Uses u128 for span calculation
    /// to handle the full [0, u64::MAX] keyspace.
    pub fn from_config(config: &PartitionConfig) -> Result<Self, PartitionError> {
        let n = config.shard_count.as_u64();
        let kmin = config.keyspace.start();
        let kmax = config.keyspace.end();

        if n == 0 {
            return Err(PartitionError::ZeroShardCount);
        }
        if n == 1 {
            let range = KeyRange::new(kmin, kmax)?;
            return Ok(Self {
                ranges: Box::new([range]),
            });
        }

        // Compute span using u128 to avoid overflow for full keyspace
        let span = match u128::from(kmax)
            .checked_sub(u128::from(kmin))
            .and_then(|s| s.checked_add(1))
        {
            Some(s) => s,
            None => {
                return Err(PartitionError::InvalidKeyRange {
                    start: kmin,
                    end: kmax,
                });
            }
        };

        let step_u128 = match span.checked_div(u128::from(n)) {
            Some(s) => s,
            None => return Err(PartitionError::ArithmeticOverflow { shard_index: 0 }),
        };
        let rem_u128 = match span.checked_rem(u128::from(n)) {
            Some(r) => r,
            None => return Err(PartitionError::ArithmeticOverflow { shard_index: 0 }),
        };

        let step = match u64::try_from(step_u128) {
            Ok(s) => s,
            Err(_) => return Err(PartitionError::ArithmeticOverflow { shard_index: 0 }),
        };
        let rem = match u64::try_from(rem_u128) {
            Ok(r) => r,
            Err(_) => return Err(PartitionError::ArithmeticOverflow { shard_index: 0 }),
        };

        #[allow(clippy::as_conversions)]
        let capacity = n as usize;
        let mut ranges = Vec::with_capacity(capacity);
        let mut cursor = kmin;

        let n_minus_1 = match n.checked_sub(1) {
            Some(v) => v,
            None => return Err(PartitionError::ArithmeticOverflow { shard_index: 0 }),
        };
        for i in 0..n_minus_1 {
            let extra = if i < rem { 1u64 } else { 0u64 };
            let size = match step.checked_add(extra) {
                Some(s) => s,
                None => return Err(PartitionError::ArithmeticOverflow { shard_index: i }),
            };
            let end = match cursor.checked_add(size).and_then(|s| s.checked_sub(1)) {
                Some(e) => e,
                None => return Err(PartitionError::ArithmeticOverflow { shard_index: i }),
            };
            if end > kmax {
                return Err(PartitionError::ArithmeticOverflow { shard_index: i });
            }
            let range = KeyRange::new(cursor, end)?;
            ranges.push(range);
            cursor = match end.checked_add(1) {
                Some(c) => c,
                None => return Err(PartitionError::ArithmeticOverflow { shard_index: i }),
            };
        }

        let final_range = KeyRange::new(cursor, kmax)?;
        ranges.push(final_range);

        Ok(Self {
            ranges: ranges.into_boxed_slice(),
        })
    }

    #[must_use]
    pub fn shard_count(&self) -> usize {
        self.ranges.len()
    }
    #[must_use]
    pub fn range_for(&self, shard: usize) -> Option<&KeyRange> {
        self.ranges.get(shard)
    }
    #[must_use]
    pub fn ranges(&self) -> &[KeyRange] {
        &self.ranges
    }

    #[must_use]
    pub fn shard_for_key(&self, key: u64) -> Option<usize> {
        let first = self.ranges.first()?;
        let last = self.ranges.last()?;
        let first_start = first.start();
        let last_end = last.end();
        if key < first_start || key > last_end {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.ranges.len();
        while lo < hi {
            let diff = hi.checked_sub(lo)?;
            let half = diff.checked_div(2)?;
            let mid = lo.checked_add(half)?;
            let range = self.ranges.get(mid)?;
            if key < range.start() {
                hi = mid;
            } else if key > range.end() {
                lo = mid.checked_add(1)?;
            } else {
                return Some(mid);
            }
        }
        None
    }

    pub fn validate_invariants(&self) -> Result<(), &'static str> {
        let ranges = &self.ranges;
        let n = ranges.len();
        if n == 0 {
            return Err("partition plan has no ranges");
        }
        for _r in ranges.iter() {
            if _r.start() > _r.end() {
                return Err("range has start > end");
            }
        }
        for i in 0..n.saturating_sub(1) {
            let Some(curr) = ranges.get(i).copied() else {
                return Err("range index out of bounds");
            };
            let Some(next_idx) = i.checked_add(1) else {
                return Err("arithmetic overflow computing next index");
            };
            let Some(next) = ranges.get(next_idx).copied() else {
                return Err("range index out of bounds");
            };
            match curr.end().checked_add(1) {
                Some(expected) if expected == next.start() => {}
                _ => return Err("ranges are not contiguous"),
            }
        }
        for i in 0..n {
            let Some(a) = ranges.get(i).copied() else {
                return Err("range index out of bounds");
            };
            let Some(start) = i.checked_add(1) else {
                return Err("arithmetic overflow computing next index");
            };
            for j in start..n {
                let Some(b) = ranges.get(j).copied() else {
                    return Err("range index out of bounds");
                };
                if !a.is_disjoint(b) {
                    return Err("ranges overlap");
                }
            }
        }
        Ok(())
    }
}

// ============================================================================
// Kani Arbitrary (GOD RULE #1)
// ============================================================================

#[cfg(kani)]
mod kani_arbitrary {
    use super::*;

    impl kani::Arbitrary for KeyRange {
        fn any() -> Self {
            let start: u64 = kani::any();
            let end: u64 = kani::any();
            kani::assume(start <= end);
            Self { start, end }
        }
    }

    impl kani::Arbitrary for ShardCount {
        fn any() -> Self {
            let raw: usize = kani::any();
            kani::assume(raw >= 1 && raw <= KANI_MAX_SHARD_COUNT);
            Self(raw)
        }
    }

    impl kani::Arbitrary for PartitionConfig {
        fn any() -> Self {
            let shard_count: ShardCount = kani::any();
            let keyspace: KeyRange = kani::any();
            Self {
                shard_count,
                keyspace,
            }
        }
    }
}

// ============================================================================
// Proptest strategies
// ============================================================================

#[cfg(test)]
#[allow(unused_imports)]
pub mod proptest_strategies {
    use super::*;
    use proptest::prelude::*;
    use proptest::strategy::Strategy;

    pub fn any_u64() -> impl Strategy<Value = u64> {
        proptest::num::u64::ANY
    }

    pub fn key_range_strategy() -> impl Strategy<Value = KeyRange> {
        (any_u64(), any_u64()).prop_map(|(a, b)| {
            let (start, end) = if a <= b { (a, b) } else { (b, a) };
            match KeyRange::new(start, end) {
                Ok(kr) => kr,
                Err(_) => KeyRange::full_keyspace(),
            }
        })
    }

    pub fn shard_count_strategy() -> impl Strategy<Value = ShardCount> {
        (1usize..=MAX_SHARD_COUNT).prop_map(ShardCount)
    }

    pub fn partition_config_strategy() -> impl Strategy<Value = PartitionConfig> {
        (shard_count_strategy(), key_range_strategy())
            .prop_map(|(sc, kr)| PartitionConfig::new(sc, kr))
    }
}

// ============================================================================
// Kani harnesses
// ============================================================================

#[cfg(kani)]
pub mod kani_key_range_properties;

#[cfg(kani)]
pub mod kani_partition_plan_safety;
