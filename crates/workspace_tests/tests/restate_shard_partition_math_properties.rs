//! Proptest properties for shard partition math.
//!
//! **Obligations:**
//! - PO-001..PO-013, PO-018: 14 proptest properties
//!
//! **Command:** `cargo test -p velvet-ballistics-workspace-tests --test restate_shard_partition_math_properties -- --nocapture`

#![forbid(unsafe_code)]

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use vb_core::ids::RunId;
use vb_core::shard::partition::{
    KeyRange, MAX_SHARD_COUNT, PartitionConfig, PartitionError, PartitionPlan, ShardCount,
};

// ============================================================================
// Helper: unwrap Result without expect/panic
// ============================================================================

/// Assert Ok and return value, or fail the proptest.
fn assert_ok<T, E: std::fmt::Debug>(
    result: Result<T, E>,
    context: &str,
) -> Result<T, TestCaseError> {
    match result {
        Ok(v) => Ok(v),
        Err(e) => Err(TestCaseError::fail(format!("{context}: {e:?}"))),
    }
}

// ============================================================================
// Proptest Strategies
// ============================================================================

fn any_u64() -> impl Strategy<Value = u64> {
    proptest::num::u64::ANY
}

fn key_range_strategy() -> impl Strategy<Value = KeyRange> {
    (any_u64(), any_u64()).prop_map(|(a, b)| {
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        match KeyRange::new(start, end) {
            Ok(kr) => kr,
            Err(_) => KeyRange::full_keyspace(), // unreachable: start <= end by construction
        }
    })
}

fn shard_count_strategy() -> impl Strategy<Value = ShardCount> {
    (1usize..=MAX_SHARD_COUNT).prop_map(|n| match ShardCount::try_new(n) {
        Ok(sc) => sc,
        Err(_) => ShardCount::default(), // unreachable: n in [1, MAX]
    })
}

fn partition_config_strategy() -> impl Strategy<Value = PartitionConfig> {
    (shard_count_strategy(), key_range_strategy()).prop_map(|(sc, kr)| PartitionConfig::new(sc, kr))
}

fn partition_plan_strategy() -> impl Strategy<Value = PartitionPlan> {
    partition_config_strategy().prop_filter_map("valid plan", |config| {
        PartitionPlan::from_config(&config).ok()
    })
}

// ============================================================================
// PO-001: Range union covers configured keyspace
// ============================================================================

proptest! {
    #[test]
    fn proptest_range_union_covers_keyspace(config in partition_config_strategy()) {
        let plan = assert_ok(PartitionPlan::from_config(&config), "from_config")?;
        let ranges = plan.ranges();
        let kmin = config.keyspace.start();
        let kmax = config.keyspace.end();

        assert_eq!(ranges.len(), config.shard_count.get());
        assert_eq!(ranges[0].start(), kmin);
        assert_eq!(ranges[ranges.len() - 1].end(), kmax);

        for i in 0..ranges.len().saturating_sub(1) {
            let next = ranges[i].end().checked_add(1);
            assert!(next.is_some(), "range end overflow");
            assert_eq!(next, Some(ranges[i + 1].start()), "gap at range {i}");
        }
    }
}

// ============================================================================
// PO-002: All range pairs disjoint
// ============================================================================

proptest! {
    #[test]
    fn proptest_ranges_intersections_empty(config in partition_config_strategy()) {
        let plan = assert_ok(PartitionPlan::from_config(&config), "from_config")?;
        let ranges = plan.ranges();
        for i in 0..ranges.len() {
            for j in (i + 1)..ranges.len() {
                assert!(ranges[i].is_disjoint(ranges[j]), "overlap at ({i}, {j})");
            }
        }
    }
}

// ============================================================================
// PO-003: Adjacent ranges contiguous
// ============================================================================

proptest! {
    #[test]
    fn proptest_ranges_contiguous(config in partition_config_strategy()) {
        let plan = assert_ok(PartitionPlan::from_config(&config), "from_config")?;
        let ranges = plan.ranges();
        assert_eq!(ranges[0].start(), config.keyspace.start());
        for i in 0..ranges.len().saturating_sub(1) {
            let next = ranges[i].end().checked_add(1);
            assert!(next.is_some(), "overflow");
            assert_eq!(next.unwrap_or(0), ranges[i + 1].start());
        }
        assert_eq!(ranges[ranges.len() - 1].end(), config.keyspace.end());
    }
}

// ============================================================================
// PO-004 & PO-005: Shard index bounds (sampling-based)
// ============================================================================

proptest! {
    #[test]
    fn proptest_shard_index_bounded(
        run in any_u64(),
        shard_count in any_u64(),
    ) {
        let run_id = RunId::new(run);
        let result = run_id.shard_index(shard_count);
        if shard_count > 0 {
            assert!(result < shard_count);
        } else {
            assert_eq!(result, 0);
        }
    }

    #[test]
    fn proptest_shard_index_distribution(
        (run, sc) in (any_u64(), 1usize..=MAX_SHARD_COUNT),
    ) {
        let run_id = RunId::new(run);
        let result = run_id.shard_index(sc as u64);
        assert!(result < sc as u64);
    }
}

// ============================================================================
// PO-006: Zero shard count rejected
// ============================================================================

proptest! {
    #[test]
    fn proptest_rejects_zero_shard_count(raw in proptest::num::usize::ANY) {
        let result = ShardCount::try_new(raw);
        if raw == 0 {
            assert!(matches!(result, Err(PartitionError::ZeroShardCount)));
        }
    }
}

// ============================================================================
// PO-007: Excessive shard count rejected
// ============================================================================

proptest! {
    #[test]
    fn proptest_rejects_excessive_shard_count(raw in proptest::num::usize::ANY) {
        let result = ShardCount::try_new(raw);
        match raw {
            0 => assert!(matches!(result, Err(PartitionError::ZeroShardCount))),
            n if n > MAX_SHARD_COUNT => {
                assert!(matches!(result, Err(PartitionError::ShardCountExceedsMax { .. })));
            }
            _ => assert!(result.is_ok()),
        }
    }
}

// ============================================================================
// PO-008: Inverted key range rejected
// ============================================================================

proptest! {
    #[test]
    fn proptest_rejects_inverted_range(a in any_u64(), b in any_u64()) {
        let result = KeyRange::new(a, b);
        if a > b {
            assert!(matches!(result, Err(PartitionError::InvalidKeyRange { start, end })
                if start == a && end == b));
        } else {
            assert!(result.is_ok());
        }
    }
}

// ============================================================================
// PO-009: Key-to-shard lookup correctness
// ============================================================================

proptest! {
    #[test]
    fn proptest_key_to_shard_lookup(plan in partition_plan_strategy()) {
        let ranges = plan.ranges();
        for (idx, range) in ranges.iter().enumerate() {
            let mid = range.start().saturating_add(
                range.end().saturating_sub(range.start()) / 2
            );
            for key in [range.start(), mid, range.end()].iter() {
                if range.contains(*key) {
                    assert_eq!(plan.shard_for_key(*key), Some(idx));
                }
            }
        }
        let first = ranges[0].start();
        let last = ranges[ranges.len() - 1].end();
        if first > 0 {
            assert_eq!(plan.shard_for_key(first.saturating_sub(1)), None);
        }
        if last < u64::MAX {
            assert_eq!(plan.shard_for_key(last.saturating_add(1)), None);
        }
    }
}

// ============================================================================
// PO-011: RunId::shard_index determinism
// ============================================================================

proptest! {
    #[test]
    fn proptest_shard_index_deterministic(
        run in any_u64(),
        shard_count in any_u64(),
    ) {
        let run_id = RunId::new(run);
        let r1 = run_id.shard_index(shard_count);
        let r2 = run_id.shard_index(shard_count);
        let r3 = run_id.shard_index(shard_count);
        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
    }
}

// ============================================================================
// PO-012: ShardCount bounded invariant
// ============================================================================

proptest! {
    #[test]
    fn proptest_shard_count_bounded(raw in 1usize..=MAX_SHARD_COUNT) {
        let sc = match ShardCount::try_new(raw) {
            Ok(s) => s,
            Err(_) => return Err(TestCaseError::fail("valid raw should produce ShardCount")),
        };
        assert!(sc.get() >= 1 && sc.get() <= MAX_SHARD_COUNT);
        let sc2 = sc;
        assert_eq!(sc2.get(), sc.get());
        let sc3 = sc;
        assert_eq!(sc3.get(), sc.get());
        assert_eq!(sc.as_u64(), sc.get() as u64);
    }
}

#[test]
fn proptest_shard_count_default() {
    let sc = ShardCount::default();
    assert_eq!(sc.get(), 1);
}

// ============================================================================
// PO-013: u64::MAX key routing edge case
// ============================================================================

proptest! {
    #[test]
    fn proptest_u64_max_key_routing(sc in 1usize..=MAX_SHARD_COUNT) {
        let run_id = RunId::new(u64::MAX);
        let result = run_id.shard_index(sc as u64);
        assert!(result < sc as u64);
        assert_eq!(result, u64::MAX % (sc as u64));
    }
}

// ============================================================================
// PO-018: KeyRange::is_adjacent_to correctness
// ============================================================================

proptest! {
    #[test]
    fn proptest_key_range_adjacent(
        a in key_range_strategy(),
        b in key_range_strategy(),
    ) {
        assert_eq!(a.is_adjacent_to(b), b.is_adjacent_to(a));

        let expected = a.end().checked_add(1) == Some(b.start())
            || b.end().checked_add(1) == Some(a.start());
        assert_eq!(a.is_adjacent_to(b), expected);

        if a.intersection(b).is_some() {
            assert!(!a.is_adjacent_to(b));
        }
    }
}

// ============================================================================
// Deterministic unit tests (non-proptest)
// ============================================================================

/// Helper for unit tests: unwrap Result without expect/panic/unwrap.
/// Uses process::abort() which is a valid never-return for test failures.
fn ok<T, E: std::fmt::Debug>(r: Result<T, E>, label: &str) -> T {
    match r {
        Ok(v) => v,
        Err(e) => {
            eprintln!("TEST FAILURE: {label}: {e:?}");
            std::process::abort();
        }
    }
}

#[test]
fn test_single_key_range() {
    let kr = ok(KeyRange::new(42, 42), "single key");
    assert!(kr.contains(42));
    assert!(!kr.contains(41));
    assert!(!kr.contains(43));
    assert!(kr.is_disjoint(ok(KeyRange::new(0, 41), "valid")));
    assert!(kr.is_adjacent_to(ok(KeyRange::new(43, 100), "valid")));
}

#[test]
fn test_max_key_final_shard() {
    let config = PartitionConfig::new(
        ok(ShardCount::try_new(4), "4"),
        ok(KeyRange::new(0, u64::MAX), "full"),
    );
    let plan = ok(PartitionPlan::from_config(&config), "valid config");
    let last = plan.ranges().len() - 1;
    assert!(plan.ranges()[last].contains(u64::MAX));
}

#[test]
fn test_four_shards_contiguous() {
    let config = PartitionConfig::new(
        ok(ShardCount::try_new(4), "4"),
        ok(KeyRange::new(0, 99), "valid"),
    );
    let plan = ok(PartitionPlan::from_config(&config), "valid config");
    assert_eq!(plan.ranges().len(), 4);
    for i in 0..3 {
        let next = plan.ranges()[i].end().checked_add(1);
        assert!(next.is_some());
        assert_eq!(next.unwrap_or(0), plan.ranges()[i + 1].start());
    }
}

#[test]
fn test_boundary_rejections() {
    assert!(matches!(
        ShardCount::try_new(0),
        Err(PartitionError::ZeroShardCount)
    ));
    assert!(matches!(
        ShardCount::try_new(MAX_SHARD_COUNT + 1),
        Err(PartitionError::ShardCountExceedsMax { .. })
    ));
    assert!(matches!(
        KeyRange::new(100, 50),
        Err(PartitionError::InvalidKeyRange { .. })
    ));
}

#[test]
fn test_partition_plan_single_shard() {
    let config = PartitionConfig::new(
        ok(ShardCount::try_new(1), "1"),
        ok(KeyRange::new(100, 200), "valid"),
    );
    let plan = ok(PartitionPlan::from_config(&config), "valid config");
    assert_eq!(plan.ranges().len(), 1);
    assert_eq!(plan.ranges()[0].start(), 100);
    assert_eq!(plan.ranges()[0].end(), 200);
    assert_eq!(plan.shard_for_key(150), Some(0));
    assert_eq!(plan.shard_for_key(50), None);
}

#[test]
fn test_partition_plan_invariants_suite() {
    let cases = [
        (1, 0, u64::MAX),
        (2, 0, u64::MAX),
        (3, 0, u64::MAX),
        (4, 0, u64::MAX),
        (7, 0, u64::MAX),
        (16, 0, u64::MAX),
        (32, 0, u64::MAX),
        (4, 100, 999),
        (5, 0, 99),
    ];
    for (n, kmin, kmax) in cases.iter() {
        let config = PartitionConfig::new(
            ok(ShardCount::try_new(*n), "valid n"),
            ok(KeyRange::new(*kmin, *kmax), "valid range"),
        );
        let plan = ok(PartitionPlan::from_config(&config), "valid config");
        let ranges = plan.ranges();
        assert_eq!(ranges.len(), *n);
        assert_eq!(ranges[0].start(), *kmin);
        assert_eq!(ranges[ranges.len() - 1].end(), *kmax);
        for i in 0..ranges.len().saturating_sub(1) {
            let next = ranges[i].end().checked_add(1);
            assert!(next.is_some(), "overflow");
            assert_eq!(next.unwrap_or(0), ranges[i + 1].start());
        }
        for i in 0..ranges.len() {
            for j in (i + 1)..ranges.len() {
                assert!(ranges[i].is_disjoint(ranges[j]));
            }
        }
    }
}
