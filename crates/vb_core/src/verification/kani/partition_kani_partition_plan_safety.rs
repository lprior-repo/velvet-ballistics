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

use crate::shard::partition::{
    KeyRange, PartitionConfig, PartitionError, PartitionPlan, ShardCount,
};

const MAX: usize = 32;

// Construction validation helpers

fn build_shard_count(n: usize) -> Option<ShardCount> {
    match ShardCount::try_new(n) {
        Ok(sc) => Some(sc),
        Err(_) => {
            kani::assert(false, "ShardCount::try_new failed for n in [1, 32]");
            None
        }
    }
}

fn build_keyspace(kmax: u64) -> Option<KeyRange> {
    match KeyRange::new(0, kmax) {
        Ok(kr) => Some(kr),
        Err(_) => {
            kani::assert(false, "KeyRange::new(0, kmax) failed for any kmax");
            None
        }
    }
}

fn span_from_kmax(kmax: u64) -> u128 {
    match u128::from(kmax).checked_add(1) {
        Some(s) => s,
        None => {
            kani::assert(false, "span computation overflowed u128");
            0
        }
    }
}

fn n_to_u128(n: usize) -> u128 {
    match u128::try_from(n) {
        Ok(value) => value,
        Err(_) => {
            kani::assert(false, "n to u128 conversion failed");
            0
        }
    }
}

fn build_plan_from_parts(sc: ShardCount, kr: KeyRange) -> Result<PartitionPlan, PartitionError> {
    let config = PartitionConfig::new(sc, kr);
    PartitionPlan::from_config(&config)
}

// Result classification helpers

fn classify_success(span: u128, n128: u128) -> bool {
    if span >= n128 {
        return true;
    }
    kani::assert(span >= n128, "successful construction requires span >= n");
    false
}

fn classify_failure(span: u128, n128: u128) -> bool {
    if span < n128 {
        return true;
    }
    kani::assert(span < n128, "sufficient keys must always construct");
    false
}

fn classify_error_exact(kmax: u64, error: PartitionError) {
    match kmax.checked_add(1) {
        Some(start) => {
            kani::assert(
                error == PartitionError::InvalidKeyRange { start, end: kmax },
                "exact InvalidKeyRange error required for undersized keyspace",
            );
        }
        None => {
            kani::assert(false, "unexpected kmax+1 overflow");
        }
    }
}

fn handle_success(span: u128, n128: u128, plan: PartitionPlan) -> Option<PartitionPlan> {
    let ok = classify_success(span, n128);
    if ok { Some(plan) } else { None }
}

fn handle_failure(
    span: u128,
    n128: u128,
    kmax: u64,
    error: PartitionError,
) -> Option<PartitionPlan> {
    let ok = classify_failure(span, n128);
    if ok {
        classify_error_exact(kmax, error);
        kani::cover!(span < n128, "undersized keyspace rejection");
    }
    None
}

fn handle_plan_result(
    span: u128,
    n128: u128,
    kmax: u64,
    result: Result<PartitionPlan, PartitionError>,
) -> Option<PartitionPlan> {
    match result {
        Ok(plan) => handle_success(span, n128, plan),
        Err(error) => handle_failure(span, n128, kmax, error),
    }
}

/// Shared helper: validate partition construction and assert exact behavior.
/// Returns Some(plan) on success, None on expected failure.
/// All legitimate errors classified; no other error accepted.
fn checked_partition(n: usize, kmax: u64) -> Option<PartitionPlan> {
    let sc = match build_shard_count(n) {
        Some(sc) => sc,
        None => return None,
    };
    let kr = match build_keyspace(kmax) {
        Some(kr) => kr,
        None => return None,
    };
    let span = span_from_kmax(kmax);
    let n128 = n_to_u128(n);
    let result = build_plan_from_parts(sc, kr);
    handle_plan_result(span, n128, kmax, result)
}

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
