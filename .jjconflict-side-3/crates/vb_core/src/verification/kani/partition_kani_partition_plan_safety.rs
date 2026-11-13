//! Kani harnesses for PartitionPlan construction safety and output correctness.
//!
//! **Obligations:**
//! - PO-010a: `PartitionPlan::from_config()` no-panic for N ∈ [1, 32]
//! - PO-010b: Output satisfies 5 construction invariants for N ∈ [1, 32]
//!
//! **Bounds:** shard_count ∈ [1, 32], keyspace start == 0, unwind = 36
//!
//! Shared helper `checked_partition` validates construction, asserts exact
//! success/failure classification, and returns Option<PartitionPlan>.
//! Success iff span >= n; failure must be exact InvalidKeyRange.

#![forbid(unsafe_code)]

use crate::shard::partition::KeyRange;

#[path = "partition_kani_construction_helpers.rs"]
mod construction;

use construction::checked_partition;

const MAX: usize = 32;

// PO-010b invariant checkers

fn invariant_length(n: usize, ranges: &[KeyRange]) {
    let len = ranges.len();
    kani::assert(len == n, "plan length must equal n");
    kani::assert(len > 0, "plan must have at least one range");
}

fn invariant_endpoints(kmax: u64, ranges: &[KeyRange]) {
    match ranges.first() {
        Some(first) => {
            kani::assert(first.start() == 0, "first range must start at 0");
        }
        None => kani::assert(false, "first range missing"),
    }
    match ranges.last() {
        Some(last) => {
            kani::assert(last.end() == kmax, "last range must end at kmax");
        }
        None => kani::assert(false, "last range missing"),
    }
}

fn check_pair_contiguity(pair: &[KeyRange]) {
    match (pair.first(), pair.get(1)) {
        (Some(curr), Some(next)) => match curr.end().checked_add(1) {
            Some(expected) => {
                kani::assert(expected == next.start(), "ranges must be contiguous");
            }
            None => {
                kani::assert(false, "end+1 overflow");
            }
        },
        _ => kani::assert(false, "pair missing range"),
    }
}

fn invariant_contiguity(ranges: &[KeyRange]) {
    for pair in ranges.windows(2) {
        check_pair_contiguity(pair);
    }
}

fn check_range_disjoint(i: usize, left: &KeyRange, ranges: &[KeyRange]) {
    for (j, right) in ranges.iter().enumerate() {
        if i < j {
            kani::assert(left.is_disjoint(*right), "ranges must be disjoint");
        }
    }
}

fn invariant_disjoint(ranges: &[KeyRange]) {
    for (i, left) in ranges.iter().enumerate() {
        check_range_disjoint(i, left, ranges);
    }
}

fn cover_post_conditions(n: usize, kmax: u64, len: usize) {
    if len > 1 {
        kani::cover!(true, "multi-shard plan");
    }
    kani::cover!(kmax == u64::MAX && n == MAX, "full keyspace, max shards");
}

// PO-010b coverage checkers

fn coverage_check_length(n: usize, ranges: &[KeyRange]) {
    let len = ranges.len();
    kani::assert(len == n, "plan length must equal n");
}

fn coverage_check_endpoints(kmax: u64, ranges: &[KeyRange]) {
    match ranges.first() {
        Some(first) => {
            kani::assert(first.start() == 0, "first range must start at 0");
        }
        None => kani::assert(false, "first range missing"),
    }
    match ranges.last() {
        Some(last) => {
            kani::assert(last.end() == kmax, "last range must end at kmax");
        }
        None => kani::assert(false, "last range missing"),
    }
}

fn check_gap(left: &KeyRange, right: &KeyRange) {
    match left.end().checked_add(1) {
        Some(expected) => {
            kani::assert(expected == right.start(), "no gaps between ranges");
        }
        None => {
            kani::assert(false, "end+1 overflow");
        }
    }
}

fn coverage_check_gaps(ranges: &[KeyRange]) {
    for (left, right) in ranges.iter().zip(ranges.iter().skip(1)) {
        check_gap(left, right);
    }
}

/// PO-010a: PartitionPlan::from_config() is panic-free for N ∈ [1, 32].
#[kani::proof]
#[kani::unwind(36)]
fn partition_plan_from_config_no_panic() {
    let n: usize = kani::any();
    kani::assume(n >= 1 && n <= MAX);
    let kmax: u64 = kani::any();

    let outcome = checked_partition(n, kmax);

    kani::cover!(outcome.is_some(), "successful construction");
    kani::cover!(n == 1, "single shard");
    kani::cover!(n == MAX, "max shards");
    kani::cover!(kmax == u64::MAX, "full keyspace");
    kani::cover!(kmax == 0, "singleton keyspace");
}

/// PO-010b: Successful construction satisfies 5 construction invariants.
#[kani::proof]
#[kani::unwind(36)]
fn partition_plan_post_conditions() {
    let n: usize = kani::any();
    kani::assume(n >= 1 && n <= MAX);
    let kmax: u64 = kani::any();

    let outcome = checked_partition(n, kmax);

    let len = if let Some(plan) = &outcome {
        let ranges = plan.ranges();
        invariant_length(n, ranges);
        invariant_endpoints(kmax, ranges);
        invariant_contiguity(ranges);
        invariant_disjoint(ranges);
        ranges.len()
    } else {
        0
    };

    cover_post_conditions(n, kmax, len);
}

/// Supplemental: Successful plans cover the full keyspace with no gaps.
#[kani::proof]
#[kani::unwind(36)]
fn partition_plan_covers_keyspace() {
    let n: usize = kani::any();
    kani::assume(n >= 1 && n <= MAX);
    let kmax: u64 = kani::any();

    let outcome = checked_partition(n, kmax);

    if let Some(plan) = outcome {
        let ranges = plan.ranges();
        coverage_check_length(n, ranges);
        coverage_check_endpoints(kmax, ranges);
        coverage_check_gaps(ranges);
        kani::cover!(kmax == u64::MAX, "full keyspace covered");
        kani::cover!(n == 1, "single shard covers all");
        kani::cover!(kmax == 0, "singleton keyspace covered");
    }
}
