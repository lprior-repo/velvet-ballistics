#![forbid(unsafe_code)]

use super::{KeyRange, PartitionError, ShardCount};

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

        let n_u128 = u128::from(n);
        let step_u128 = match span.checked_div(n_u128) {
            Some(s) => s,
            None => return Err(PartitionError::ArithmeticOverflow { shard_index: 0 }),
        };
        let rem_u128 = match span.checked_rem(n_u128) {
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

        let capacity = match usize::try_from(n) {
            Ok(value) => value,
            Err(_) => return Err(PartitionError::ArithmeticOverflow { shard_index: 0 }),
        };
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
        let first_start = self.ranges.first()?.start();
        let last_end = self.ranges.last()?.end();
        if key < first_start || key > last_end {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.ranges.len();
        while lo < hi {
            let width = hi.checked_sub(lo)?;
            let half = width.checked_div(2)?;
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
        if ranges.is_empty() {
            return Err("partition plan has no ranges");
        }
        for range in ranges.iter() {
            if range.start() > range.end() {
                return Err("range has start > end");
            }
        }
        for pair in ranges.windows(2) {
            let curr = match pair.first().copied() {
                Some(value) => value,
                None => return Err("partition plan pair missing current range"),
            };
            let next = match pair.get(1).copied() {
                Some(value) => value,
                None => return Err("partition plan pair missing next range"),
            };
            match curr.end().checked_add(1) {
                Some(expected) if expected == next.start() => {}
                _ => return Err("ranges are not contiguous"),
            }
        }
        for (index, left) in ranges.iter().enumerate() {
            for right in ranges.iter().skip(index.saturating_add(1)) {
                if !left.is_disjoint(*right) {
                    return Err("ranges overlap");
                }
            }
        }
        Ok(())
    }
}
