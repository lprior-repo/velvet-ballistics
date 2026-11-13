#![forbid(unsafe_code)]

use crate::shard::partition::{
    KeyRange, PartitionConfig, PartitionError, PartitionPlan, ShardCount,
};

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
pub(super) fn checked_partition(n: usize, kmax: u64) -> Option<PartitionPlan> {
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
