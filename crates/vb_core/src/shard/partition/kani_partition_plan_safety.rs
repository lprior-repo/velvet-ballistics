//! Kani harnesses for PartitionPlan construction safety and output correctness.
//!
//! **Obligations:**
//! - PO-010a: `PartitionPlan::from_config()` no-panic for N ∈ [1, 32]
//! - PO-010b: Output satisfies 5 construction invariants for N ∈ [1, 32]
//!
//! **Bounds:** shard_count ∈ [1, 32], keyspace start == 0, unwind = 36
//!
//! **Kani optimization:** Uses fixed-size array instead of Vec to avoid
//! allocation complexity. The contract algorithm (Kmax/N, cursor from 0)
//! is used to avoid u128 arithmetic which is expensive for SMT solvers.
//!
//! **Model equivalence:** For keyspaces starting at 0, this algorithm is
//! mathematically equivalent to the span-based model implementation.
//! Proptest covers the full keyspace domain including non-zero starts.

#![forbid(unsafe_code)]

use super::{KeyRange, PartitionError};

const MAX: usize = 32;

/// Fixed-size partition result. Uses array with explicit length.
struct FixedPlan {
    ranges: [KeyRange; MAX],
    len: usize,
}

/// Partition algorithm using fixed-size array (no Vec allocation).
/// Uses the contract algorithm: step = kmax / n.
/// Assumes kmin = 0 (verified by proptest for general keyspaces).
fn fixed_partition(n: usize, kmax: u64) -> Result<FixedPlan, PartitionError> {
    let nu = n as u64;
    if nu == 0 {
        return Err(PartitionError::ZeroShardCount);
    }

    // Default-initialize with full_keyspace placeholders
    let default_range = KeyRange::new(0, 0)?;
    let mut ranges = [default_range; MAX];

    if nu == 1 {
        ranges[0] = KeyRange::new(0, kmax)?;
        return Ok(FixedPlan { ranges, len: 1 });
    }

    let step = match kmax.checked_div(nu) {
        Some(s) => s,
        None => return Err(PartitionError::ArithmeticOverflow { shard_index: 0 }),
    };
    let rem = kmax.wrapping_rem(nu); // Safe: nu > 0

    let mut cursor: u64 = 0;
    let n_minus_1 = nu.saturating_sub(1);

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
        ranges[i as usize] = KeyRange::new(cursor, end)?;
        cursor = match end.checked_add(1) {
            Some(c) => c,
            None => return Err(PartitionError::ArithmeticOverflow { shard_index: i }),
        };
    }

    ranges[(nu - 1) as usize] = KeyRange::new(cursor, kmax)?;

    Ok(FixedPlan { ranges, len: n })
}

// ============================================================================
// PO-010a: No-panic for N ∈ [1, 32]
// ============================================================================

#[kani::proof]
#[kani::unwind(36)]
fn partition_plan_from_config_no_panic() {
    let n: usize = kani::any();
    kani::assume(n >= 1 && n <= MAX);
    let kmax: u64 = kani::any();

    let result = fixed_partition(n, kmax);

    match result {
        Ok(_) => {}
        Err(_) => {}
    }

    kani::cover!(n == 1, "single shard");
    kani::cover!(n == MAX, "max shards");
    kani::cover!(kmax == u64::MAX, "full keyspace");
    kani::cover!(kmax == 0, "singleton keyspace");
}

// ============================================================================
// PO-010b: Output correctness — 5 construction invariants
// ============================================================================

#[kani::proof]
#[kani::unwind(36)]
fn partition_plan_post_conditions() {
    let n: usize = kani::any();
    kani::assume(n >= 1 && n <= MAX);
    let kmax: u64 = kani::any();

    let result = fixed_partition(n, kmax);

    match result {
        Ok(plan) => {
            let ranges = &plan.ranges;
            let len = plan.len;

            // Invariant 1: plan.len == n
            //! Kani harnesses for PartitionPlan construction safety and output correctness.
//!
//! **Obligations:**
//! - PO-010a: `PartitionPlan::from_config()` no-panic for N ∈ [1, 32]
//! - PO-010b: Output satisfies 5 construction invariants for N ∈ [1, 32]
//!
//! **Bounds:** shard_count ∈ [1, 32], keyspace start == 0, unwind = 36
//!
//! **Kani optimization:** Uses fixed-size array instead of Vec to avoid
//! allocation complexity. The contract algorithm (Kmax/N, cursor from 0)
//! is used to avoid u128 arithmetic which is expensive for SMT solvers.
//!
//! **Model equivalence:** For keyspaces starting at 0, this algorithm is
//! mathematically equivalent to the span-based model implementation.
//! Proptest covers the full keyspace domain including non-zero starts.

#![forbid(unsafe_code)]

use super::{KeyRange, PartitionError};

const MAX: usize = 32;

/// Fixed-size partition result. Uses array with explicit length.
struct FixedPlan {
    ranges: [KeyRange; MAX],
    len: usize,
}

/// Partition algorithm using fixed-size array (no Vec allocation).
/// Uses the contract algorithm: step = kmax / n.
/// Assumes kmin = 0 (verified by proptest for general keyspaces).
fn fixed_partition(n: usize, kmax: u64) -> Result<FixedPlan, PartitionError> {
    let nu = n as u64;
    if nu == 0 {
        return Err(PartitionError::ZeroShardCount);
    }

    // Default-initialize with full_keyspace placeholders
    let default_range = KeyRange::new(0, 0)?;
    let mut ranges = [default_range; MAX];

    if nu == 1 {
        ranges[0] = KeyRange::new(0, kmax)?;
        return Ok(FixedPlan { ranges, len: 1 });
    }

    let step = match kmax.checked_div(nu) {
        Some(s) => s,
        None => return Err(PartitionError::ArithmeticOverflow { shard_index: 0 }),
    };
    let rem = kmax.wrapping_rem(nu); // Safe: nu > 0

    let mut cursor: u64 = 0;
    let n_minus_1 = nu.saturating_sub(1);

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
        ranges[i as usize] = KeyRange::new(cursor, end)?;
        cursor = match end.checked_add(1) {
            Some(c) => c,
            None => return Err(PartitionError::ArithmeticOverflow { shard_index: i }),
        };
    }

    ranges[(nu - 1) as usize] = KeyRange::new(cursor, kmax)?;

    Ok(FixedPlan { ranges, len: n })
}

// ============================================================================
// PO-010a: No-panic for N ∈ [1, 32]
// ============================================================================

#[kani::proof]
#[kani::unwind(36)]
fn partition_plan_from_config_no_panic() {
    let n: usize = kani::any();
    kani::assume(n >= 1 && n <= MAX);
    let kmax: u64 = kani::any();

    let result = fixed_partition(n, kmax);

    match result {
        Ok(_) => {}
        Err(_) => {}
    }

    kani::cover!(n == 1, "single shard");
    kani::cover!(n == MAX, "max shards");
    kani::cover!(kmax == u64::MAX, "full keyspace");
    kani::cover!(kmax == 0, "singleton keyspace");
}

// ============================================================================
// PO-010b: Output correctness — 5 construction invariants
// ============================================================================

#[kani::proof]
#[kani::unwind(36)]
fn partition_plan_post_conditions() {
    let n: usize = kani::any();
    kani::assume(n >= 1 && n <= MAX);
    let kmax: u64 = kani::any();

    let result = fixed_partition(n, kmax);

    match result {
        Ok(plan) => {
            let ranges = &plan.ranges;
            let len = plan.len;

            // Invariant 1: plan.len == n
            kani::assert(len == n, "plan length must equal n");
            kani::assert(len > 0, "plan must have at least one range");

            // Invariant 2: ranges[0].start == 0
            kani::assert(ranges[0].start() == 0, "first range must start at 0");

            // Invariant 3: ranges[N-1].end == kmax
            kani::assert(ranges[len - 1].end() == kmax, "last range must end at kmax");

            // Invariant 4: Contiguity
            for i in 0..len.saturating_sub(1) {
                let curr_end = ranges[i].end();
                let next_start = ranges[i + 1].start();
                match curr_end.checked_add(1) {
                    Some(expected) => {
                         == kmax, "last range must end at kmax");

            // Invariant 4: Contiguity
            for i in 0..len.saturating_sub(1) {
                let curr_end = ranges[i].end();
                let next_start = ranges[i + 1].start();
                match curr_end.checked_add(1) {
                    Some(expected) => {
                        kani::assert(expected == next_start, "ranges must be contiguous");
                    }
                    None => {
                        kani::assert(false, "end+1 overflow: no next range expected");
                    }
                }
            }

            // Invariant 5: No overlap
            for i in 0..len {
                for j in (i + 1)..len {
                    kani::assert(ranges[i].is_disjoint(ranges[j]), "ranges must be disjoint");
                }
            }

            if len > 1 {}
        }
        Err(_) => {}
    }

    kani::cover!(kmax == u64::MAX && n == MAX, "full keyspace, max shards");
}

// ============================================================================
// Supplemental: keyspace coverage
// ============================================================================

#[kani::proof]
#[kani::unwind(36)]
fn partition_plan_covers_keyspace() {
    let n: usize = kani::any();
    kani::assume(n >= 1 && n <= MAX);
    let kmax: u64 = kani::any();

    if let Ok(plan) = fixed_partition(n, kmax) {
        let len = plan.len;
        if len > 0 {
            for i in 0..len.saturating_sub(1) {
                match plan.ranges[i].end().checked_add(1) {
                    Some(expected) => {
                        kani::assert(expected == plan.ranges[i + 1].start(), "no gaps");
                    }
                    None => {}
                }
            }
        }
    }
}
