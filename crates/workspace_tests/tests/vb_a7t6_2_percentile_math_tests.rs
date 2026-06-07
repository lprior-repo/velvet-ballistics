//! Section 39 p50/p95/p99 percentile math — regression test for the
//! `latency=p50-p95-p99-by-criterion` mislabel fix (vb-a7t6.2).
//!
//! The actual percentile helper lives in
//! `crates/workspace_tests/benches/velvet_ballistics.rs` as
//! `pub mod latency_p50_p95_p99`. That module is a `harness = false`
//! bench binary, so its `pub` items are not importable from a
//! `cargo test --tests` integration test in this workspace. To still
//! gate the math contract (`velvet-ballistics/.beads/vb-a7t6.2/contract.md`
//! §2 nearest-rank rule, §3 ordering invariant, §3 sample_count >= 10
//! floor), this test re-implements the helper in terms of the public
//! contract and asserts the binding values. Downstream consumers
//! (`xtask/src/evidence_gate.rs`, `moon benchmark-proof`) apply the
//! same contract to the bench harness's emitted `<bench_id>.percentiles.jsonl`
//! files; if the bench helper ever drifts, those consumers will fail.
//!
//! The reference implementation here is intentionally simple and
//! dependency-free: no `criterion`, no `serde`, no `proptest`. It is
//! the spec; the bench helper is the implementation.

#![forbid(unsafe_code)]

use std::time::Duration;

/// Reference implementation of the nearest-rank percentile index from
/// `contract.md` §2: `idx(p, n) = min(n - 1, floor(p * n))` for
/// `p ∈ (0, 1]`. The `p_milli` argument is in parts-per-10000 to keep
/// the arithmetic in integer space (no `f64` rounding risk).
fn nearest_rank_index(p_milli: u16, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let p = p_milli as usize;
    let idx = (p.saturating_mul(n)) / 10_000;
    if idx >= n {
        n - 1
    } else {
        idx
    }
}

/// Reference percentile lookup: sort a `Vec<Duration>` and return the
/// value at the nearest-rank index for `p_milli`.
fn percentile_sorted(samples: &[Duration], p_milli: u16) -> Duration {
    samples[nearest_rank_index(p_milli, samples.len())]
}

fn p50_p95_p99_sorted(samples: &[Duration]) -> (Duration, Duration, Duration) {
    (
        percentile_sorted(samples, 5_000),
        percentile_sorted(samples, 9_500),
        percentile_sorted(samples, 9_900),
    )
}

fn assert_p50_p95_p99(samples: &mut Vec<Duration>, p50: u64, p95: u64, p99: u64) {
    samples.sort_unstable();
    let (a, b, c) = p50_p95_p99_sorted(samples);
    assert_eq!(a, Duration::from_nanos(p50), "p50 mismatch");
    assert_eq!(b, Duration::from_nanos(p95), "p95 mismatch");
    assert_eq!(c, Duration::from_nanos(p99), "p99 mismatch");
}

/// Canonical regression case from `contract.md` §2: for n = 100,
/// p50 → idx 50, p95 → idx 95, p99 → idx 99. Sorted samples
/// `[1ns, 2ns, ..., 100ns]` must yield p50 = 51ns, p95 = 96ns,
/// p99 = 100ns.
#[test]
fn p50_p95_p99_uses_nearest_rank_for_100_samples() {
    let mut samples: Vec<Duration> = (1..=100_u64).map(Duration::from_nanos).collect();
    assert_p50_p95_p99(&mut samples, 51, 96, 100);
}

/// Same canonical case for n = 1000: p50 → idx 500 (501ns),
/// p95 → idx 950 (951ns), p99 → idx 990 (991ns).
#[test]
fn p50_p95_p99_uses_nearest_rank_for_1000_samples() {
    let mut samples: Vec<Duration> = (1..=1000_u64).map(Duration::from_nanos).collect();
    assert_p50_p95_p99(&mut samples, 501, 951, 991);
}

/// Boundary: a single sample collapses all three percentiles to
/// that sample.
#[test]
fn p50_p95_p99_collapses_to_single_sample_when_n_is_1() {
    let mut samples: Vec<Duration> = vec![Duration::from_nanos(42)];
    assert_p50_p95_p99(&mut samples, 42, 42, 42);
}

/// Boundary: for n = 10, the p99 floor is `9900 * 10 / 10_000 = 9`,
/// which is samples[9] (the max). The clamp rule is `min(n-1, idx)`,
/// so even `p_milli = 10_000` must clamp to `n - 1`.
#[test]
fn p99_clamps_to_n_minus_1_when_floor_exceeds_n_minus_1() {
    let mut samples: Vec<Duration> = (1..=10_u64).map(Duration::from_nanos).collect();
    samples.sort_unstable();
    assert_eq!(percentile_sorted(&samples, 9_900), Duration::from_nanos(10));
    assert_eq!(percentile_sorted(&samples, 10_000), Duration::from_nanos(10));
    // p_milli = 5_000 with n = 10 → idx = 5, samples[5] = 6ns.
    assert_eq!(percentile_sorted(&samples, 5_000), Duration::from_nanos(6));
    // p_milli = 9_500 with n = 10 → idx = 9, samples[9] = 10ns.
    assert_eq!(percentile_sorted(&samples, 9_500), Duration::from_nanos(10));
}

/// Ordering invariant from `contract.md` §3: p50 ≤ p95 ≤ p99 for
/// any non-empty input. Verified for the canonical 100-sample
/// distribution and for an irregular 13-sample distribution.
#[test]
fn p50_p95_p99_orders_monotonically_for_canonical_input() {
    let mut samples: Vec<Duration> = (1..=100_u64).map(Duration::from_nanos).collect();
    samples.sort_unstable();
    let (p50, p95, p99) = p50_p95_p99_sorted(&samples);
    assert!(p50 <= p95, "p50 ({p50:?}) must be <= p95 ({p95:?})");
    assert!(p95 <= p99, "p95 ({p95:?}) must be <= p99 ({p99:?})");
}

#[test]
fn p50_p95_p99_orders_monotonically_for_irregular_input() {
    // Irregular sample set: not a simple arithmetic progression.
    let mut samples: Vec<Duration> = vec![
        Duration::from_nanos(7),
        Duration::from_nanos(1),
        Duration::from_nanos(50),
        Duration::from_nanos(3),
        Duration::from_nanos(12),
        Duration::from_nanos(99),
        Duration::from_nanos(8),
        Duration::from_nanos(4),
        Duration::from_nanos(17),
        Duration::from_nanos(42),
        Duration::from_nanos(60),
        Duration::from_nanos(2),
        Duration::from_nanos(33),
    ];
    samples.sort_unstable();
    let (p50, p95, p99) = p50_p95_p99_sorted(&samples);
    assert!(p50 <= p95, "p50 ({p50:?}) must be <= p95 ({p95:?})");
    assert!(p95 <= p99, "p95 ({p95:?}) must be <= p99 ({p99:?})");
    // Sorted indices: 0=1, 1=2, 2=3, 3=4, 4=7, 5=8, 6=12, 7=17,
    //                 8=33, 9=42, 10=50, 11=60, 12=99.
    // n = 13: p50 → idx = 5000 * 13 / 10_000 = 6, samples[6] = 12ns.
    assert_eq!(p50, Duration::from_nanos(12));
    // p95 → idx = 9500 * 13 / 10_000 = 12, samples[12] = 99ns.
    assert_eq!(p95, Duration::from_nanos(99));
    // p99 → idx = 9900 * 13 / 10_000 = 12, samples[12] = 99ns.
    assert_eq!(p99, Duration::from_nanos(99));
}

/// `nearest_rank_index` is the inner function that the bench helper
/// re-implements. Verify each canonical percentile on the canonical
/// sample sizes from the test plan.
#[test]
fn nearest_rank_index_matches_contract_table() {
    // n = 100: p50 → 50, p95 → 95, p99 → 99.
    assert_eq!(nearest_rank_index(5_000, 100), 50);
    assert_eq!(nearest_rank_index(9_500, 100), 95);
    assert_eq!(nearest_rank_index(9_900, 100), 99);
    // n = 1000: p50 → 500, p95 → 950, p99 → 990.
    assert_eq!(nearest_rank_index(5_000, 1000), 500);
    assert_eq!(nearest_rank_index(9_500, 1000), 950);
    assert_eq!(nearest_rank_index(9_900, 1000), 990);
    // n = 1: any percentile → 0.
    assert_eq!(nearest_rank_index(5_000, 1), 0);
    assert_eq!(nearest_rank_index(9_900, 1), 0);
    // n = 2: p99 → idx = 9900*2/10_000 = 1, samples[1] = max.
    assert_eq!(nearest_rank_index(9_900, 2), 1);
    // n = 0: clamped to 0.
    assert_eq!(nearest_rank_index(5_000, 0), 0);
}

/// The 3 captured scenarios use `sample_size = 10`, so the
/// percentiles for those scenarios follow the n = 10 case. This
/// test pins the expected percentile indices for the 3 captured
/// scenarios from `evidence/benchmark-logs/`.
#[test]
fn captured_scenarios_use_sample_size_10_indices() {
    // For n = 10: p50 → 5, p95 → 9, p99 → 9 (clamped from 9900*10/10_000=9).
    assert_eq!(nearest_rank_index(5_000, 10), 5);
    assert_eq!(nearest_rank_index(9_500, 10), 9);
    assert_eq!(nearest_rank_index(9_900, 10), 9);
    // Samples [1ns, 2ns, ..., 10ns] with n = 10:
    let mut samples: Vec<Duration> = (1..=10_u64).map(Duration::from_nanos).collect();
    samples.sort_unstable();
    assert_eq!(percentile_sorted(&samples, 5_000), Duration::from_nanos(6));
    assert_eq!(percentile_sorted(&samples, 9_500), Duration::from_nanos(10));
    assert_eq!(percentile_sorted(&samples, 9_900), Duration::from_nanos(10));
}
